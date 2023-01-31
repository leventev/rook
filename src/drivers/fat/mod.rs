use core::mem::{transmute, MaybeUninit};

use alloc::{boxed::Box, format, rc::Weak, string::String, vec::Vec};

use crate::{
    blk::{IORequest, Partition, BLOCK_LBA_SIZE},
    fs::{self, path::Path, FileSystemError, FileSystemInner, FileSystemSkeleton},
};

#[repr(C, packed)]
struct BIOSPBLegacy {
    jmp: [u8; 3],
    oem_id: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sector_count: u16,
    fat_count: u8,
    root_dir_entries: u16,
    total_sectors_small: u16,
    media_descriptor_type: u8,

    /// Only in FAT12/FAT16
    sectors_per_fat: u16,

    sectors_per_track: u16,
    head_count: u16,
    hidden_sector_count: u32,
    total_sectors_large: u32,
}

#[repr(C, packed)]
struct BIOSPB {}

#[repr(C, packed)]
// fat 32
struct ExtendedBIOSPB {
    sectors_per_fat: u32,
    flags: u16,
    fat_version_number: u16,
    root_dir_cluster: u32,
    fsinfo_struct_sector: u16,
    backup_boot_sector: u16,
    reserved1: [u8; 12],
    drive_num: u8,
    reserved2: u8,
    signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
}

const MAGIC_NUMBER: [u8; 2] = [0x55, 0xAA];

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
struct ShortDirectoryEntry {
    name: [u8; 11],
    attr: u8,
    reserved: u8,
    create_time_tenth: u8,
    create_time: u16,
    create_date: u16,
    last_acc_date: u16,
    cluster_high: u16,
    write_time: u16,
    write_date: u16,
    cluster_low: u16,
    file_size: u32,
}

#[repr(C, packed)]
struct LongDirectoryEntry {
    order: u8,
    name1: [u8; 10],
    attr: u8,
    ent_type: u8,
    checksum: u8,
    name2: [u8; 12],
    cluster_low: u16,
    name3: [u8; 4],
}

struct FATFileSystem {
    partition: Weak<Partition>,
    lba_count: usize,
    reserved_sector_count: usize,
    data_sectors_start: usize,
    sectors_per_cluster: usize,
    fat_count: usize,
    root_cluster: usize,
}

impl FATFileSystem {
    pub fn new(part: Weak<Partition>) -> Result<FATFileSystem, FileSystemError> {
        let p = part.upgrade().unwrap();

        let mut bios_parameter_block: [u8; BLOCK_LBA_SIZE] = unsafe {
            transmute(MaybeUninit::<[MaybeUninit<u8>; BLOCK_LBA_SIZE]>::uninit().assume_init())
        };

        p.read(IORequest::new(0, 1, &mut bios_parameter_block[..]))
            .unwrap();

        if bios_parameter_block[510..] != MAGIC_NUMBER {
            return Err(FileSystemError::FailedToInitializeFileSystem);
        }

        let bios_parameter_data: &BIOSPBLegacy = unsafe {
            (bios_parameter_block.as_ptr() as *const BIOSPBLegacy)
                .as_ref()
                .unwrap()
        };

        if bios_parameter_data.root_dir_entries != 0 {
            println!("FAT: non FAT-32 FAT filesystem detected");
            return Err(FileSystemError::FailedToInitializeFileSystem);
        }

        let extended_bpd: &ExtendedBIOSPB = unsafe {
            (bios_parameter_block
                .as_ptr()
                .offset(core::mem::size_of::<BIOSPBLegacy>() as isize)
                as *const ExtendedBIOSPB)
                .as_ref()
                .unwrap()
        };

        let lba_count = match bios_parameter_data.total_sectors_small {
            0 => bios_parameter_data.total_sectors_large as usize,
            n => n as usize,
        };

        let fat_size = match bios_parameter_data.sectors_per_fat {
            0 => extended_bpd.sectors_per_fat as usize,
            n => n as usize,
        };

        let reserved_sector_count = bios_parameter_data.reserved_sector_count as usize;
        let fat_count = bios_parameter_data.fat_count as usize;

        // this is always zero on FAT-32
        let root_dir_sectors = 0;

        let fs = FATFileSystem {
            partition: part,
            lba_count,
            reserved_sector_count,
            data_sectors_start: reserved_sector_count as usize
                + (fat_count * fat_size)
                + root_dir_sectors,
            sectors_per_cluster: bios_parameter_data.sectors_per_cluster as usize,
            fat_count,
            root_cluster: extended_bpd.root_dir_cluster as usize,
        };

        Ok(fs)
    }

    #[inline]
    fn cluster_sector_index(&self, cluster: usize) -> usize {
        assert!(cluster >= 2);
        self.data_sectors_start + (cluster - 2) * self.sectors_per_cluster
    }

    #[inline]
    fn cluster_fat_offset(&self, cluster: usize) -> usize {
        cluster * core::mem::size_of::<u32>()
    }

    #[inline]
    fn fat_offset_sector(&self, cluster: usize) -> (usize, usize) {
        let fat_offset = self.cluster_fat_offset(cluster);
        let sector = self.reserved_sector_count + (fat_offset / BLOCK_LBA_SIZE);
        let offset = fat_offset % BLOCK_LBA_SIZE;
        (sector, offset)
    }
}

impl FileSystemInner for FATFileSystem {
    fn open(&self, _path: &Path) -> Result<usize, fs::FileSystemError> {
        let p = self.partition.upgrade().unwrap();
        let lba = self.cluster_sector_index(self.root_cluster);

        let mut sector_data: [u8; BLOCK_LBA_SIZE] = unsafe {
            transmute(MaybeUninit::<[MaybeUninit<u8>; BLOCK_LBA_SIZE]>::uninit().assume_init())
        };

        p.read(IORequest::new(lba, 1, &mut sector_data[..]))
            .unwrap();

        const LONG_DIR_ENTRY_LAST_ENTRY: u8 = 0x40;

        let mut offset = 0;
        let long_entry_count = if sector_data[offset] & LONG_DIR_ENTRY_LAST_ENTRY > 0 {
            (sector_data[offset] - LONG_DIR_ENTRY_LAST_ENTRY) as usize
        } else {
            0
        };

        let mut long_file_name = String::new();

        'outer: for i in (0..long_entry_count).rev() {
            let start_off = 0 + i * core::mem::size_of::<LongDirectoryEntry>();
            let ent: &LongDirectoryEntry = unsafe {
                (sector_data.as_ptr().offset(start_off as isize) as *const LongDirectoryEntry)
                    .as_ref()
                    .unwrap()
            };

            for c in [&ent.name1[..], &ent.name2[..], &ent.name3[..]]
                .concat()
                .chunks_exact(2)
                .into_iter()
                .map(|ch| u16::from_ne_bytes([ch[0], ch[1]]))
            {
                if c == 0xFFFF || c == 0x0 {
                    break 'outer;
                }
                // TODO: support utf16
                long_file_name.push(c as u8 as char);
            }
        }
        offset += long_entry_count * core::mem::size_of::<LongDirectoryEntry>();

        let ent: &ShortDirectoryEntry = unsafe {
            (sector_data.as_ptr().offset(offset as isize) as *const ShortDirectoryEntry)
                .as_ref()
                .unwrap()
        };

        println!("{} {:?}", long_file_name, ent);

        todo!()
    }

    fn close(&self, _inode: usize) -> Result<(), fs::FileSystemError> {
        todo!()
    }

    fn read(
        &self,
        _inode: usize,
        _offset: usize,
        _buff: &mut [u8],
        _size: usize,
    ) -> Result<usize, fs::FileSystemError> {
        todo!()
    }

    fn write(
        &self,
        _inode: usize,
        _offset: usize,
        _buff: &[u8],
        _size: usize,
    ) -> Result<usize, fs::FileSystemError> {
        todo!()
    }
}

fn create_fs(part: Weak<Partition>) -> Result<Box<dyn FileSystemInner>, FileSystemError> {
    match FATFileSystem::new(part) {
        Ok(fs) => Ok(Box::new(fs)),
        Err(err) => Err(err),
    }
}

pub fn init() -> bool {
    fs::register_fs_skeleton(FileSystemSkeleton {
        new: create_fs,
        name: "FAT",
    })
    .is_ok()
}
