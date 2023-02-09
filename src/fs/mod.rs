use core::cell::{Cell, RefCell};

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

pub trait FileSystemInner {
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

struct VFSNode {
    mount: Weak<MountPoint>,
    fds_open: Cell<usize>,
    size: Cell<usize>,
    inode: FSInode,
}

pub struct FileDescriptor {
    node: Weak<VFSNode>,
    offset: usize,
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

        let vnode = self.node.upgrade().unwrap();
        let mount = vnode.mount.upgrade().unwrap();

        let read = mount.fs.inner.read(vnode.inode, self.offset, buff, size)?;
        self.offset += read;

        Ok(read)
    }

    pub fn write(&mut self, size: usize, buff: &mut [u8]) -> Result<usize, FileSystemError> {
        if buff.len() != size {
            return Err(FileSystemError::InvalidBuffer);
        }

        if buff.len() == 0 {
            return Ok(0);
        }

        let vnode = self.node.upgrade().unwrap();
        let mount = vnode.mount.upgrade().unwrap();

        let read = mount.fs.inner.write(vnode.inode, self.offset, buff, size)?;
        self.offset += read;

        Ok(read)
    }

    pub fn file_info(&self) -> Result<FileInfo, FileSystemError> {
        let vnode = self.node.upgrade().unwrap();
        let mount = vnode.mount.upgrade().unwrap();

        mount.fs.inner.file_info(vnode.inode)
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

        if self.find_path_mount(&parsed_path).is_some() {
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
        for i in (1..path.len() + 1).rev() {
            let subpath = &path[1..i];

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
            let inode = mount.fs.inner.open(&subpath)?;

            let n = VFSNode {
                mount: Rc::downgrade(&mount),
                inode,
                fds_open: Cell::new(0),
                size: Cell::new(1234),
            };

            mount
                .nodes
                .borrow_mut()
                .insert(subpath.to_vec(), Rc::new(n));
        }

        let binding = mount.nodes.borrow();
        let node = binding.get(subpath).unwrap();

        node.fds_open.set(node.fds_open.get() + 1);
        Ok(Box::new(FileDescriptor {
            node: Rc::downgrade(node),
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

pub fn mount(path: String, part: Weak<Partition>, fs_name: &str) -> Result<(), FileSystemError> {
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

pub fn init() {
    println!("vfs initialized");
}
