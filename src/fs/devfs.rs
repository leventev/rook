use alloc::{boxed::Box, string::String, vec::Vec};
use hashbrown::HashMap;
use spin::{Lazy, Mutex};

use super::{
    inode::FSInode, mount_special, FileInfo, FileSystem, FileSystemError, FileSystemInner,
};

pub trait DeviceOperations {
    fn read(
        &mut self,
        minor: usize,
        offset: usize,
         buff: &mut [u8],
        size: usize,
    ) -> Result<usize, FileSystemError>;
    fn write(
        &mut self,
        minor: usize,
        offset: usize,
        buff: &mut [u8],
        size: usize,
    ) -> Result<usize, FileSystemError>;
}

#[derive(Debug)]
enum DeviceFileTreeNode {
    Directory(Vec<(String, DeviceFileTreeNode)>),
    File(FSInode),
}

struct DeviceFileSystemInner {
    root_node: DeviceFileTreeNode,
    major_operations: HashMap<u16, Box<dyn DeviceOperations>>,
}

unsafe impl Send for DeviceFileSystemInner {}

static DEVFS_INNER: Lazy<Mutex<DeviceFileSystemInner>> =
    Lazy::new(|| Mutex::new(DeviceFileSystemInner::new()));

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
    fn open(&self, path: &[String]) -> Result<FSInode, FileSystemError> {
        let mut inner = DEVFS_INNER.lock();

        let node = match inner.get_node(path) {
            Ok(n) => n,
            Err(_) => return Err(FileSystemError::InvalidPath),
        };

        let node = match node {
            Some(n) => n,
            None => return Err(FileSystemError::FileNotFound),
        };

        match node {
            DeviceFileTreeNode::Directory(_) => panic!("not implemented"),
            DeviceFileTreeNode::File(inode) => Ok(*inode),
        }
    }

    fn close(&self, _inode: FSInode) -> Result<(), FileSystemError> {
        todo!()
    }

    fn file_info(&self, _inode: FSInode) -> Result<FileInfo, FileSystemError> {
        todo!()
    }

    fn read(
        &self,
        _inode: FSInode,
        _offset: usize,
        _buff: &mut [u8],
        _size: usize,
    ) -> Result<usize, FileSystemError> {
        todo!()
    }

    fn write(
        &self,
        _inode: FSInode,
        _offset: usize,
        _buff: &[u8],
        _size: usize,
    ) -> Result<usize, FileSystemError> {
        todo!()
    }
}

impl DeviceFileSystemInner {
    /// Traverses the node tree to find a node, if a directory in the path does
    /// not exist an Err is returned otherwise if the last element of the path
    /// exists a mutable reference is returned to it
    fn get_node<'a>(
        &'a mut self,
        path: &[String],
    ) -> Result<Option<&'a mut DeviceFileTreeNode>, DevFsError> {
        let mut node = &mut self.root_node;

        if path.len() == 0 {
            return Ok(Some(node));
        }

        for part in &path[..path.len() - 1] {
            match node {
                &mut DeviceFileTreeNode::File(_) => return Err(DevFsError::InvalidPath),
                &mut DeviceFileTreeNode::Directory(ref mut entries) => {
                    let new_node = entries.iter_mut().find(|ent| ent.0 == *part);
                    match new_node {
                        Some(n) => node = &mut n.1,
                        None => return Err(DevFsError::InvalidPath),
                    }
                }
            }
        }

        let last_element = &path[path.len() - 1];
        match node {
            DeviceFileTreeNode::Directory(entries) => {
                let last_node = entries.iter_mut().find(|ent| ent.0 == *last_element);
                Ok(match last_node {
                    Some(n) => Some(&mut n.1),
                    None => None,
                })
            }
            // we already know the node is a directory
            DeviceFileTreeNode::File(_) => unreachable!(),
        }
    }
}

fn dev_number_to_inode(major: u16, minor: u16) -> FSInode {
    FSInode::new((major as u64) << 16 | minor as u64)
}

#[derive(Debug)]
pub enum DevFsError {
    InvalidPath,
    MajorAlreadyRegistered,
    AlreadyExists,
    IsDirectory,
    IsFile,
}

pub fn register_devfs_node(path: &[String], major: u16, minor: u16) -> Result<(), DevFsError> {
    let inode = dev_number_to_inode(major, minor);

    let mut inner = DEVFS_INNER.lock();

    let parent_node = match inner.get_node(&path[..path.len() - 1]) {
        Ok(n) => {
            match n {
                Some(n) => n,
                None => return Err(DevFsError::InvalidPath),
            }
        },
        Err(_) => return Err(DevFsError::InvalidPath),
    };

    match parent_node {
        DeviceFileTreeNode::Directory(entries) => {
            let last_part = path.last().unwrap();
            let node = entries.iter().find(|f| f.0 == *last_part);
            match node {
                Some(_) => return Err(DevFsError::AlreadyExists),
                None => entries.push((last_part.clone(), DeviceFileTreeNode::File(inode))),
            }
        }
        DeviceFileTreeNode::File(_) => return Err(DevFsError::IsFile),
    }

    Ok(())
}

pub fn register_devfs_node_operations(
    major: u16,
    ops: Box<dyn DeviceOperations>,
) -> Result<(), DevFsError> {
    let mut inner = DEVFS_INNER.lock();
    if inner.major_operations.contains_key(&major) {
        return Err(DevFsError::MajorAlreadyRegistered);
    }

    inner.major_operations.insert(major, ops);
    Ok(())
}

pub fn init() {
    mount_special(
        "/dev",
        FileSystem {
            name: "devfs",
            inner: Box::new(DeviceFileSystem {}),
        },
    )
    .unwrap();
}
