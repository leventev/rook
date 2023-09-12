use alloc::sync::Weak;
use spin::Mutex;

use crate::posix::{Stat, FileOpenFlags};

use super::{SeekWhence, FsIoctlError, VFSNodeType, FsStatError, FsWriteError, FsReadError, VFSNode, errors::FsSeekError};

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub vnode: Weak<Mutex<VFSNode>>,
    pub offset: usize,
    pub flags: FileOpenFlags,
}

impl Drop for FileDescriptor {
    fn drop(&mut self) {
        warn!("file descriptor dropped");
        // TODO
    }
}

impl FileDescriptor {
    pub fn read(&mut self, buff: &mut [u8]) -> Result<usize, FsReadError> {
        if buff.is_empty() {
            return Ok(0);
        }

        let vnode = self.vnode.upgrade().unwrap();
        let vnode = vnode.lock();

        let file_data = match &vnode.node_type {
            VFSNodeType::File(data) => data,
            _ => unreachable!(),
        };

        let mount_lock = file_data.mount.upgrade().unwrap();
        let mut mount = mount_lock.lock();
        let fs = mount.get_fs().unwrap();

        let read = fs.inner.read(file_data.inode, self.offset, buff)?;
        self.offset += read;

        Ok(read)
    }

    pub fn write(&mut self, buff: &[u8]) -> Result<usize, FsWriteError> {
        if buff.is_empty() {
            return Ok(0);
        }

        let vnode = self.vnode.upgrade().unwrap();
        let vnode = vnode.lock();

        let file_data = match &vnode.node_type {
            VFSNodeType::File(data) => data,
            _ => unreachable!(),
        };

        let mount_lock = file_data.mount.upgrade().unwrap();
        let mut mount = mount_lock.lock();
        let fs = mount.get_fs().unwrap();

        let read = fs.inner.write(file_data.inode, self.offset, buff)?;
        self.offset += read;

        Ok(read)
    }

    pub fn stat(&self, stat_buf: &mut Stat) -> Result<(), FsStatError> {
        let vnode = self.vnode.upgrade().unwrap();
        let vnode = vnode.lock();

        let file_data = match &vnode.node_type {
            VFSNodeType::File(data) => data,
            _ => unreachable!(),
        };

        let mount_lock = file_data.mount.upgrade().unwrap();
        let mut mount = mount_lock.lock();
        let fs = mount.get_fs().unwrap();

        fs.inner.stat(file_data.inode, stat_buf)
    }

    pub fn ioctl(&self, req: usize, arg: usize) -> Result<usize, FsIoctlError> {
        let vnode = self.vnode.upgrade().unwrap();
        let vnode = vnode.lock();

        let file_data = match &vnode.node_type {
            VFSNodeType::File(data) => data,
            _ => unreachable!(),
        };

        let mount_lock = file_data.mount.upgrade().unwrap();
        let mut mount = mount_lock.lock();
        let fs = mount.get_fs().unwrap();

        fs.inner.ioctl(file_data.inode, req, arg)
    }

    pub fn lseek(&mut self, offset: usize, whence: SeekWhence) -> Result<usize, FsSeekError> {
        let new_off = match whence {
            SeekWhence::Set => offset,
            SeekWhence::Cur => self.offset + offset,
            SeekWhence::End => {
                // TODO: normal SeekWhence::End
                let mut buff = Stat::zero();
                self.stat(&mut buff).unwrap();
                buff.st_size as usize + offset
            }
        };

        self.offset = new_off;

        Ok(new_off)
    }
}