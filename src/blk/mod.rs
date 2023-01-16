use alloc::{vec::Vec, boxed::Box};
use spin::Mutex;

pub static BLOCK_DEVICES: Mutex<Vec<Box<dyn BlockDevice>>> = Mutex::new(Vec::new());

pub struct BlockRequest {
    lba: usize,
    size: usize,
    buff: *mut u8,
    tid: usize
}

pub enum BlockDeviceError {
    FailedToReadSectors
}

pub trait BlockDevice: Send {    
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
