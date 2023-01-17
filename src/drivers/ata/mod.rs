use core::mem::MaybeUninit;

use alloc::boxed::Box;
use spin::Mutex;

use crate::{
    arch::x86_64::{inb, inw, outb},
    blk,
};

bitflags::bitflags! {
    pub struct ATAStatus: u8 {
        const ERROR = 1 << 0;
        const INDEX = 1 << 1;
        const CORRECTED_DATA = 1 << 2;
        const DATA_REQUEST_READY = 1 << 3;
        const DRIVE_SEEK_COMPLETE = 1 << 4;
        const DRIVE_FAULT = 1 << 5;
        const READY = 1 << 6;
        const BUSY = 1 << 7;
    }

    pub struct ATAError: u8 {
        const NO_ADDRESS_MARK = 1 << 0;
        const TRACK_ZERO_NOT_FOUND = 1 << 1;
        const COMMAND_ABORTED = 1 << 2;
        const MEDIA_CHANGE_REQUEST = 1 << 3;
        const ID_MARK_NOT_FOUND = 1 << 4;
        const MEDIA_CHANGED = 1 << 5;
        const UNCORRECTABLE_DATA = 1 << 6;
        const BAD_BLOCK = 1 << 7;
    }
}

// https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/ata/ns-ata-_identify_device_data
const ID_DEVICETYPE: isize = 0x00;
const ID_CYLINDERS: isize = 0x02;
const ID_HEADS: isize = 0x06;
const ID_SECTORS: isize = 0x0C;
const ID_SERIAL: isize = 0x14;
const ID_MODEL: isize = 0x36;
const ID_CAPABILITIES: isize = 0x62;
const ID_FIELDVALID: isize = 0x6A;
const ID_MAX_LBA: isize = 0x78;
const ID_COMMANDSETS: isize = 0xA4;
const ID_MAX_LBA_EXT: isize = 0xC8;

const CMD_READ_PIO: u8 = 0x20;
const CMD_READ_PIO_EXT: u8 = 0x24;
const CMD_READ_DMA: u8 = 0xC8;
const CMD_READ_DMA_EXT: u8 = 0x25;
const CMD_WRITE_PIO: u8 = 0x30;
const CMD_WRITE_PIO_EXT: u8 = 0x34;
const CMD_WRITE_DMA: u8 = 0xCA;
const CMD_WRITE_DMA_EXT: u8 = 0x35;
const CMD_FLUSH_CACHE: u8 = 0xE7;
const CMD_FLUSH_CACHE_EXT: u8 = 0xEA;
const CMD_PACKET: u8 = 0xA0;
const CMD_IDENTIFY_PACKET: u8 = 0xA1;
const CMD_IDENTIFY: u8 = 0xEC;

const REG_DATA: u16 = 0x00;
const REG_ERROR: u16 = 0x01;
const REG_FEATURES: u16 = 0x01;
const REG_SECCOUNT0: u16 = 0x02;
const REG_LBA0: u16 = 0x03;
const REG_LBA1: u16 = 0x04;
const REG_LBA2: u16 = 0x05;
const REG_DRIVE: u16 = 0x06;
const REG_COMMAND: u16 = 0x07;
const REG_STATUS: u16 = 0x07;

const ST_ERROR: u8 = 1 << 0;
const ST_INDEX: u8 = 1 << 1;
const ST_CORRECTED_DATA: u8 = 1 << 2;
const ST_DATA_REQUEST_READY: u8 = 1 << 3;
const ST_DRIVE_SEEK_COMPLETE: u8 = 1 << 4;
const ST_DRIVE_FAULT: u8 = 1 << 5;
const ST_READY: u8 = 1 << 6;
const ST_BUSY: u8 = 1 << 7;

const SECTOR_SIZE: usize = 512;

pub const ATA_PRIMARY_BUS: u16 = 0x1F0;
pub const ATA_SECONDARY_BUS: u16 = 0x170;
pub const ATA_MASTER_DRIVE: u8 = 0xA0;
pub const ATA_SLAVE_DRIVE: u8 = 0xB0;

/// this is a wrapper for the ATA controller
struct ATADevice {
    current_bus: u16,
    current_drive: u8,
}

/// this is a wrapper for an ATA drive
struct ATADrive {
    size: usize, // lba
}

static ATA_DEVICE: Mutex<ATADevice> = Mutex::new(ATADevice::new());

impl blk::BlockDevice for ATADrive {
    fn read(&mut self, req: blk::BlockRequest) -> Result<(), blk::BlockDeviceError> {
        Ok(())
    }

    fn write(&mut self, req: blk::BlockRequest) -> Result<(), blk::BlockDeviceError> {
        Ok(())
    }

    fn size(&self) -> usize {
        self.size
    }

    fn lba_size(&self) -> usize {
        SECTOR_SIZE
    }

    fn name(&self) -> &str {
        "ATA"
    }
}

impl ATADevice {
    fn select_drive(&mut self, bus: u16, drive: u8) {
        assert!(
            (bus == ATA_PRIMARY_BUS || bus == ATA_SECONDARY_BUS)
                && (drive == ATA_MASTER_DRIVE || drive == ATA_SLAVE_DRIVE)
        );

        // only select drive when its necessary
        if bus == self.current_bus && drive == self.current_drive {
            return;
        }

        self.current_bus = bus;
        self.current_drive = drive;
        outb(bus + REG_DRIVE, drive);
    }

    const fn new() -> ATADevice {
        ATADevice {
            current_bus: 0,
            current_drive: 0,
        }
    }

    fn try_identify(&mut self, bus: u16, drive: u8) -> Option<ATADrive> {
        self.select_drive(bus, drive);

        outb(bus + REG_SECCOUNT0, 0);
        outb(bus + REG_LBA0, 0);
        outb(bus + REG_LBA1, 0);
        outb(bus + REG_LBA2, 0);

        outb(bus + REG_COMMAND, CMD_IDENTIFY);

        let mut status = inb(bus + REG_STATUS);
        if status == 0 {
            return None;
        }

        while inb(bus + REG_STATUS) & ST_BUSY > 0 {
            let lba1 = inb(bus + REG_LBA1);
            let lba2 = inb(bus + REG_LBA2);
            if lba1 != 0 || lba2 != 0 {
                // TODO: ATAPI
                return None;
            }
        }

        while (status & ST_DATA_REQUEST_READY) == 0 && (status & ST_ERROR) == 0 {
            status = inb(bus + REG_STATUS);
        }

        if status & ST_ERROR > 0 {
            return None;
        }

        let mut device_data: [MaybeUninit<u8>; SECTOR_SIZE] =
            unsafe { MaybeUninit::uninit().assume_init() };

        let ptr = device_data.as_mut_ptr() as *mut u16;
        for i in 0..SECTOR_SIZE / 2 {
            unsafe {
                let addr = ptr.offset(i as isize);
                let data = inw(bus + REG_DATA);
                *addr = data;
            }
        }

        let max_lba = unsafe { *((device_data.as_ptr()).offset(ID_MAX_LBA) as *const u32) };

        Some(ATADrive {
            size: max_lba as usize,
        })
    }
}

pub fn init() -> bool {
    let mut device = ATA_DEVICE.lock();

    for bus in [ATA_PRIMARY_BUS, ATA_SECONDARY_BUS] {
        for drive in [ATA_MASTER_DRIVE, ATA_SLAVE_DRIVE] {
            if let Some(identified_drive) = device.try_identify(bus, drive) {
                let bus_str = match bus {
                    ATA_PRIMARY_BUS => "primary",
                    _ => "secondary",
                };

                let drive_str = match drive {
                    ATA_MASTER_DRIVE => "master",
                    _ => "slave",
                };

                println!(
                    "ATA: found device on the {} bus/{} drive with LBA count: {}",
                    bus_str, drive_str, identified_drive.size
                );
                blk::register(Box::new(identified_drive));
            }
        }
    }

    true
}
