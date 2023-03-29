use core::{
    cell::{Cell, RefCell},
    fmt::Debug,
};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    rc::{Rc, Weak},
    string::String,
    vec::Vec,
};
use spin::RwLock;

use crate::blk::Partition;

use self::inode::FSInode;

pub mod devfs;
pub mod inode;

#[derive(Debug)]
pub enum FileSystemError {
    FailedToInitializeFileSystem,
    AlreadyMounted,
    FsSkeletonNotFound,
    FsSkeletonAlreadyExists,
    InvalidPath,
    FileNotFound,
    InvalidBuffer,
    BlockDeviceError,
    IsDirectory,
}

pub trait FileSystemInner: Debug {
    // Opens a file, returns the inode
    fn open(&self, path: &[String]) -> Result<FSInode, FileSystemError>;

    // Opens a file, returns the inode
    fn close(&self, inode: FSInode) -> Result<(), FileSystemError>;

    // Reads __size__ bytes into __buff__ from the __offset__, returns the number of bytes read
    fn read(
        &self,
        inode: FSInode,
        offset: usize,
        buff: &mut [u8],
        size: usize,
    ) -> Result<usize, FileSystemError>;

    // Write __size__ bytes from __buff__ to the __offset__, returns the number of bytes written
    fn write(
        &self,
        inode: FSInode,
        offset: usize,
        buff: &[u8],
        size: usize,
    ) -> Result<usize, FileSystemError>;

    fn file_info(&self, inode: FSInode) -> Result<FileInfo, FileSystemError>;

    fn ioctl(&self, inode: FSInode, req: usize, arg: usize) -> Result<usize, FileSystemError>;
}

#[derive(Debug)]
pub struct FileSystemSkeleton {
    pub new: fn(part: Weak<Partition>) -> Result<Box<dyn FileSystemInner>, FileSystemError>,
    pub name: &'static str,
}

#[derive(Debug)]
pub struct FileSystem {
    name: &'static str,
    inner: Box<dyn FileSystemInner>,
}

#[derive(Debug)]
struct MountPoint {
    path: Vec<String>,
    fs: FileSystem,
    nodes: RefCell<BTreeMap<Vec<String>, Rc<VFSNode>>>,
}

impl MountPoint {
    /// Extract the local mount path of a path
    fn get_subpath<'a>(&self, path: &'a Vec<String>) -> &'a [String] {
        &path[self.path.len()..]
    }
}

impl PartialEq for MountPoint {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }

    fn ne(&self, other: &Self) -> bool {
        self.path != other.path
    }
}

struct VirtualFileSystem {
    fs_skeletons: Vec<FileSystemSkeleton>,
    mounts: Vec<Rc<MountPoint>>,
}

#[derive(Debug)]
pub struct VFSNode {
    path: String,
    mount: Weak<MountPoint>,
    size: Cell<usize>,
    inode: FSInode,
}

impl VFSNode {
    pub fn path(&self) -> &String {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub vnode: Rc<VFSNode>,
    pub offset: usize,
}

impl Drop for FileDescriptor {
    fn drop(&mut self) {
        let strong_count = Rc::strong_count(&self.vnode);
        // not sure if this is the best way to do this but its fine for now
        // 2 = hashmap and self
        if strong_count == 2 {
            // TODO: remove
        }
    }
}

#[derive(Clone, Copy)]
pub struct FileInfo {
    pub size: usize,
    pub blocks_used: usize,
}

impl FileDescriptor {
    pub fn read(&mut self, size: usize, buff: &mut [u8]) -> Result<usize, FileSystemError> {
        if buff.len() != size {
            return Err(FileSystemError::InvalidBuffer);
        }

        if buff.len() == 0 {
            return Ok(0);
        }

        let mount = self.vnode.mount.upgrade().unwrap();

        let read = mount
            .fs
            .inner
            .read(self.vnode.inode, self.offset, buff, size)?;
        self.offset += read;

        Ok(read)
    }

    pub fn write(&mut self, size: usize, buff: &[u8]) -> Result<usize, FileSystemError> {
        if buff.len() != size {
            return Err(FileSystemError::InvalidBuffer);
        }

        if buff.len() == 0 {
            return Ok(0);
        }

        let mount = self.vnode.mount.upgrade().unwrap();

        let read = mount
            .fs
            .inner
            .write(self.vnode.inode, self.offset, buff, size)?;
        self.offset += read;

        Ok(read)
    }

    pub fn file_info(&self) -> Result<FileInfo, FileSystemError> {
        let mount = self.vnode.mount.upgrade().unwrap();
        mount.fs.inner.file_info(self.vnode.inode)
    }

    pub fn ioctl(&self, req: usize, arg: usize) -> Result<usize, FileSystemError> {
        let mount = self.vnode.mount.upgrade().unwrap();
        mount.fs.inner.ioctl(self.vnode.inode, req, arg)
    }
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

    fn mount_special(&mut self, path: &str, filesystem: FileSystem) -> Result<(), FileSystemError> {
        if cfg!(vfs_debug) {
            println!(
                "VFS: attempting to mount {} filesystem to {} ",
                filesystem.name, path
            );
        }

        let parsed_path = match parse_path(path) {
            Some(s) => s,
            None => return Err(FileSystemError::InvalidPath),
        };

        if self.find_mount(&parsed_path).is_some() {
            return Err(FileSystemError::AlreadyMounted);
        }

        self.mounts.push(Rc::new(MountPoint {
            path: parsed_path,
            fs: filesystem,
            nodes: RefCell::new(BTreeMap::new()),
        }));

        Ok(())
    }

    fn mount(
        &mut self,
        path: &str,
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

        let parsed_path = match parse_path(path) {
            Some(s) => s,
            None => return Err(FileSystemError::InvalidPath),
        };

        if self.find_mount(&parsed_path).is_some() {
            return Err(FileSystemError::AlreadyMounted);
        }

        let filesystem = self.create_new_filesystem(fs_name, part)?;

        self.mounts.push(Rc::new(MountPoint {
            path: parsed_path,
            fs: filesystem,
            nodes: RefCell::new(BTreeMap::new()),
        }));

        Ok(())
    }

    fn find_path_mount(&mut self, path: &Vec<String>) -> Option<Rc<MountPoint>> {
        let mounts = &mut self.mounts;
        for i in (0..path.len() + 1).rev() {
            let subpath = &path[0..i];

            let pos = mounts
                .iter_mut()
                .position(|mount| &mount.path[..] == subpath);

            match pos {
                Some(idx) => return Some(self.mounts[idx].clone()),
                None => continue,
            };
        }

        None
    }

    fn find_mount(&mut self, mount_path: &Vec<String>) -> Option<Rc<MountPoint>> {
        let mounts = &mut self.mounts;
        let pos = mounts
            .iter_mut()
            .position(|mount| &mount.path[..] == mount_path);

        match pos {
            Some(idx) => Some(self.mounts[idx].clone()),
            None => None,
        }
    }

    fn open(&mut self, path: &str) -> Result<Box<FileDescriptor>, FileSystemError> {
        let parsed_path = match parse_path(path) {
            Some(s) => s,
            None => return Err(FileSystemError::InvalidPath),
        };

        let mount = self
            .find_path_mount(&parsed_path)
            .expect("Root filesystem is not mounted");
        let subpath = mount.get_subpath(&parsed_path);

        if !mount.nodes.borrow().contains_key(subpath) {
            let inode = mount.fs.inner.open(subpath)?;

            let n = VFSNode {
                path: String::from(path),
                mount: Rc::downgrade(&mount),
                inode,
                size: Cell::new(1234),
            };

            mount
                .nodes
                .borrow_mut()
                .insert(subpath.to_vec(), Rc::new(n));
        }

        let binding = mount.nodes.borrow();
        let node = binding.get(subpath).unwrap();

        Ok(Box::new(FileDescriptor {
            vnode: Rc::clone(node),
            offset: 0,
        }))
    }
}

static VFS: RwLock<VirtualFileSystem> = RwLock::new(VirtualFileSystem::new());

/// Parses a string and returns a vector of the parts without the /-s except the first
fn parse_path(path: &str) -> Option<Vec<String>> {
    if !path.starts_with("/") {
        return None;
    }

    // TODO: check if there are invalid paths such as /test//test2
    let path = path
        .split("/")
        .filter(|s| s.len() > 0)
        .map(|s| String::from(s))
        .collect::<Vec<String>>();

    Some(path)
}

pub fn mount_special(path: &str, filesystem: FileSystem) -> Result<(), FileSystemError> {
    let mut vfs = VFS.write();
    vfs.mount_special(path, filesystem)
}

pub fn mount(path: &str, part: Weak<Partition>, fs_name: &str) -> Result<(), FileSystemError> {
    let mut vfs = VFS.write();
    vfs.mount(path, part, fs_name)
}

pub fn open(path: &str) -> Result<Box<FileDescriptor>, FileSystemError> {
    let node = {
        let mut vfs = VFS.write();
        match vfs.open(path) {
            Ok(node) => node,
            Err(err) => return Err(err),
        }
    };

    Ok(node)
}

pub fn register_fs_skeleton(skel: FileSystemSkeleton) -> Result<(), FileSystemError> {
    let mut vfs = VFS.write();
    vfs.register_fs_skeleton(skel)
}

pub fn init() {}
