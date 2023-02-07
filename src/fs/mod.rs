use alloc::{boxed::Box, format, rc::Weak, string::String, vec::Vec};
use spin::RwLock;

use crate::blk::Partition;

use self::{
    inode::Inode,
    path::{Path, PathOwned},
};

pub mod inode;
pub mod path;

#[derive(Debug)]
pub enum FileSystemError {
    FailedToInitializeFileSystem,
    AlreadyMounted,
    FsSkeletonNotFound,
    FsSkeletonAlreadyExists,
    InvalidPath,
    FileNotFound,
}

pub trait FileSystemInner {
    // Opens a file, returns the inode
    fn open(&self, path: &Path) -> Result<Inode, FileSystemError>;

    // Opens a file, returns the inode
    fn close(&self, inode: Inode) -> Result<(), FileSystemError>;

    // Reads __size__ bytes into __buff__ from the __offset__, returns the number of bytes read
    fn read(
        &self,
        inode: Inode,
        offset: usize,
        buff: &mut [u8],
        size: usize,
    ) -> Result<usize, FileSystemError>;

    // Write __size__ bytes from __buff__ to the __offset__, returns the number of bytes written
    fn write(
        &self,
        inode: Inode,
        offset: usize,
        buff: &[u8],
        size: usize,
    ) -> Result<usize, FileSystemError>;
}

#[derive(Debug)]
pub struct FileSystemSkeleton {
    pub new: fn(part: Weak<Partition>) -> Result<Box<dyn FileSystemInner>, FileSystemError>,
    pub name: &'static str,
}

pub struct FileSystem {
    name: &'static str,
    inner: Box<dyn FileSystemInner>,
}

struct MountPoint {
    path: PathOwned,
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
                inner: (fs.new)(part)?,
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
            println!(
                "VFS: registered {} {:?} file system skeleton",
                skel.name, skel.new
            );
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
                "VFS: attempting to mount {}({}) filesystem to {} ",
                fs_name, blk_dev_name, path
            );
        }

        let parsed_path = match parse_path(path.as_str()) {
            Some(s) => s,
            None => return Err(FileSystemError::InvalidPath),
        };

        if self.find_path_mount(&parsed_path.as_path_ref()).is_some() {
            return Err(FileSystemError::AlreadyMounted);
        }

        let filesystem = self.create_new_filesystem(fs_name, part)?;

        self.mounts.push(MountPoint {
            path: parsed_path,
            fs: filesystem,
        });

        Ok(())
    }

    fn find_path_mount(&mut self, path: &Path) -> Option<&mut MountPoint> {
        let mounts = &mut self.mounts;
        for i in (1..path.len() + 1).rev() {
            let subpath = Path::new(&path[1..i]);

            let pos = mounts
                .iter_mut()
                .position(|mount| *&mount.path.as_path_ref() == subpath);

            match pos {
                Some(idx) => return Some(&mut self.mounts[idx]),
                None => continue,
            };
        }

        None
    }

    fn open(&mut self, path: &str) -> Result<(), FileSystemError> {
        let parsed_path = match parse_path(path) {
            Some(s) => s,
            None => return Err(FileSystemError::InvalidPath),
        };

        let mount = self
            .find_path_mount(&parsed_path.as_path_ref())
            .expect("Root filesystem is not mounted");
        let subpath = get_mount_subpath(mount, &parsed_path);
        let inode = mount.fs.inner.open(&subpath)?;
        println!("inode: {}", inode);

        let mut a: [u8; 256] = [0; 256];
        mount.fs.inner.read(inode, 0, &mut a[..], 1)?;

        todo!()
    }
}

static VFS: RwLock<VirtualFileSystem> = RwLock::new(VirtualFileSystem::new());

/// Extract the local mount path of a path
fn get_mount_subpath<'a>(mount: &MountPoint, path: &'a PathOwned) -> Path<'a> {
    Path::new(&path[mount.path.len()..])
}

/// Parses a string and returns a vector of the parts without the /-s except the first
fn parse_path(path: &str) -> Option<PathOwned> {
    if !path.starts_with("/") {
        return None;
    }

    // TODO: check if there are invalid paths such as /test//test2

    let path = path
        .split("/")
        .filter(|s| s.len() > 0)
        .map(|s| String::from(s))
        .collect::<Vec<String>>()
        .into_boxed_slice();

    Some(PathOwned(path))
}

pub fn mount(path: String, part: Weak<Partition>, fs_name: &str) -> Result<(), FileSystemError> {
    let mut vfs = VFS.write();
    vfs.mount(path, part, fs_name)
}

pub fn open(path: &str) -> Result<(), FileSystemError> {
    let mut vfs = VFS.write();
    println!("vfs open {}", path);
    vfs.open(path)
}

pub fn register_fs_skeleton(skel: FileSystemSkeleton) -> Result<(), FileSystemError> {
    let mut vfs = VFS.write();
    vfs.register_fs_skeleton(skel)
}

pub fn init() {
    println!("vfs initialized");
}
