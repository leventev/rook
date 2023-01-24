use core::mem::MaybeUninit;

use alloc::{boxed::Box, vec::Vec};
use spin::Mutex;

use crate::{
    arch::x86_64::{idt::install_interrupt_handler, inb, inw, outb, outw, pic::clear_irq},
    blk, dma,
    mm::{PhysAddr, VirtAddr},
    pci::{self, PCIDevice},
    scheduler::block_current_thread,
};

bitflags::bitflags! {
    pub struct ATAStatus: u8 {
        const ERROR = 1 << 0;
        const INDEX = 1 << 1;
        const CORRECTED_DATA = 1 << 2;
        const DATA_REQUEST_READY = 1 << 3;
        const DISK_SEEK_COMPLETE = 1 << 4;
        const DISK_FAULT = 1 << 5;
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

// https://docs.microsoft.com/en-us/windows-hardware/diskrs/ddi/ata/ns-ata-_identify_device_data
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
const ST_DISK_SEEK_COMPLETE: u8 = 1 << 4;
const ST_DISK_FAULT: u8 = 1 << 5;
const ST_READY: u8 = 1 << 6;
const ST_BUSY: u8 = 1 << 7;

const SECTOR_SIZE: usize = 512;

pub const ATA_PRIMARY_BUS_PORT: u16 = 0x1F0;
pub const ATA_PRIMARY_BUS_CONTROL_PORT: u16 = 0x3F6;
pub const ATA_SECONDARY_BUS_PORT: u16 = 0x170;
pub const ATA_SECONDARY_BUS_CONTROL_PORT: u16 = 0x376;

pub const ATA_MASTER_DISK: u8 = 0xA0;
pub const ATA_SLAVE_DISK: u8 = 0xB0;

bitflags::bitflags! {
    pub struct ATAProgIf: u8 {
        const PRIMARY_CHANNEL_PCI_NATIVE = 1 << 0;
        const PRIMARY_CHANNEL_SWITCH_MODE = 1 << 1;
        const SECONDARY_CHANNEL_PCI_NATIVE = 1 << 2;
        const SECONDARY_CHANNEL_SWITCH_MODE = 1 << 3;
        const DMA_SUPPORT = 1 << 7;
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct PhysicalRegionDescriptor {
    phys_addr: u32,
    count: u16,
    reserved: u16,
}

#[derive(Debug)]
struct ATABus {
    bus_port: u16,
    control_port: u16,
}

/// Describes an ATA controller, a controller can have 4 disks
#[derive(Debug)]
struct ATAController {
    /// Index in the controller list
    index: usize,

    /// Primary bus
    primary_bus: ATABus,

    /// Secondary bus
    secondary_bus: ATABus,
    //dma_port_base: u16,
    //
    //primary_dma: (PhysAddr, VirtAddr),
    //secondary_dma: (PhysAddr, VirtAddr),
    //
    //primary_prdt: PhysicalRegionDescriptor,
    //secondary_prdt: PhysicalRegionDescriptor,
}

/// Describes an ATA disk
#[derive(Debug)]
struct ATADisk {
    /// Index of the controller the disk is associated with
    controller_idx: usize,

    /// Size of the disk in LBAs
    size: usize,

    /// ATA bus
    primary_bus: bool,

    /// ATA disk
    master_disk: bool,
    // If true the disk on the
    //primary: bool
}

extern "C" {
    fn __ata_interrupt();
}

static ATA_CONTROLLERS: Mutex<Vec<ATAController>> = Mutex::new(Vec::new());

impl blk::BlockDevice for ATADisk {
    fn read(&mut self, req: blk::BlockRequest) -> Result<(), blk::BlockDeviceError> {
        let mut controllers = ATA_CONTROLLERS.lock();
        let controller = &mut controllers[self.controller_idx];

        controller.read(
            self.primary_bus,
            self.master_disk,
            req.lba,
            req.size,
            req.buff,
        );

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

impl ATABus {
    #[inline]
    fn write_io8(&self, reg: u16, val: u8) {
        outb(self.bus_port + reg, val);
    }

    #[inline]
    fn read_io8(&self, reg: u16) -> u8 {
        inb(self.bus_port + reg)
    }

    #[inline]
    fn write_io16(&self, reg: u16, val: u16) {
        outw(self.bus_port + reg, val);
    }

    #[inline]
    fn read_io16(&self, reg: u16) -> u16 {
        inw(self.bus_port + reg)
    }

    #[inline]
    fn write_ctrl8(&self, reg: u16, val: u8) {
        outb(self.bus_port + reg, val);
    }

    #[inline]
    fn read_ctrl8(&self, reg: u16) -> u8 {
        inb(self.bus_port + reg)
    }

    fn select_disk(&mut self, master_selected: bool) {
        self.write_io8(
            REG_DRIVE,
            if master_selected {
                ATA_MASTER_DISK
            } else {
                ATA_SLAVE_DISK
            },
        );
    }

    fn write_lba48(&self, master_disk: bool, lba: usize, count: usize) {
        let disk_val = if master_disk { 0x40 } else { 0x50 };
        self.write_io8(REG_DRIVE, disk_val);

        let count_high = (count >> 8) as u8;
        self.write_io8(REG_SECCOUNT0, count_high);

        let lba4 = (lba >> 24) as u8;
        self.write_io8(REG_LBA0, lba4);

        let lba5 = (lba >> 32) as u8;
        self.write_io8(REG_LBA1, lba5);

        let lba6 = (lba >> 40) as u8;
        self.write_io8(REG_LBA2, lba6);

        let count_low = count as u8;
        self.write_io8(REG_SECCOUNT0, count_low);

        let lba1 = lba as u8;
        self.write_io8(REG_LBA0, lba1);

        let lba2 = (lba >> 8) as u8;
        self.write_io8(REG_LBA1, lba2);

        let lba3 = (lba >> 16) as u8;
        self.write_io8(REG_LBA2, lba3);
    }

    fn write_lba28(&self, master_disk: bool, lba: usize, count: usize) {
        let highest_4bits: u8 = ((lba >> 24) & 0b00001111) as u8;
        let disk_val = if master_disk { 0xE0 } else { 0xF0 } | highest_4bits;

        self.write_io8(REG_DRIVE, disk_val);
        self.write_io8(REG_ERROR, 0);
        self.write_io8(REG_SECCOUNT0, count as u8);
        self.write_io8(REG_LBA0, lba as u8);
        self.write_io8(REG_LBA1, (lba >> 8) as u8);
        self.write_io8(REG_LBA2, (lba >> 16) as u8);
    }

    fn write_lba(&self, master_disk: bool, is_lba48: bool, lba: usize, count: usize) {
        if is_lba48 {
            self.write_lba48(master_disk, lba, count);
        } else {
            self.write_lba28(master_disk, lba, count);
        }
    }

    /// Read the status register 15 times then return the last one
    fn wait_400ns(&self) -> u8 {
        for _ in 0..14 {
            self.read_io8(REG_STATUS);
        }

        self.read_io8(REG_STATUS)
    }

    fn wait_until_not_busy(&self) {
        loop {
            let status = self.wait_400ns();
            if status & ST_BUSY == 0 {
                return;
            }
        }
    }

    fn read(&mut self, master_disk: bool, lba: usize, count: usize, buff: *mut u8) {
        assert!(count < 256);
        self.select_disk(master_disk);
        self.wait_until_not_busy();

        let sector_count = if count == u16::MAX as usize { 0 } else { count };

        let is_lba48 = lba > 0x0FFFFFFF;
        self.write_lba(master_disk, is_lba48, lba, sector_count);

        self.write_io8(
            REG_COMMAND,
            if is_lba48 {
                CMD_READ_PIO_EXT
            } else {
                CMD_READ_PIO
            },
        );

        let out_buff = buff as *mut u16;

        for i in 0..count {
            self.wait_until_not_busy();
            for j in 0..256 {
                unsafe {
                    let idx = i * 256 + j;
                    let ptr = out_buff.offset(idx as isize);
                    let val = self.read_io16(REG_DATA);
                    *ptr = val;
                }
            }

            // status must be read after reading the sector
            self.read_io16(REG_STATUS);
        }
    }

    /// Returns the size of the disk in LBAs if the disk is
    fn try_identify(&mut self, master_disk: bool) -> Option<usize> {
        self.select_disk(master_disk);

        self.write_io8(REG_SECCOUNT0, 0);
        self.write_io8(REG_LBA0, 0);
        self.write_io8(REG_LBA1, 0);
        self.write_io8(REG_LBA2, 0);

        self.write_io8(REG_COMMAND, CMD_IDENTIFY);

        let mut status = self.read_io8(REG_STATUS);
        if status == 0 {
            return None;
        }

        while self.read_io8(REG_STATUS) & ST_BUSY > 0 {
            let lba1 = self.read_io8(REG_LBA1);
            let lba2 = self.read_io8(REG_LBA2);
            if lba1 != 0 || lba2 != 0 {
                // TODO: ATAPI
                return None;
            }
        }

        while (status & ST_DATA_REQUEST_READY) == 0 && (status & ST_ERROR) == 0 {
            status = self.read_io8(REG_STATUS);
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
                let data = self.read_io16(REG_DATA);
                *addr = data;
            }
        }

        let max_lba = unsafe { *((device_data.as_ptr()).offset(ID_MAX_LBA) as *const u32) };

        Some(max_lba as usize)
    }
}

impl ATAController {
    fn read(
        &mut self,
        primary_bus: bool,
        master_disk: bool,
        lba: usize,
        count: usize,
        buff: *mut u8,
    ) {
        let bus = if primary_bus {
            &mut self.primary_bus
        } else {
            &mut self.secondary_bus
        };
        bus.read(master_disk, lba, count, buff);
    }
}

fn init_controllers(devices: Vec<&PCIDevice>) {
    let mut controllers = ATA_CONTROLLERS.lock();

    for pci_device in devices.iter() {
        // TODO: support polling
        if pci_device.prog_if & ATAProgIf::DMA_SUPPORT.bits == 0 {
            println!("ATA: device does not support DMA");
            continue;
        }

        let primary_bus_pci_native =
            pci_device.prog_if & ATAProgIf::PRIMARY_CHANNEL_PCI_NATIVE.bits > 0;
        let secondary_bus_pci_native =
            pci_device.prog_if & ATAProgIf::SECONDARY_CHANNEL_PCI_NATIVE.bits > 0;

        let primary_bus_ports = if primary_bus_pci_native {
            unsafe {
                (
                    (pci_device.specific.type0.bar0 & 0xFFF0) as u16,
                    (pci_device.specific.type0.bar1 & 0xFFF0) as u16,
                )
            }
        } else {
            (ATA_PRIMARY_BUS_PORT, ATA_PRIMARY_BUS_CONTROL_PORT)
        };

        let secondary_bus_ports = if secondary_bus_pci_native {
            unsafe {
                (
                    (pci_device.specific.type0.bar2 & 0xFFF0) as u16,
                    (pci_device.specific.type0.bar3 & 0xFFF0) as u16,
                )
            }
        } else {
            (ATA_SECONDARY_BUS_PORT, ATA_SECONDARY_BUS_CONTROL_PORT)
        };

        let primary_dma = dma::alloc(16 * 4096, 0x10000);
        let secondary_dma = dma::alloc(16 * 4096, 0x10000);

        let mut controller = ATAController {
            index: controllers.len(),
            primary_bus: ATABus {
                bus_port: primary_bus_ports.0,
                control_port: primary_bus_ports.1,
            },
            secondary_bus: ATABus {
                bus_port: secondary_bus_ports.0,
                control_port: primary_bus_ports.1,
            },
        };

        for bus in 0..=1 {
            for disk in 0..=1 {
                let ata_bus = if bus == 0 {
                    &mut controller.primary_bus
                } else {
                    &mut controller.secondary_bus
                };

                if let Some(disk_size) = ata_bus.try_identify(disk == 0) {
                    let bus_str = match bus {
                        0 => "primary",
                        _ => "secondary",
                    };

                    let disk_str = match disk {
                        0 => "master",
                        _ => "slave",
                    };

                    let identified_disk = ATADisk {
                        controller_idx: controller.index,
                        primary_bus: bus == 0,
                        master_disk: disk == 0,
                        size: disk_size,
                    };

                    println!(
                        "ATA: found device on the {} bus/{} disk with LBA count: {}",
                        bus_str, disk_str, identified_disk.size
                    );
                    blk::register(Box::new(identified_disk));
                }
            }
        }

        controllers.push(controller);
    }
}

pub fn init() -> bool {
    pci::match_devices(
        pci::class::PCIClass::MassStorageController(
            pci::class::MassStorageController::IDEController,
        ),
        init_controllers,
    );

    true
}

#[no_mangle]
fn ata_interrupt() {
    println!("ata interrupt");
    loop {}
}
