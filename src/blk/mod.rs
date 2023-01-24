use core::fmt::Debug;

use alloc::{boxed::Box, vec::Vec};
use spin::Mutex;

pub static BLOCK_DEVICES: Mutex<Vec<Box<dyn BlockDevice>>> = Mutex::new(Vec::new());

struct MBREntry {

}

pub struct BlockRequest {
    pub lba: usize,
    pub size: usize,
    pub buff: *mut u8,
}

#[derive(Debug)]
pub enum BlockDeviceError {
    FailedToReadSectors,
}

pub trait BlockDevice: Send + Debug {
    /// queues a read request
    fn read(&mut self, req: BlockRequest) -> Result<(), BlockDeviceError>;

    /// queues a write request
    fn write(&mut self, req: BlockRequest) -> Result<(), BlockDeviceError>;

    /// returns the size of the device in LBAs
    fn size(&self) -> usize;

    /// returns the size of an LBA(usually 512 bytes)
    fn lba_size(&self) -> usize;

    /// returns the name of the block device
    fn name(&self) -> &str;
}

pub fn register(dev: Box<dyn BlockDevice>) {
    let mut blks = BLOCK_DEVICES.lock();
    println!("BLK: added block device {}", dev.name());
    blks.push(dev);
}

fn parse_partition_table(dev: &mut dyn BlockDevice) {
    let mut buff: [u8; 512] = [0; 512];
    dev.read(BlockRequest {
        lba: 0,
        size: 1,
        buff: buff.as_mut_ptr(),
    }).unwrap();

    println!("{:?}", buff);
}

pub fn parse_partition_tables() {
    let mut devices = BLOCK_DEVICES.lock();

    for device in devices.iter_mut() {
        parse_partition_table(device.as_mut());
    }
}
