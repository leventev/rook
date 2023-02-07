use core::mem::{transmute, MaybeUninit};

use alloc::{boxed::Box, rc::Weak, string::String};

use crate::{
    blk::{IORequest, Partition, BLOCK_LBA_SIZE},
    fs::{self, inode::Inode, path::Path, FileSystemError, FileSystemInner, FileSystemSkeleton},
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

const DIR_ENT_READ_ONLY: u8 = 1 << 0;
const DIR_ENT_HIDDEN: u8 = 1 << 1;
const DIR_ENT_SYSTEM: u8 = 1 << 2;
const DIR_ENT_VOLUME_ID: u8 = 1 << 3;
const DIR_ENT_DIRECTORY: u8 = 1 << 4;
const DIR_ENT_ARCHIVE: u8 = 1 << 5;
const DIR_ENT_LONG_NAME: u8 =
    DIR_ENT_READ_ONLY | DIR_ENT_HIDDEN | DIR_ENT_SYSTEM | DIR_ENT_VOLUME_ID;

const DIR_ENTRIES_PER_SECTOR: usize = BLOCK_LBA_SIZE / core::mem::size_of::<ShortDirectoryEntry>();
const LONG_DIR_ENTRY_LAST_ENTRY_MARKER: u8 = 0x40;
const MAX_FILENAME_LENGTH: usize = 256;
// TODO: utf-16
const CHARS_PER_LONG_ENTRY: usize = 26;

const FAT_ENTRIES_PER_SECTOR: usize = BLOCK_LBA_SIZE / core::mem::size_of::<u32>();

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

#[derive(Debug, PartialEq)]
enum DirectoryEntryType {
    File(usize),
    Directory,
}

#[derive(Debug)]
struct DirectoryEntry {
    ent_type: DirectoryEntryType,
    data_cluster_start: u32,
    directory_cluster: u32,
    directory_cluster_index: usize,
}

struct FATFileSystem {
    partition: Weak<Partition>,
    lba_count: usize,
    reserved_sector_count: usize,
    sectors_per_cluster: usize,
    fat_count: usize,
    data_sectors_start: usize,
    root_cluster: u32,
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
            lba_count: lba_count as usize,
            reserved_sector_count: reserved_sector_count as usize,
            data_sectors_start: reserved_sector_count + (fat_count * fat_size) + root_dir_sectors,
            sectors_per_cluster: bios_parameter_data.sectors_per_cluster as usize,
            fat_count,
            root_cluster: extended_bpd.root_dir_cluster,
        };

        Ok(fs)
    }

    #[inline]
    fn cluster_sector_index(&self, cluster: u32) -> usize {
        assert!(cluster >= 2);
        self.data_sectors_start + (cluster as usize - 2) * self.sectors_per_cluster
    }

    #[inline]
    fn fat_offset_sector(&self, cluster: u32) -> (usize, usize) {
        let sector = (cluster as usize) / FAT_ENTRIES_PER_SECTOR;
        let idx = (cluster as usize) % FAT_ENTRIES_PER_SECTOR;
        (sector, idx)
    }

    #[inline]
    fn fat_table_sector(&self, fat_sector_idx: u32) -> usize {
        self.reserved_sector_count + fat_sector_idx as usize
    }

    /// Read the specified cluster from the File Allocation Table
    fn get_fat_entry(&self, cluster: u32) -> u32 {
        let offsets = self.fat_offset_sector(cluster);

        let p = self.partition.upgrade().unwrap();
        let mut sector_data: [u8; BLOCK_LBA_SIZE] = unsafe {
            transmute(MaybeUninit::<[MaybeUninit<u8>; BLOCK_LBA_SIZE]>::uninit().assume_init())
        };

        let sector = self.fat_table_sector(offsets.0 as u32) as usize;
        p.read(IORequest::new(sector, 1, &mut sector_data[..]))
            .unwrap();

        // TODO: do this safely
        let ptr = unsafe { (sector_data.as_ptr() as *const u32).offset(offsets.1 as isize) };
        (unsafe { *ptr } & 0x0FFFFFFF)
    }

    fn parse_short_dir_ent_filename(filename: &[u8; 11]) -> String {
        let filebase = &filename[..8];
        let filename_len = filebase.iter().position(|c| *c == ' ' as u8).unwrap();
        let filebase_str = core::str::from_utf8(&filebase[..filename_len]).unwrap();

        let extension = &filename[8..];
        let extension_len = extension.iter().position(|c| *c == ' ' as u8).unwrap();
        let extension_str = core::str::from_utf8(&extension[..extension_len]).unwrap();

        // TODO: make this work without allocation
        let mut full = String::from(filebase_str);
        if extension_len > 0 {
            full.push('.');
            full.push_str(extension_str);
        }

        full
    }

    #[inline]
    fn valid_cluster(cluster: u32) -> bool {
        cluster < 0x0FFFFFF7
    }

    #[inline]
    fn fuse_cluster_parts(low: u16, high: u16) -> u32 {
        u32::from_le_bytes([low as u8, (low >> 8) as u8, high as u8, (high >> 8) as u8])
    }

    fn find_dir_ent(&self, dir_start_cluster: u32, filename: &str) -> Option<DirectoryEntry> {
        let p = self.partition.upgrade().unwrap();
        let mut sector_data: [u8; BLOCK_LBA_SIZE] = unsafe {
            transmute(MaybeUninit::<[MaybeUninit<u8>; BLOCK_LBA_SIZE]>::uninit().assume_init())
        };

        let mut long_file_name = String::with_capacity(MAX_FILENAME_LENGTH);
        let mut cluster = dir_start_cluster;

        while Self::valid_cluster(cluster as u32) {
            let sector = self.cluster_sector_index(cluster);
            p.read(IORequest::new(sector, 1, &mut sector_data[..]))
                .unwrap();

            // TODO: check the other sectors of the directory
            for i in 0..DIR_ENTRIES_PER_SECTOR {
                let offset = i * core::mem::size_of::<ShortDirectoryEntry>();

                // first byte of the entry
                let long_entry = match sector_data[offset] {
                    // end of directory entries
                    0 => return None,
                    // unused
                    0xE5 => continue,
                    // attribute
                    _ => sector_data[offset + 0xB] == DIR_ENT_LONG_NAME,
                };

                if long_entry {
                    let ent: &LongDirectoryEntry = unsafe {
                        (sector_data.as_ptr().offset(offset as isize) as *const LongDirectoryEntry)
                            .as_ref()
                            .unwrap()
                    };

                    // remove the long dir entry flag
                    let order = if ent.order & LONG_DIR_ENTRY_LAST_ENTRY_MARKER > 0 {
                        ent.order ^ LONG_DIR_ENTRY_LAST_ENTRY_MARKER
                    } else {
                        ent.order
                    };

                    // directory entries cant cross sector boundaries supposedly
                    assert!(i + order as usize <= DIR_ENTRIES_PER_SECTOR);

                    let mut temp_str = String::with_capacity(CHARS_PER_LONG_ENTRY);
                    for c in [&ent.name1[..], &ent.name2[..], &ent.name3[..]]
                        .concat()
                        .chunks_exact(2)
                        .into_iter()
                        .map(|ch| u16::from_le_bytes([ch[0], ch[1]]))
                    {
                        if c == 0xFFFF || c == 0x0 {
                            break;
                        }

                        // TODO: support utf16
                        temp_str.push(c as u8 as char);
                    }

                    long_file_name.insert_str(0, &temp_str);
                } else {
                    let ent: &ShortDirectoryEntry = unsafe {
                        (sector_data.as_ptr().offset(offset as isize) as *const ShortDirectoryEntry)
                            .as_ref()
                            .unwrap()
                    };

                    let ent_type = if ent.attr & DIR_ENT_DIRECTORY > 0 {
                        DirectoryEntryType::Directory
                    } else {
                        DirectoryEntryType::File(ent.file_size as usize)
                    };

                    if long_file_name.len() > 0 {
                        if long_file_name != filename {
                            long_file_name.clear();
                            continue;
                        }
                    } else {
                        // TODO: test this
                        let full = &Self::parse_short_dir_ent_filename(&ent.name);
                        if full != filename {
                            continue;
                        }
                    };

                    return Some(DirectoryEntry {
                        data_cluster_start: Self::fuse_cluster_parts(
                            ent.cluster_low,
                            ent.cluster_high,
                        ),
                        ent_type,
                        directory_cluster: cluster,
                        directory_cluster_index: i,
                    });
                }
            }

            cluster = self.get_fat_entry(cluster);
        }

        None
    }

    fn file_to_inode(directory_cluster: u32, directory_entry_index: usize) -> Inode {
        assert!(directory_entry_index < 16);
        assert!(directory_cluster < 0x0FFFFFFF);
        Inode::new((directory_cluster as u64) << 4 | directory_entry_index as u64)
    }

    const fn inode_to_file(inode: &Inode) -> (u32, usize) {
        // a FAT-32 inode is always less than u32::MAX
        assert!(inode.0 < u32::MAX as u64);
        let val = inode.0 as u32;
        (val >> 4, (val & 0xF) as usize)
    }
}

impl FileSystemInner for FATFileSystem {
    fn open(&self, path: &Path) -> Result<Inode, fs::FileSystemError> {
        let root_dir_start_cluster = self.root_cluster;
        let mut start_cluster = root_dir_start_cluster;

        let mut directory_cluster: u32 = 0;
        let mut directory_cluster_index: usize = 0;

        for part in &path[..path.len() - 1] {
            let dir_ent = self.find_dir_ent(start_cluster, part.as_str());
            match dir_ent {
                Some(ent) => {
                    match ent.ent_type {
                        DirectoryEntryType::File(_) => return Err(FileSystemError::FileNotFound),
                        DirectoryEntryType::Directory => (),
                    }

                    start_cluster = ent.data_cluster_start;
                    directory_cluster = ent.directory_cluster;
                    directory_cluster_index = ent.directory_cluster_index;
                }
                None => {
                    return Err(FileSystemError::FileNotFound);
                }
            }
        }

        let last_file = self.find_dir_ent(start_cluster, path.last().unwrap());
        if last_file.is_none() {
            return Err(FileSystemError::FileNotFound);
        }

        let inode = Self::file_to_inode(directory_cluster, directory_cluster_index);

        Ok(inode)
    }

    fn close(&self, _inode: Inode) -> Result<(), fs::FileSystemError> {
        todo!()
    }

    fn read(
        &self,
        inode: Inode,
        _offset: usize,
        _buff: &mut [u8],
        _size: usize,
    ) -> Result<usize, fs::FileSystemError> {
        todo!()
    }

    fn write(
        &self,
        _inode: Inode,
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
