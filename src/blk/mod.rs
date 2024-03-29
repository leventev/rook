use core::{
    fmt::Debug,
    mem::size_of,
    ops::{Add, Deref, DerefMut},
};

use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
    vec::Vec,
};
use spin::Mutex;

pub const BLOCK_SIZE: usize = 512;

struct BlockDeviceManager {
    block_devices: Vec<Arc<BlockDevice>>,
    partitions: Vec<Arc<Partition>>,
}

unsafe impl Send for BlockDeviceManager {}

static BLOCK_DEVICE_MANAGER: Mutex<BlockDeviceManager> = Mutex::new(BlockDeviceManager {
    block_devices: Vec::new(),
    partitions: Vec::new(),
});

#[repr(C, packed)]
/// Represents an entry in the Master Boot Record partition table
struct MBREntry {
    /// 0x80 means the partition is bootable, 0x0 means it's not
    bootable: u8,

    /// Head of the sector where the partition starts
    start_head: u8,

    /// First 6 bits are the sector, last 10 bits are the cylinder of sector where the partition starts
    start_sector_cylinder: u16,

    /// File system identifier
    system_id: u8,

    /// Head of the last sector in the partition
    last_partition_head: u8,

    /// First 6 bits are the sector, last 10 bits are the cylinder of the last sector in the partition
    last_partition_sector_cylinder: u16,

    /// LBA of the start of the partition
    start_lba: u32,

    /// Partition size in LBAs
    lba_count: u32,
}

/// Represents a Linear Base Address(sector)
#[derive(Debug, PartialEq, PartialOrd, Clone)]
#[repr(transparent)]
pub struct LinearBlockAddress(usize);

impl LinearBlockAddress {
    /// Creates a new LBA from a usize
    pub fn new(lba: usize) -> LinearBlockAddress {
        LinearBlockAddress(lba)
    }

    /// Consumes an LBA and returns the inner usize
    pub fn inner(self) -> usize {
        self.0
    }
}

impl Deref for LinearBlockAddress {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LinearBlockAddress {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Add for LinearBlockAddress {
    type Output = LinearBlockAddress;
    fn add(self, rhs: Self) -> Self::Output {
        LinearBlockAddress(self.0 + rhs.0)
    }
}

/// Represents either a write or read request to a block device
#[derive(Debug)]
pub struct IORequest<'a> {
    /// Start LBA
    pub lba: LinearBlockAddress,

    /// Size of the request of LBAs
    pub size: usize,

    /// Buffer to write from/read to, must equal __size__ multiplied by the size
    /// of an LBA of the target device
    pub buff: &'a mut [u8],
}

impl<'a> IORequest<'a> {
    pub fn new(lba: LinearBlockAddress, size: usize, buff: &'a mut [u8]) -> IORequest<'a> {
        IORequest { lba, size, buff }
    }
}

#[derive(Debug)]
pub enum BlockDeviceError {
    FailedToReadSectors,
}

pub trait BlockOperations: Send + Debug {
    /// Sends a read request
    fn read(&self, req: IORequest) -> Result<(), BlockDeviceError>;

    /// Sends a write request
    fn write(&self, req: IORequest) -> Result<(), BlockDeviceError>;
}

#[derive(Debug)]
pub struct BlockDevice {
    pub operations: Box<dyn BlockOperations>,
    pub major: usize,
    pub minor: usize,
    pub name: &'static str,
    pub size: usize,
}

impl BlockDevice {}

pub fn register_blk(
    name: &'static str,
    major: usize,
    size: usize,
    operations: Box<dyn BlockOperations>,
) {
    let mut blk_dev_manager = BLOCK_DEVICE_MANAGER.lock();
    log!("BLK: added block device {}", name);

    let minor = blk_dev_manager
        .block_devices
        .iter()
        .filter(|dev| dev.major == major)
        .count();

    let dev = BlockDevice {
        operations,
        major,
        minor,
        name,
        size,
    };

    let rc = Arc::new(dev);
    let mut parts = parse_partition_table(rc.clone())
        .into_iter()
        .map(Arc::new)
        .collect::<Vec<Arc<Partition>>>();

    for part in parts.iter() {
        log!("{:?}", part);
    }

    blk_dev_manager.block_devices.push(rc);
    blk_dev_manager.partitions.append(&mut parts);
}

pub fn get_partition(major: usize, minor: usize, part_idx: usize) -> Option<Weak<Partition>> {
    let blk_dev_manager = BLOCK_DEVICE_MANAGER.lock();
    let part = blk_dev_manager.partitions.iter().find(|part| {
        let dev = part.block_device.upgrade().unwrap();
        dev.major == major && dev.minor == minor && part.part_idx == part_idx
    });

    part.map(Arc::downgrade)
}

/// Sends a read request to the target block device
pub fn blk_read(block_device: &BlockDevice, req: IORequest) -> Result<(), BlockDeviceError> {
    assert_eq!(req.size % BLOCK_SIZE, 0, "Invalid buffer size");
    assert_ne!(req.size, 0, "Invalid buffer size");
    assert_eq!(
        req.buff.len(),
        req.size * BLOCK_SIZE,
        "Invalid buffer and buffer size"
    );
    assert!(req.lba.0 < block_device.size, "Invalid LBA");
    assert!(req.lba.0 + req.size < block_device.size, "Invalid LBA");

    block_device.operations.read(req)
}

/// Sends a write request to the target block device
pub fn blk_write(block_device: &BlockDevice, req: IORequest) -> Result<(), BlockDeviceError> {
    assert_eq!(req.size % BLOCK_SIZE, 0, "Invalid buffer size");
    assert_ne!(req.size, 0, "Invalid buffer size");
    assert_eq!(
        req.buff.len(),
        req.size * BLOCK_SIZE,
        "Invalid buffer and buffer size"
    );
    assert!(req.lba.0 < block_device.size, "Invalid LBA");
    assert!(req.lba.0 + req.size < block_device.size, "Invalid LBA");

    block_device.operations.write(req)
}

#[derive(Debug)]
/// Represents a partition
pub struct Partition {
    /// Block device where the partition resides
    pub block_device: Weak<BlockDevice>,

    /// Partition index in the block device
    pub part_idx: usize,

    /// LBA of the start of the partition in the associated block device
    pub start: LinearBlockAddress,

    /// Size of the partition in LBAs
    pub size: usize,
}

impl Partition {
    pub fn read(&self, req: IORequest) -> Result<(), BlockDeviceError> {
        let block_dev = self.block_device.upgrade().unwrap();

        assert_ne!(req.size, 0, "Invalid buffer size");
        assert_eq!(
            req.buff.len(),
            req.size * BLOCK_SIZE,
            "Invalid buffer and buffer size"
        );
        assert!(req.lba.0 < self.size, "Invalid LBA");
        assert!(req.lba.0 + req.size < self.size, "Invalid LBA");

        block_dev.operations.read(IORequest {
            lba: self.start.clone() + req.lba,
            size: req.size,
            buff: req.buff,
        })
    }

    pub fn write(&self, req: IORequest) -> Result<(), BlockDeviceError> {
        let block_dev = self.block_device.upgrade().unwrap();

        assert_ne!(req.size, 0, "Invalid buffer size");
        assert_eq!(
            req.buff.len(),
            req.size * BLOCK_SIZE,
            "Invalid buffer and buffer size"
        );
        assert!(req.lba.0 < self.size, "Invalid LBA");
        assert!(req.lba.0 + req.size < self.size, "Invalid LBA");

        block_dev.operations.write(IORequest {
            lba: self.start.clone() + req.lba,
            size: req.size,
            buff: req.buff,
        })
    }
}

fn parse_partition_table(dev: Arc<BlockDevice>) -> Vec<Partition> {
    log!("parse partition table {}", dev.name);

    let mut buff: [u8; 512] = [0; 512];

    dev.operations
        .read(IORequest {
            lba: LinearBlockAddress::new(0),
            size: 1,
            buff: buff.as_mut_slice(),
        })
        .unwrap();

    let mut partitions: Vec<Partition> = Vec::new();

    const MBR_PARTITION_TABLE_START: usize = 0x1BE;
    for i in 0..4 {
        let buff_offset = MBR_PARTITION_TABLE_START + i * size_of::<MBREntry>();

        let start: u32;
        let size: u32;

        unsafe {
            let entry = buff.as_ptr().add(buff_offset) as *const MBREntry;

            let system_id = (*entry).system_id;
            start = (*entry).start_lba;
            size = (*entry).lba_count;

            if system_id == 0 || start == 0 || size == 0 {
                continue;
            }
        }

        partitions.push(Partition {
            block_device: Arc::downgrade(&dev),
            part_idx: partitions.len(),
            start: LinearBlockAddress::new(start as usize),
            size: size as usize,
        })
    }

    partitions
}
