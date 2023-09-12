use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use hashbrown::HashMap;
use spin::{Lazy, Mutex};

use crate::posix::Stat;

use super::{
    inode::FSInode, path::Path, FileSystem, FileSystemInner, FsCloseError, FsIoctlError,
    FsOpenError, FsPathError, FsReadError, FsStatError, FsWriteError, VFS,
};

pub trait DevFsDevice {
    fn read(&self, minor: u16, off: usize, buff: &mut [u8]) -> Result<usize, FsReadError>;

    fn write(&self, minor: u16, off: usize, buff: &[u8]) -> Result<usize, FsWriteError>;

    fn ioctl(&self, minor: u16, req: usize, arg: usize) -> Result<usize, FsIoctlError>;

    fn stat(&self, minor: u16, stat_buf: &mut Stat) -> Result<(), FsStatError>;
}

#[derive(Debug)]
enum DeviceFileTreeNode {
    Directory(Vec<(String, DeviceFileTreeNode)>),
    File(FSInode),
}

struct DeviceFileSystemInner {
    pub root_node: DeviceFileTreeNode,
    pub major_operations: HashMap<u16, Arc<dyn DevFsDevice>>,
}

unsafe impl Send for DeviceFileSystemInner {}

static DEVFS_INNER: Lazy<Mutex<DeviceFileSystemInner>> =
    Lazy::new(|| Mutex::new(DeviceFileSystemInner::new()));

#[derive(Debug)]
pub enum DevFsError {
    BadPath(FsPathError),
    AlreadyExists,
    MajorAlreadyRegistered,
    IsFile,
}

#[derive(Debug)]
struct DeviceFileSystem {}

impl DeviceFileSystemInner {
    fn new() -> DeviceFileSystemInner {
        DeviceFileSystemInner {
            root_node: DeviceFileTreeNode::Directory(Vec::new()),
            major_operations: HashMap::new(),
        }
    }
}

impl FileSystemInner for DeviceFileSystem {
    fn open(&mut self, path: Path) -> Result<FSInode, FsOpenError> {
        let mut inner = DEVFS_INNER.lock();

        let node = inner.get_node(path).map_err(FsOpenError::BadPath)?;

        match node {
            DeviceFileTreeNode::Directory(_) => panic!("not implemented"),
            DeviceFileTreeNode::File(inode) => Ok(*inode),
        }
    }

    fn close(&mut self, _inode: FSInode) -> Result<(), FsCloseError> {
        warn!("devfs close unimplemented");
        Ok(())
    }

    fn stat(&mut self, inode: FSInode, stat_buf: &mut Stat) -> Result<(), FsStatError> {
        let mut inner = DEVFS_INNER.lock();

        let (major, minor) = inode_to_dev_number(inode);
        let ops = inner.major_operations.get_mut(&major).unwrap();

        ops.stat(minor, stat_buf)
    }

    fn read(&mut self, inode: FSInode, off: usize, buff: &mut [u8]) -> Result<usize, FsReadError> {
        // TODO: check if inode is valid
        let mut inner = DEVFS_INNER.lock();

        let (major, minor) = inode_to_dev_number(inode);
        let ops = inner.major_operations.get_mut(&major).unwrap();

        ops.read(minor, off, buff)
    }

    fn write(&mut self, inode: FSInode, off: usize, buff: &[u8]) -> Result<usize, FsWriteError> {
        // TODO: check if inode is valid
        let mut inner = DEVFS_INNER.lock();

        let (major, minor) = inode_to_dev_number(inode);
        let ops = inner.major_operations.get_mut(&major).unwrap();

        ops.write(minor, off, buff)
    }

    fn ioctl(&mut self, inode: FSInode, req: usize, arg: usize) -> Result<usize, FsIoctlError> {
        // TODO: check if inode is valid
        let mut inner = DEVFS_INNER.lock();

        let (major, minor) = inode_to_dev_number(inode);
        let ops = inner.major_operations.get_mut(&major).unwrap();

        ops.ioctl(minor, req, arg)
    }
}

impl DeviceFileSystemInner {
    /// Traverses the node tree to find a node, if a directory in the path does
    /// not exist an Err is returned otherwise if the last element of the path
    /// exists a mutable reference is returned to it
    fn get_node<'a>(
        &'a mut self,
        mut path: Path,
    ) -> Result<&'a mut DeviceFileTreeNode, FsPathError> {
        let mut node = &mut self.root_node;

        if path.components_left() == 0 {
            return Ok(node);
        }

        while path.components_left() > 1 {
            let comp = path.next().unwrap();
            match node {
                DeviceFileTreeNode::File(_) => return Err(FsPathError::NotADirectory),
                DeviceFileTreeNode::Directory(ref mut entries) => {
                    let new_node = entries.iter_mut().find(|ent| ent.0 == comp);
                    match new_node {
                        Some(n) => node = &mut n.1,
                        None => return Err(FsPathError::NoSuchFileOrDirectory),
                    }
                }
            }
        }

        let last_element = path.next().unwrap();
        match node {
            DeviceFileTreeNode::Directory(entries) => {
                let last_node = entries.iter_mut().find(|ent| ent.0 == *last_element);
                match last_node {
                    Some(n) => Ok(&mut n.1),
                    None => Err(FsPathError::NoSuchFileOrDirectory),
                }
            }
            // we already know the node is a directory
            DeviceFileTreeNode::File(_) => unreachable!(),
        }
    }
}

fn dev_number_to_inode(major: u16, minor: u16) -> FSInode {
    FSInode::new((major as u64) << 16 | minor as u64)
}

fn inode_to_dev_number(inode: FSInode) -> (u16, u16) {
    let major = (inode.0 >> 16) & 0xFFF;
    let minor = inode.0 & 0xFFFF;
    (major as u16, minor as u16)
}

pub fn register_devfs_node(mut path: Path, major: u16, minor: u16) -> Result<(), DevFsError> {
    let inode = dev_number_to_inode(major, minor);

    let mut inner = DEVFS_INNER.lock();
    let mut node = &mut inner.root_node;

    if path.components_left() == 0 {
        return Err(DevFsError::AlreadyExists);
    }

    while path.components_left() > 1 {
        let comp = path.next().unwrap();
        match node {
            DeviceFileTreeNode::File(_) => {
                return Err(DevFsError::BadPath(FsPathError::NotADirectory))
            }
            DeviceFileTreeNode::Directory(ref mut entries) => {
                let new_node = entries.iter_mut().find(|ent| ent.0 == comp);
                match new_node {
                    Some(n) => node = &mut n.1,
                    None => return Err(DevFsError::BadPath(FsPathError::NoSuchFileOrDirectory)),
                }
            }
        }
    }

    let last_element = path.next().unwrap();
    match node {
        DeviceFileTreeNode::Directory(entries) => {
            let last_node = entries.iter_mut().find(|ent| ent.0 == *last_element);
            match last_node {
                Some(_) => return Err(DevFsError::AlreadyExists),
                None => entries.push((last_element.to_string(), DeviceFileTreeNode::File(inode))),
            }
        }
        DeviceFileTreeNode::File(_) => return Err(DevFsError::BadPath(FsPathError::NotADirectory)),
    }

    Ok(())
}

pub fn register_devfs_node_operations(
    major: u16,
    ops: Arc<dyn DevFsDevice>,
) -> Result<(), DevFsError> {
    let mut inner = DEVFS_INNER.lock();
    if inner.major_operations.contains_key(&major) {
        return Err(DevFsError::MajorAlreadyRegistered);
    }

    inner.major_operations.insert(major, ops);
    Ok(())
}

pub fn init() {
    let mut vfs = VFS.write();
    vfs.mount_special(
        "/dev",
        FileSystem {
            name: "devfs",
            inner: Box::new(DeviceFileSystem {}),
        },
    )
    .unwrap();
}
