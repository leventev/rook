use alloc::{
    boxed::Box,
    format,
    rc::Weak,
    string::String,
    vec::Vec,
};
use spin::RwLock;

use crate::blk::Partition;

#[derive(Debug)]
pub enum FileSystemError {
    FailedToInitialize,
    AlreadyMounted,
    FsSkeletonNotFound,
    FsSkeletonAlreadyExists,
}

pub trait FileSystemInner {
    // Opens a file, returns the inode
    fn open(&self, path: Vec<String>) -> Result<usize, FileSystemError>;

    // Opens a file, returns the inode
    fn close(&self, inode: usize) -> Result<(), FileSystemError>;

    // Reads __size__ bytes into __buff__ from the __offset__, returns the number of bytes read
    fn read(
        &self,
        inode: usize,
        offset: usize,
        buff: &mut [u8],
        size: usize,
    ) -> Result<usize, FileSystemError>;

    // Write __size__ bytes from __buff__ to the __offset__, returns the number of bytes written
    fn write(
        &self,
        inode: usize,
        offset: usize,
        buff: &[u8],
        size: usize,
    ) -> Result<usize, FileSystemError>;
}

#[derive(Debug)]
pub struct FileSystemSkeleton {
    pub new: fn(part: Weak<Partition>) -> Box<dyn FileSystemInner>,
    pub name: &'static str,
}

pub struct FileSystem {
    name: &'static str,
    inner: Box<dyn FileSystemInner>,
}

struct MountPoint {
    path: String,
    fs: FileSystem,
}

struct VirtualFileSystem {
    fs_skeletons: Vec<FileSystemSkeleton>,
    mounts: Vec<MountPoint>,
}

unsafe impl Send for VirtualFileSystem {}
unsafe impl Sync for VirtualFileSystem {}

impl VirtualFileSystem {
    const fn new() -> VirtualFileSystem {
        VirtualFileSystem {
            mounts: Vec::new(),
            fs_skeletons: Vec::new(),
        }
    }

    /// Finds the skeleton file system for __skel_name__ and creates a new instance of it
    fn create_new_filesystem(
        &mut self,
        skel_name: &str,
        part: Weak<Partition>,
    ) -> Result<FileSystem, FileSystemError> {
        match self.fs_skeletons.iter().find(|fs| fs.name == skel_name) {
            Some(fs) => Ok(FileSystem {
                name: fs.name,
                inner: (fs.new)(part),
            }),
            None => Err(FileSystemError::FsSkeletonNotFound),
        }
    }

    /// Registers a skeleton file system
    fn register_fs_skeleton(&mut self, skel: FileSystemSkeleton) -> Result<(), FileSystemError> {
        if self
            .fs_skeletons
            .iter()
            .find(|fs| fs.name == skel.name)
            .is_some()
        {
            return Err(FileSystemError::FsSkeletonAlreadyExists);
        }

        if cfg!(vfs_debug) {
            println!("VFS: registered {} file system skeleton", skel.name);
        }

        self.fs_skeletons.push(skel);
        Ok(())
    }

    fn mount(
        &mut self,
        path: String,
        part: Weak<Partition>,
        fs_name: &str,
    ) -> Result<(), FileSystemError> {
        // TODO: check if its already mounted
        // TODO: validate path
        
        if cfg!(vfs_debug) {
            let blk_dev_name = {
                let part = part.upgrade().unwrap();
                let blk_dev = part.block_device.upgrade().unwrap();
                format!(
                    "device: {} major: {} minor: {} part: {}",
                    blk_dev.name, blk_dev.major, blk_dev.minor, part.part_idx
                )
            };
            println!(
                "VFS: trying to mount {}({}) filesystem to {} ",
                fs_name, blk_dev_name, path
            );
        }
        
        let filesystem = self.create_new_filesystem(fs_name, part)?;

        self.mounts.push(MountPoint {
            path,
            fs: filesystem,
        });

        Ok(())
    }
}

static VFS: RwLock<VirtualFileSystem> = RwLock::new(VirtualFileSystem::new());

/// Parses a string and returns a vector of the parts without the /-s except the first
fn parse_path(path: String) -> Option<Vec<String>> {
    if !path.starts_with("/") {
        return None;
    }

    let mut parts = path.split("/");
    if parts.find(|s| s.len() == 0).is_some() {
        return None;
    }

    let parts: Vec<String> = [
        [String::from("/")].to_vec(),
        parts.map(|s| String::from(s)).collect::<Vec<String>>(),
    ]
    .concat();
    Some(parts)
}

pub fn mount(path: String, part: Weak<Partition>, fs_name: &str) -> Result<(), FileSystemError> {
    let mut vfs = VFS.write();
    vfs.mount(path, part, fs_name)
}

pub fn register_fs_skeleton(skel: FileSystemSkeleton) -> Result<(), FileSystemError> {
    let mut vfs = VFS.write();
    vfs.register_fs_skeleton(skel)
}

pub fn init() {
    println!("vfs initialized");
}
