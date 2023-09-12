use alloc::{
    string::ToString,
    sync::{Arc, Weak},
};
use spin::Mutex;

use crate::{blk::Partition, posix::Stat};

use super::{
    errors::FsMountError, path::Path, FileSystem, FileSystemSkeleton, FsInitError,
    FsPathError, Node, VFSMountData, VFSNode, VFSNodeType, VirtualFileSystem,
};

fn create_mount_point_node(name: &str, parent: Weak<Node>, fs: FileSystem) -> Arc<Node> {
    let node = VFSNode {
        name: name.to_string(),
        parent,
        stat: Stat::zero(),
        node_type: VFSNodeType::MountPoint(VFSMountData::new(fs)),
    };

    Arc::new(Mutex::new(node))
}

impl VirtualFileSystem {
    fn mount_internal(&mut self, path: &str, filesystem: FileSystem) -> Result<(), FsMountError> {
        let mut path =
            Path::new(path).map_err(|err| FsMountError::BadPath(FsPathError::ParseError(err)))?;

        if path.components_left() == 0 {
            return match self.root {
                Some(_) => Err(FsMountError::PathAlreadyInUse),
                None => {
                    self.root = Some(create_mount_point_node("", Weak::new(), filesystem));
                    Ok(())
                }
            };
        }

        let parent_lock = self
            .traverse_path(&mut path, 1)
            .map_err(FsMountError::BadPath)?;

        let name = path.next().unwrap();

        let mut parent = parent_lock.lock();

        let dir_data = parent
            .get_dir_data()
            .ok_or(FsMountError::BadPath(FsPathError::NotADirectory))?;
        let mut entries = dir_data.entries.write();

        match entries.get(name) {
            Some(_) => return Err(FsMountError::PathAlreadyInUse),
            None => entries.insert(
                name.to_string(),
                create_mount_point_node(name, Arc::downgrade(&parent_lock), filesystem),
            ),
        };

        Ok(())
    }

    pub fn mount_special(
        &mut self,
        path: &str,
        filesystem: FileSystem,
    ) -> Result<(), FsMountError> {
        if cfg!(vfs_debug) {
            log!(
                "VFS: attempting to mount {} filesystem to {} ",
                filesystem.name,
                path
            );
        }

        self.mount_internal(path, filesystem)
    }

    pub fn mount(
        &mut self,
        path: &str,
        part: Weak<Partition>,
        fs_name: &str,
    ) -> Result<(), FsMountError> {
        if cfg!(vfs_debug) {
            let blk_dev_name = {
                let part = part.upgrade().unwrap();
                let blk_dev = part.block_device.upgrade().unwrap();
                format!(
                    "device: {} major: {} minor: {} part: {}",
                    blk_dev.name, blk_dev.major, blk_dev.minor, part.part_idx
                )
            };
            log!(
                "VFS: attempting to mount {}({}) filesystem to {} ",
                fs_name,
                blk_dev_name,
                path
            );
        }

        let fs = self
            .create_new_filesystem(fs_name, part)
            .map_err(|err| FsMountError::FileSystemInitFailed(err))?;

        self.mount_internal(path, fs)
    }

    /// Finds the skeleton file system for __skel_name__ and creates a new instance of it
    fn create_new_filesystem(
        &mut self,
        skel_name: &str,
        part: Weak<Partition>,
    ) -> Result<FileSystem, FsInitError> {
        match self.fs_skeletons.iter().find(|fs| fs.name == skel_name) {
            Some(fs) => Ok(FileSystem {
                name: fs.name,
                inner: (fs.new)(part)?,
            }),
            None => Err(FsInitError::InvalidSkeleton),
        }
    }

    /// Registers a skeleton file system
    pub fn register_fs_skeleton(&mut self, skel: FileSystemSkeleton) -> Result<(), ()> {
        if self
            .fs_skeletons
            .iter()
            .find(|fs| fs.name == skel.name)
            .is_some()
        {
            return Err(());
        }

        if cfg!(vfs_debug) {
            log!(
                "VFS: registered {} {:?} file system skeleton",
                skel.name,
                skel.new
            );
        }

        self.fs_skeletons.push(skel);
        Ok(())
    }
}
