use alloc::{boxed::Box, rc::Weak};

use crate::{
    blk::Partition,
    fs::{self, FileSystemInner, FileSystemSkeleton, path::Path},
};

struct FATFileSystem {
    partition: Weak<Partition>,
}

impl FileSystemInner for FATFileSystem {
    fn open(
        &self,
        _path: &Path,
    ) -> Result<usize, fs::FileSystemError> {
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

fn create_fs(part: Weak<Partition>) -> Box<dyn FileSystemInner> {
    Box::new(FATFileSystem { partition: part })
}

pub fn init() -> bool {
    fs::register_fs_skeleton(FileSystemSkeleton {
        new: create_fs,
        name: "FAT",
    })
    .is_ok()
}
