use core::fmt::Debug;

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use spin::{Mutex, RwLock};

use crate::{
    blk::Partition,
    posix::{FileOpenFlags, Stat},
};

use self::{
    errors::{
        FsCloseError, FsInitError, FsIoctlError, FsOpenError, FsPathError, FsReadError,
        FsStatError, FsWriteError,
    },
    fd::FileDescriptor,
    inode::FSInode,
    path::Path,
};

pub mod devfs;
pub mod errors;
pub mod fd;
pub mod inode;
pub mod mount;
pub mod path;

pub enum SeekWhence {
    Set,
    Cur,
    End,
}

pub trait FileSystemInner: Debug {
    /// Opens a file, returns the inode
    fn open(&mut self, path: Path) -> Result<FSInode, FsOpenError>;

    /// Opens a file, returns the inode
    fn close(&mut self, inode: FSInode) -> Result<(), FsCloseError>;

    fn read(&mut self, inode: FSInode, off: usize, buff: &mut [u8]) -> Result<usize, FsReadError>;

    fn write(&mut self, inode: FSInode, off: usize, buff: &[u8]) -> Result<usize, FsWriteError>;

    fn stat(&mut self, inode: FSInode, stat_buf: &mut Stat) -> Result<(), FsStatError>;

    fn ioctl(&mut self, inode: FSInode, req: usize, arg: usize) -> Result<usize, FsIoctlError>;
}

#[derive(Debug)]
pub struct FileSystemSkeleton {
    pub new: fn(part: Weak<Partition>) -> Result<Box<dyn FileSystemInner>, FsInitError>,
    pub name: &'static str,
}

#[derive(Debug)]
pub struct FileSystem {
    name: &'static str,
    inner: Box<dyn FileSystemInner>,
}

#[derive(Debug)]
pub enum FileType {
    Directory,
    CharacterDevice,
    BlockDevice,
    RegularFile,
    FIFO,
    Link,
    Socket,
}

pub struct VirtualFileSystem {
    fs_skeletons: Vec<FileSystemSkeleton>,
    // the root vnode only has one owner but it needs to be an Arc
    // for file descriptors to be able to point to it with a Weak
    root: Option<Arc<Node>>,
}

#[derive(Debug)]
pub struct VFSDirectoryData {
    mount: Weak<Mutex<VFSNode>>,
    entries: RwLock<BTreeMap<String, Arc<Node>>>,
}

#[derive(Debug)]
pub struct VFSFileData {
    mount: Weak<Mutex<VFSNode>>,
    inode: FSInode,
}

#[derive(Debug)]
pub struct VFSMountData {
    fs: FileSystem,
    dir: VFSDirectoryData,
}

#[derive(Debug)]
pub enum VFSNodeType {
    File(VFSFileData),
    Directory(VFSDirectoryData),
    MountPoint(VFSMountData),
}

// TODO: use the same string that the hashmap uses to reduce memory usage
#[derive(Debug)]
pub struct VFSNode {
    name: String,
    node_type: VFSNodeType,
    parent: Weak<Node>,
    stat: Stat,
}

type Node = Mutex<VFSNode>;

impl VFSDirectoryData {
    fn new(mount: Weak<Node>) -> VFSDirectoryData {
        VFSDirectoryData {
            entries: RwLock::new(BTreeMap::new()),
            mount,
        }
    }
}

impl VFSMountData {
    fn new(fs: FileSystem) -> VFSMountData {
        VFSMountData {
            fs,
            dir: VFSDirectoryData::new(Weak::new()),
        }
    }
}

impl VFSFileData {
    fn new(mount: Weak<Mutex<VFSNode>>, inode: FSInode) -> VFSFileData {
        VFSFileData { mount, inode }
    }
}

impl VFSNode {
    fn get_fs(&mut self) -> Option<&mut FileSystem> {
        match &mut self.node_type {
            VFSNodeType::MountPoint(mount) => Some(&mut mount.fs),
            _ => None,
        }
    }

    pub fn get_path(&self) -> String {
        // TODO: optimize
        let mut str = String::new();
        for ch in self.name.chars().rev() {
            str.push(ch);
        }

        let mut parent = self.parent.clone();
        while let Some(p) = parent.upgrade() {
            let p = p.lock();
            str.push('/');
            for ch in p.name.chars().rev() {
                str.push(ch);
            }
            parent = p.parent.clone();
        }

        str.chars().rev().collect()
    }

    pub fn is_file(&self) -> bool {
        matches!(self.node_type, VFSNodeType::File(_))
    }

    pub fn is_mount_point(&self) -> bool {
        matches!(self.node_type, VFSNodeType::MountPoint(_))
    }

    pub fn is_dirile(&self) -> bool {
        matches!(self.node_type, VFSNodeType::Directory(_))
    }

    fn get_dir_data(&mut self) -> Option<&mut VFSDirectoryData> {
        match &mut self.node_type {
            VFSNodeType::File(_) => None,
            VFSNodeType::Directory(dir) | VFSNodeType::MountPoint(VFSMountData { dir, .. }) => {
                Some(dir)
            }
        }
    }
}

unsafe impl Send for VirtualFileSystem {}
unsafe impl Sync for VirtualFileSystem {}

pub fn dir_get_entry(
    parent: Arc<Node>,
    name: &str,
    current_mount: &Arc<Node>,
    subpath: Path,
) -> Result<Arc<Node>, FsPathError> {
    {
        let mut dir = parent.lock();
        let dir_data = dir.get_dir_data().ok_or(FsPathError::NotADirectory)?;
        let entries = dir_data.entries.read();

        if let Some(node) = entries.get(name) {
            return Ok(node.clone());
        }
    }

    // unlock because the parent directory can be the current mount too and create_new_node causes a deadlock if parent is locked

    let node = VirtualFileSystem::create_new_node(name, &parent, current_mount, subpath)
        .map_err(|_| FsPathError::NoSuchFileOrDirectory)?;

    let mut dir = parent.lock();
    let dir_data = dir.get_dir_data().ok_or(FsPathError::NotADirectory)?;
    let mut entries = dir_data.entries.write();

    entries.insert(name.to_string(), node.clone());

    Ok(node)
}

impl VirtualFileSystem {
    const fn new() -> VirtualFileSystem {
        VirtualFileSystem {
            root: None,
            fs_skeletons: Vec::new(),
        }
    }

    fn create_new_node(
        name: &str,
        parent: &Arc<Node>,
        mount_lock: &Arc<Mutex<VFSNode>>,
        subpath: Path,
    ) -> Result<Arc<Node>, FsOpenError> {
        let mut mount = mount_lock.lock();
        let fs = mount.get_fs().unwrap();

        // normal subpath
        let inode = fs.inner.open(subpath)?;

        let mut stat_buf: Stat = Stat::zero();
        fs.inner.stat(inode, &mut stat_buf).unwrap();

        let mount_weak = Arc::downgrade(mount_lock);
        let node_type = match stat_buf.file_type() {
            FileType::Directory => VFSNodeType::Directory(VFSDirectoryData::new(mount_weak)),
            _ => VFSNodeType::File(VFSFileData::new(mount_weak, inode)),
        };

        let node = VFSNode {
            name: name.to_string(),
            parent: Arc::downgrade(parent),
            node_type,
            stat: stat_buf,
        };

        Ok(Arc::new(Mutex::new(node)))
    }

    fn traverse_path(
        &mut self,
        path: &mut Path,
        components_to_leave_out: usize,
    ) -> Result<Arc<Node>, FsPathError> {
        let root_node = self.root.as_ref().expect("Root filesystem is not mounted");
        let mut current_node = root_node.clone();
        let mut current_mount = root_node.clone();
        let mut remaining_path = path.clone();
        let mut subpath_comp_count = 0;

        while path.components_left() > components_to_leave_out {
            subpath_comp_count += 1;
            let comp = path.next().unwrap();
            current_node = dir_get_entry(
                current_node,
                comp,
                &current_mount,
                remaining_path.clone().shorten(subpath_comp_count),
            )?;

            let node = current_node.lock();
            if node.is_mount_point() {
                current_mount = current_node.clone();
                remaining_path = path.clone();
                subpath_comp_count = 0;
            }
        }

        Ok(current_node)
    }

    pub fn open(
        &mut self,
        path: &str,
        flags: FileOpenFlags,
    ) -> Result<Box<FileDescriptor>, FsOpenError> {
        let mut path =
            Path::new(path).map_err(|err| FsOpenError::BadPath(FsPathError::ParseError(err)))?;
        let node = self
            .traverse_path(&mut path, 0)
            .map_err(FsOpenError::BadPath)?;

        Ok(Box::new(FileDescriptor {
            vnode: Arc::downgrade(&node),
            offset: 0,
            flags,
        }))
    }

    pub fn stat(&mut self, path: &str, stat_buf: &mut Stat) -> Result<(), FsStatError> {
        let mut path =
            Path::new(path).map_err(|err| FsStatError::BadPath(FsPathError::ParseError(err)))?;
        let node = self
            .traverse_path(&mut path, 0)
            .map_err(FsStatError::BadPath)?;
        *stat_buf = node.lock().stat.clone();

        Ok(())
    }
}

pub static VFS: RwLock<VirtualFileSystem> = RwLock::new(VirtualFileSystem::new());
