use core::mem::{transmute, MaybeUninit};

use alloc::{boxed::Box, string::String, sync::Weak, vec};

use crate::{
    blk::{IORequest, LinearBlockAddress, Partition, BLOCK_SIZE},
    fs::{
        errors::{
            FsCloseError, FsInitError, FsIoctlError, FsOpenError, FsPathError, FsReadError,
            FsStatError, FsWriteError,
        },
        inode::FSInode,
        path::Path,
        FileSystemInner, FileSystemSkeleton, VFS,
    },
    posix::{Stat, S_IFDIR, S_IFREG},
    utils::slot_allocator::SlotAllocator,
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

const DIR_ENTRIES_PER_SECTOR: usize = BLOCK_SIZE / core::mem::size_of::<ShortDirectoryEntry>();
const LONG_DIR_ENTRY_LAST_ENTRY_MARKER: u8 = 0x40;
const MAX_FILENAME_LENGTH: usize = 256;
// TODO: utf-16
const CHARS_PER_LONG_ENTRY: usize = 26;

const FAT_ENTRIES_PER_BLOCK: usize = BLOCK_SIZE / core::mem::size_of::<u32>();

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
    data_cluster_start: ClusterIndex,
    directory_cluster: ClusterIndex,
    directory_cluster_index: usize,
}

impl DirectoryEntry {
    fn file_size(&self) -> usize {
        match self.ent_type {
            DirectoryEntryType::Directory => 0,
            DirectoryEntryType::File(n) => n,
        }
    }
}

#[derive(Debug)]
struct DirectoryIndex {
    cluster: ClusterIndex,
    cluster_index: usize,
}

impl DirectoryIndex {
    fn new(cluster: ClusterIndex, directory_index: usize) -> DirectoryIndex {
        DirectoryIndex {
            cluster,
            cluster_index: directory_index,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
/// Represents a cluster
struct ClusterIndex(usize);

const MAX_VALID_CLUSTER: usize = 0x0FFFFFF7;

impl ClusterIndex {
    #[inline]
    // Returns the block number and local index of where the cluster is in the FAT
    fn fat_position(&self) -> (usize, usize) {
        let block_idx = self.0 / FAT_ENTRIES_PER_BLOCK;
        let idx = self.0 % FAT_ENTRIES_PER_BLOCK;
        (block_idx, idx)
    }

    #[inline]
    pub fn valid_cluster(&self) -> bool {
        self.0 < MAX_VALID_CLUSTER
    }
}

#[derive(Debug)]
struct FATFileSystem {
    partition: Weak<Partition>,

    sector_count: usize,
    reserved_sector_count: usize,
    sectors_per_cluster: usize,
    fat_count: usize,
    data_sectors_start: usize,
    root_cluster: ClusterIndex,

    inode_table: SlotAllocator<DirectoryIndex>,
}

impl FATFileSystem {
    pub fn new(part: Weak<Partition>) -> Result<FATFileSystem, FsInitError> {
        let p = part.upgrade().unwrap();

        let mut bios_parameter_block: [u8; BLOCK_SIZE] = unsafe {
            transmute(MaybeUninit::<[MaybeUninit<u8>; BLOCK_SIZE]>::uninit().assume_init())
        };

        p.read(IORequest::new(
            LinearBlockAddress::new(0),
            1,
            &mut bios_parameter_block[..],
        ))
        .unwrap();

        if bios_parameter_block[510..] != MAGIC_NUMBER {
            return Err(FsInitError::InvalidMagic);
        }

        let bios_parameter_data: &BIOSPBLegacy = unsafe {
            (bios_parameter_block.as_ptr() as *const BIOSPBLegacy)
                .as_ref()
                .unwrap()
        };

        if bios_parameter_data.root_dir_entries != 0 {
            log!("FAT: non FAT-32 FAT filesystem detected");
            return Err(FsInitError::InvalidSuperBlock);
        }

        let extended_bpd: &ExtendedBIOSPB = unsafe {
            (bios_parameter_block
                .as_ptr()
                .add(core::mem::size_of::<BIOSPBLegacy>()) as *const ExtendedBIOSPB)
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

        let mut fs = FATFileSystem {
            partition: part,
            sector_count: lba_count,
            reserved_sector_count,
            data_sectors_start: reserved_sector_count + (fat_count * fat_size) + root_dir_sectors,
            sectors_per_cluster: bios_parameter_data.sectors_per_cluster as usize,
            fat_count,
            root_cluster: ClusterIndex(extended_bpd.root_dir_cluster as usize),
            inode_table: SlotAllocator::new(None),
        };

        // root inode
        fs.inode_table
            .allocate(Some(0), DirectoryIndex::new(ClusterIndex(0), 0));

        Ok(fs)
    }

    #[inline]
    /// Returns the sector where the specified cluster starts
    fn cluster_start_lba(&self, cluster: ClusterIndex) -> LinearBlockAddress {
        assert!(cluster.0 >= 2);
        LinearBlockAddress::new(
            self.data_sectors_start + (cluster.0 - 2) * self.sectors_per_cluster,
        )
    }

    #[inline]
    /// Returns the LBA of the specified block in the FAT
    fn fat_table_lba(&self, block_idx: usize) -> LinearBlockAddress {
        LinearBlockAddress::new(self.reserved_sector_count + block_idx)
    }

    /// Read the specified cluster from the File Allocation Table
    fn get_fat_entry(&self, cluster: ClusterIndex) -> ClusterIndex {
        let (table_lba_idx, table_idx) = cluster.fat_position();

        let p = self.partition.upgrade().unwrap();
        let mut sector_data: [u8; BLOCK_SIZE] = unsafe {
            transmute(MaybeUninit::<[MaybeUninit<u8>; BLOCK_SIZE]>::uninit().assume_init())
        };

        let table_lba = self.fat_table_lba(table_lba_idx);
        p.read(IORequest::new(table_lba, 1, &mut sector_data[..]))
            .unwrap();

        // TODO: do this safely
        let val = unsafe {
            let ptr = (sector_data.as_ptr() as *const u32).add(table_idx);
            ptr.read()
        } as usize;
        ClusterIndex(val & 0x0FFFFFFF)
    }

    fn parse_short_dir_ent_filename(filename: &[u8; 11]) -> String {
        let filebase = &filename[..8];
        let filename_len = filebase.iter().position(|c| *c == b' ').unwrap();
        let filebase_str = core::str::from_utf8(&filebase[..filename_len]).unwrap();

        let extension = &filename[8..];
        let extension_len = extension.iter().position(|c| *c == b' ').unwrap();
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
    fn fuse_cluster_parts(low: u16, high: u16) -> u32 {
        u32::from_le_bytes([low as u8, (low >> 8) as u8, high as u8, (high >> 8) as u8])
    }

    fn find_dir_ent(
        &self,
        dir_start_cluster: ClusterIndex,
        filename: &str,
    ) -> Option<DirectoryEntry> {
        let p = self.partition.upgrade().unwrap();
        let mut sector_data: [u8; BLOCK_SIZE] = unsafe {
            transmute(MaybeUninit::<[MaybeUninit<u8>; BLOCK_SIZE]>::uninit().assume_init())
        };

        let mut long_file_name = String::with_capacity(MAX_FILENAME_LENGTH);
        let mut cluster = dir_start_cluster;

        while cluster.valid_cluster() {
            let sector = self.cluster_start_lba(cluster);
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
                        (sector_data.as_ptr().add(offset) as *const LongDirectoryEntry)
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
                        (sector_data.as_ptr().add(offset) as *const ShortDirectoryEntry)
                            .as_ref()
                            .unwrap()
                    };

                    let ent_type = if ent.attr & DIR_ENT_DIRECTORY > 0 {
                        DirectoryEntryType::Directory
                    } else {
                        DirectoryEntryType::File(ent.file_size as usize)
                    };

                    if !long_file_name.is_empty() {
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
                        data_cluster_start: ClusterIndex(Self::fuse_cluster_parts(
                            ent.cluster_low,
                            ent.cluster_high,
                        ) as usize),
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

    fn get_dir_ent(&self, dir_cluster: ClusterIndex, index: usize) -> DirectoryEntry {
        let p = self.partition.upgrade().unwrap();
        let mut block_data: [u8; BLOCK_SIZE] = unsafe {
            transmute(MaybeUninit::<[MaybeUninit<u8>; BLOCK_SIZE]>::uninit().assume_init())
        };

        let lba = self.cluster_start_lba(dir_cluster);
        p.read(IORequest::new(lba, 1, &mut block_data[..])).unwrap();

        let mut offset = index * core::mem::size_of::<ShortDirectoryEntry>();

        // first byte of the entry
        let long_entry = match block_data[offset] {
            // end of directory entries, unused
            // TODO: return error
            0 | 0xE5 => unreachable!(),
            // attribute
            _ => block_data[offset + 0xB] == DIR_ENT_LONG_NAME,
        };

        if long_entry {
            let ent: &LongDirectoryEntry = unsafe {
                (block_data.as_ptr().add(offset) as *const LongDirectoryEntry)
                    .as_ref()
                    .unwrap()
            };

            let order = if ent.order & LONG_DIR_ENTRY_LAST_ENTRY_MARKER > 0 {
                ent.order ^ LONG_DIR_ENTRY_LAST_ENTRY_MARKER
            } else {
                ent.order
            };

            offset += order as usize * core::mem::size_of::<LongDirectoryEntry>();
        }

        let ent: &ShortDirectoryEntry = unsafe {
            (block_data.as_ptr().add(offset) as *const ShortDirectoryEntry)
                .as_ref()
                .unwrap()
        };

        let ent_type = if ent.attr & DIR_ENT_DIRECTORY > 0 {
            DirectoryEntryType::Directory
        } else {
            DirectoryEntryType::File(ent.file_size as usize)
        };

        DirectoryEntry {
            ent_type,
            data_cluster_start: ClusterIndex(Self::fuse_cluster_parts(
                ent.cluster_low,
                ent.cluster_high,
            ) as usize),
            directory_cluster: dir_cluster,
            directory_cluster_index: index,
        }
    }

    fn get_dir_index_from_inode(&self, inode: FSInode) -> Option<&DirectoryIndex> {
        self.inode_table.get(inode.0 as usize)
    }

    fn find_file(&self, mut path: Path) -> Option<DirectoryEntry> {
        let root_dir_start_cluster = self.root_cluster;
        let mut start_cluster = root_dir_start_cluster;

        while path.components_left() > 1 {
            let comp = path.next().unwrap();
            let dir_ent = self.find_dir_ent(start_cluster, comp);
            match dir_ent {
                Some(ent) => {
                    match ent.ent_type {
                        DirectoryEntryType::File(_) => return None,
                        DirectoryEntryType::Directory => (),
                    }

                    start_cluster = ent.data_cluster_start;
                    if !start_cluster.valid_cluster() {
                        warn!(
                            "directory entry start cluster is not valid: {}",
                            start_cluster.0
                        );
                        return None;
                    }
                }
                None => return None,
            }
        }

        self.find_dir_ent(start_cluster, path.next().unwrap())
    }
}

impl FileSystemInner for FATFileSystem {
    fn open(&mut self, path: Path) -> Result<FSInode, FsOpenError> {
        if path.components_left() == 0 {
            return Ok(FSInode::new(0));
        }

        match self.find_file(path) {
            Some(file) => {
                let inode = self
                    .inode_table
                    .allocate(
                        None,
                        DirectoryIndex::new(file.directory_cluster, file.directory_cluster_index),
                    )
                    .unwrap();
                Ok(FSInode(inode as u64))
            }
            None => Err(FsOpenError::BadPath(FsPathError::NoSuchFileOrDirectory)),
        }
    }

    fn stat(&mut self, inode: FSInode, stat_buf: &mut Stat) -> Result<(), FsStatError> {
        let (file_size, file_type) = if inode == FSInode(0) {
            (0, S_IFDIR)
        } else {
            let dir_index = self.get_dir_index_from_inode(inode).expect("Invalid inode");
            let file = self.get_dir_ent(dir_index.cluster, dir_index.cluster_index);

            match file.ent_type {
                DirectoryEntryType::Directory => (0, S_IFDIR),
                DirectoryEntryType::File(n) => (n, S_IFREG),
            }
        };

        stat_buf.st_blksize = BLOCK_SIZE as u64;
        stat_buf.st_size = file_size as u64;
        stat_buf.st_ino = inode.0;
        stat_buf.st_mode = file_type | 0o777;

        // TODO: make sure we can determine st_blocks with this calculation only
        stat_buf.st_blocks = file_size.div_ceil(BLOCK_SIZE) as u64;

        Ok(())
    }

    fn close(&mut self, inode: FSInode) -> Result<(), FsCloseError> {
        if inode == FSInode(0) {
            return Ok(());
        }

        self.inode_table.deallocate(inode.0 as usize);
        Ok(())
    }

    fn read(
        &mut self,
        inode: FSInode,
        offset: usize,
        buff: &mut [u8],
    ) -> Result<usize, FsReadError> {
        assert!(inode != FSInode(0));

        let part = self.partition.upgrade().unwrap();

        let dir_index = self.get_dir_index_from_inode(inode).expect("Invalid inode");
        let file = self.get_dir_ent(dir_index.cluster, dir_index.cluster_index);

        let lba = offset / BLOCK_SIZE;
        let mut cluster = file.data_cluster_start;
        for _ in 0..lba {
            cluster = self.get_fat_entry(cluster);
            assert!(cluster.valid_cluster());
        }

        let mut buff_left = buff.len();
        let mut size_left = file.file_size();

        if offset >= size_left {
            return Ok(0);
        }

        let cluster_size = self.sectors_per_cluster * BLOCK_SIZE;

        let mut total_read = 0;
        let mut start_off = offset % cluster_size;

        while size_left > 0 && buff_left > 0 {
            assert!(cluster.valid_cluster());

            let read = (if start_off > 0 {
                cluster_size - start_off
            } else {
                size_left
            })
            .min(cluster_size)
            .min(buff_left)
            .min(size_left);

            let sub_buff = &mut buff[total_read..total_read + read];

            if read == cluster_size {
                part.read(IORequest {
                    lba: self.cluster_start_lba(cluster),
                    buff: &mut sub_buff[..],
                    size: self.sectors_per_cluster,
                })
                .unwrap();
            } else {
                // TODO
                let mut sector_buff = vec![0; cluster_size];

                part.read(IORequest {
                    lba: self.cluster_start_lba(cluster),
                    buff: &mut sector_buff[..],
                    size: self.sectors_per_cluster,
                })
                .unwrap();

                sub_buff.copy_from_slice(&sector_buff[..read]);
            }

            total_read += read;
            size_left -= read;
            buff_left -= read;
            start_off = 0;

            cluster = self.get_fat_entry(cluster);
        }

        Ok(total_read)
    }

    fn write(
        &mut self,
        inode: FSInode,
        _offset: usize,
        _buff: &[u8],
    ) -> Result<usize, FsWriteError> {
        assert!(inode != FSInode(0));
        todo!()
    }

    fn ioctl(&mut self, _inode: FSInode, _req: usize, _arg: usize) -> Result<usize, FsIoctlError> {
        todo!()
    }
}

fn create_fs(part: Weak<Partition>) -> Result<Box<dyn FileSystemInner>, FsInitError> {
    match FATFileSystem::new(part) {
        Ok(fs) => Ok(Box::new(fs)),
        Err(err) => Err(err),
    }
}

pub fn init() -> bool {
    let mut vfs = VFS.write();
    vfs.register_fs_skeleton(FileSystemSkeleton {
        new: create_fs,
        name: "fat32",
    })
    .is_ok()
}
