use self::class::*;
use crate::arch::x86_64::*;
use alloc::{fmt, vec::Vec};
use spin::Mutex;

pub mod class;

#[derive(Clone, Copy, Debug)]
pub struct PCIDeviceType0 {
    pub bar0: u32,
    pub bar1: u32,
    pub bar2: u32,
    pub bar3: u32,
    pub bar4: u32,
    pub bar5: u32,
    pub cardbus_cis_pointer: u32,
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,
    pub expansion_rom_base_address: u32,
    pub capabilities_pointer: u8,
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub min_grant: u8,
    pub max_latency: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct PCIDeviceType1 {
    pub bar0: u32,
    pub bar1: u32,
    pub primary_bus_number: u8,
    pub secondary_bus_number: u8,
    pub subordinate_bus_number: u8,
    pub secondary_latency_timer: u8,
    pub io_base: u8,
    pub io_limit: u8,
    pub secondary_status: u16,
    pub memory_base: u16,
    pub memory_limit: u16,
    pub prefetchable_memory_base: u16,
    pub prefetchable_memory_limit: u16,
    pub prefetchable_base_upper: u32,
    pub prefetchable_limit_upper: u32,
    pub io_base_upper: u16,
    pub io_limit_upper: u16,
    pub capability_pointer: u8,
    pub expansion_rom_base_address: u32,
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub bridge_control: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct PCIDeviceType2 {
    pub cardbus_socket_base_address: u32,
    pub capabilites_list_off: u8,
    pub secondary_status: u16,
    pub pci_bus_number: u8,
    pub cardbus_bus_number: u8,
    pub subordinate_bus_number: u8,
    pub cardbus_latency_timer: u8,
    pub memory_base_address_0: u32,
    pub memory_limit_0: u32,
    pub memory_base_address_1: u32,
    pub memory_limit_1: u32,
    pub io_base_address_0: u32,
    pub io_limit_0: u32,
    pub io_base_address_1: u32,
    pub io_limit_1: u32,
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub bridge_control: u16,
    pub subsystem_device_id: u16,
    pub subsystem_vendor_id: u16,
    pub pc_card_legacy_mode_base_address: u32,
}

#[repr(C)]
pub union PCIDeviceExtended {
    pub type0: PCIDeviceType0,
    pub type1: PCIDeviceType1,
    pub type2: PCIDeviceType2,
}

fn class_from_u8(classcode: u8, subclass: u8) -> PCIClass {
    match classcode {
        0x00 => PCIClass::Unclassified(Unclassified::from_subclass(subclass)),
        0x01 => PCIClass::MassStorageController(MassStorageController::from_subclass(subclass)),
        0x02 => PCIClass::NetworkController(NetworkController::from_subclass(subclass)),
        0x03 => PCIClass::DisplayController(DisplayController::from_subclass(subclass)),
        0x04 => PCIClass::MultimediaController(MultimediaController::from_subclass(subclass)),
        0x05 => PCIClass::MemoryController(MemoryController::from_subclass(subclass)),
        0x06 => PCIClass::Bridge(Bridge::from_subclass(subclass)),
        0x07 => PCIClass::SimpleCommunicationController(
            SimpleCommunicationController::from_subclass(subclass),
        ),
        0x08 => PCIClass::BaseSystemPeripheral(BaseSystemPeripheral::from_subclass(subclass)),
        0x09 => PCIClass::InputDeviceController(InputDeviceController::from_subclass(subclass)),
        0x0A => PCIClass::DockingStation(DockingStation::from_subclass(subclass)),
        0x0B => PCIClass::Processor(Processor::from_subclass(subclass)),
        0x0C => PCIClass::SerialBusController(SerialBusController::from_subclass(subclass)),
        0x0D => PCIClass::WirelessController(WirelessController::from_subclass(subclass)),
        0x0E => PCIClass::IntelligentController(IntelligentController::from_subclass(subclass)),
        0x0F => PCIClass::SatelliteCommunicationController(
            SatelliteCommunicationController::from_subclass(subclass),
        ),
        0x10 => PCIClass::EncryptionController(EncryptionController::from_subclass(subclass)),
        0x11 => PCIClass::SignalProcessingController(SignalProcessingController::from_subclass(
            subclass,
        )),
        0x12 => PCIClass::ProcessingAccelerator(ProcessingAccelerator::from_subclass(subclass)),
        0x13 => PCIClass::NonEssentialInstrumentation(NonEssentialInstrumentation::from_subclass(
            subclass,
        )),
        0x40 => PCIClass::CoProcessor(CoProcessor::from_subclass(subclass)),
        _ => unreachable!(),
    }
}

pub struct PCIDevice {
    pub bus: u8,
    pub dev: u8,
    pub function: u8,

    pub vendor_id: u16,
    pub device_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub prog_if: u8,
    pub class: PCIClass,
    pub cache_line_size: u8,
    pub latency_timer: u8,
    pub header_type: u8,
    pub bist: u8,

    pub specific: PCIDeviceExtended,
}

impl fmt::Display for PCIDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bus: {} ", self.bus).unwrap();
        write!(f, "dev: {} ", self.dev).unwrap();
        write!(f, "function: {} ", self.function).unwrap();
        write!(f, "vendor_id: {} ", self.vendor_id).unwrap();
        write!(f, "device_id: {} ", self.device_id).unwrap();
        write!(f, "command: {} ", self.command).unwrap();
        write!(f, "status: {} ", self.status).unwrap();
        write!(f, "revision_id: {} ", self.revision_id).unwrap();
        write!(f, "prog_if: {} ", self.prog_if).unwrap();
        write!(f, "class: {:?} ", self.class).unwrap();
        write!(f, "cache_line_size: {} ", self.cache_line_size).unwrap();
        write!(f, "latency_timer: {} ", self.latency_timer).unwrap();
        write!(f, "header_type: {} ", self.header_type).unwrap();
        write!(f, "bist: {} ", self.bist).unwrap();

        match self.header_type {
            0x0 => unsafe { write!(f, "{:?}", self.specific.type0) },
            0x1 => unsafe { write!(f, "{:?}", self.specific.type1) },
            0x2 => unsafe { write!(f, "{:?}", self.specific.type2) },
            _ => unreachable!(),
        }
    }
}

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

// common header
pub const VENDOR_ID_OFF: u8 = 0x0;
pub const DEVICE_ID_OFF: u8 = 0x2;
pub const DEVICE_COMMAND_OFF: u8 = 0x4;
pub const DEVICE_STATUS_OFF: u8 = 0x6;
pub const DEVICE_REVISION_ID_OFF: u8 = 0x8;
pub const DEVICE_PROG_IF_OFF: u8 = 0x9;
pub const DEVICE_SUBCLASS_OFF: u8 = 0xA;
pub const DEVICE_CLASS_CODE_OFF: u8 = 0xB;
pub const DEVICE_CACHE_LINE_SIZE_OFF: u8 = 0xC;
pub const DEVICE_LATENCY_TIMER_OFF: u8 = 0xD;
pub const DEVICE_HEADER_TYPE_OFF: u8 = 0xE;
pub const DEVICE_BIST_OFF: u8 = 0xF;

// header type 0
pub const DEVICE_TYPE0_BAR0_OFF: u8 = 0x10;
pub const DEVICE_TYPE0_BAR1_OFF: u8 = 0x14;
pub const DEVICE_TYPE0_BAR2_OFF: u8 = 0x18;
pub const DEVICE_TYPE0_BAR3_OFF: u8 = 0x1c;
pub const DEVICE_TYPE0_BAR4_OFF: u8 = 0x20;
pub const DEVICE_TYPE0_BAR5_OFF: u8 = 0x24;
pub const DEVICE_TYPE0_CARDBUS_POINTER_OFF: u8 = 0x28;
pub const DEVICE_TYPE0_SUBSYSTEM_VENDOR_ID_OFF: u8 = 0x2C;
pub const DEVICE_TYPE0_SUBSYSTEM_ID_OFF: u8 = 0x2E;
pub const DEVICE_TYPE0_EXPANSION_ROM_BASE_ADDRESS_OFF: u8 = 0x30;
pub const DEVICE_TYPE0_CAPABILITIES_POINTER_OFF: u8 = 0x34;
pub const DEVICE_TYPE0_INTERRUPT_LINE_OFF: u8 = 0x3C;
pub const DEVICE_TYPE0_INTERRUPT_PIN_OFF: u8 = 0x3D;
pub const DEVICE_TYPE0_MIN_GRANT_OFF: u8 = 0x3E;
pub const DEVICE_TYPE0_MAX_LATENCY_OFF: u8 = 0x3F;

// header type 1
pub const DEVICE_TYPE1_BAR0_OFF: u8 = 0x10;
pub const DEVICE_TYPE1_BAR1_OFF: u8 = 0x14;
pub const DEVICE_TYPE1_PRIMARY_BUS_NUMBER_OFF: u8 = 0x18;
pub const DEVICE_TYPE1_SECONDARY_BUS_NUMBER_OFF: u8 = 0x19;
pub const DEVICE_TYPE1_SUBORDINATE_BUS_NUMBER_OFF: u8 = 0x1A;
pub const DEVICE_TYPE1_SECONDARY_LATENCY_TIMER_OFF: u8 = 0x1B;
pub const DEVICE_TYPE1_IO_BASE_OFF: u8 = 0x1C;
pub const DEVICE_TYPE1_IO_LIMIT_OFF: u8 = 0x1D;
pub const DEVICE_TYPE1_SECONDARY_STATUS_OFF: u8 = 0x1E;
pub const DEVICE_TYPE1_MEMORY_BASE_OFF: u8 = 0x20;
pub const DEVICE_TYPE1_MEMORY_LIMIT_OFF: u8 = 0x22;
pub const DEVICE_TYPE1_PREFETCHABLE_MEMORY_BASE_OFF: u8 = 0x24;
pub const DEVICE_TYPE1_PREFETCHABLE_MEMORY_LIMIT_OFF: u8 = 0x26;
pub const DEVICE_TYPE1_PREFETCHABLE_MEMORY_UPPER_BASE_OFF: u8 = 0x28;
pub const DEVICE_TYPE1_PREFETCHABLE_MEMORY_UPPER_LIMIT_OFF: u8 = 0x2C;
pub const DEVICE_TYPE1_IO_UPPER_BASE_OFF: u8 = 0x30;
pub const DEVICE_TYPE1_IO_UPPER_LIMIT_OFF: u8 = 0x32;
pub const DEVICE_TYPE1_CAPABILITY_POINTER_OFF: u8 = 0x34;
pub const DEVICE_TYPE1_EXPANSION_ROM_BASE_ADDRESS_OFF: u8 = 0x38;
pub const DEVICE_TYPE1_INTERRUPT_LINE_OFF: u8 = 0x3C;
pub const DEVICE_TYPE1_INTERRUPT_PIN_OFF: u8 = 0x3D;
pub const DEVICE_TYPE1_BRIDGE_CONTROL_OFF: u8 = 0x3E;

// header type 2
pub const DEVICE_TYPE2_CARDBUS_SOCKET_BASE_ADDRESS_OFF: u8 = 0x10;
pub const DEVICE_TYPE2_CAPABILITIES_LIST_OFFSET_OFF: u8 = 0x14;
pub const DEVICE_TYPE2_SECONDARY_STATUS_OFF: u8 = 0x16;
pub const DEVICE_TYPE2_PCI_BUS_NUMBER_OFF: u8 = 0x18;
pub const DEVICE_TYPE2_CARDBUS_BUS_NUMBER_OFF: u8 = 0x19;
pub const DEVICE_TYPE2_SUBORDINATE_BUS_NUMBER_OFF: u8 = 0x1A;
pub const DEVICE_TYPE2_CARDBUS_LATENCY_TIMER_OFF: u8 = 0x1B;
pub const DEVICE_TYPE2_MEMORY_BASE_0_OFF: u8 = 0x1C;
pub const DEVICE_TYPE2_MEMORY_LIMIT_0_OFF: u8 = 0x20;
pub const DEVICE_TYPE2_MEMORY_BASE_1_OFF: u8 = 0x24;
pub const DEVICE_TYPE2_MEMORY_LIMIT_1_OFF: u8 = 0x28;
pub const DEVICE_TYPE2_IO_BASE_0_OFF: u8 = 0x2C;
pub const DEVICE_TYPE2_IO_LIMIT_0_OFF: u8 = 0x30;
pub const DEVICE_TYPE2_IO_BASE_1_OFF: u8 = 0x34;
pub const DEVICE_TYPE2_IO_LIMIT_1_OFF: u8 = 0x38;
pub const DEVICE_TYPE2_INTERRUPT_LINE_OFF: u8 = 0x3C;
pub const DEVICE_TYPE2_INTERRUPT_PIN_OFF: u8 = 0x3D;
pub const DEVICE_TYPE2_BRIDGE_CONTROL_OFF: u8 = 0x3E;
pub const DEVICE_TYPE2_SUBSYSTEM_DEVICE_ID_OFF: u8 = 0x40;
pub const DEVICE_TYPE2_SUBSYSTEM_VENDOR_ID_OFF: u8 = 0x42;
pub const DEVICE_TYPE2_PC_CARD_LEGACY_MODE_BASE_ADDRESS_OFF: u8 = 0x44;

static PCI_DEVICES: Mutex<Vec<PCIDevice>> = Mutex::new(Vec::new());

const MAX_DEVICE: u8 = 32;
const MAX_FUNCTION: u8 = 8;

fn construct_addr(bus: u8, dev: u8, function: u8) -> u32 {
    assert!(dev < MAX_DEVICE);
    assert!(function < MAX_FUNCTION);
    (1 << 31) | ((bus as u32) << 16) | ((dev as u32) << 11) | ((function as u32) << 8)
}

#[inline]
fn write_config_addr(addr: u32, off: u8) {
    outl(CONFIG_ADDRESS, addr | (off & 0b11111100) as u32);
}

fn read8(addr: u32, off: u8) -> u8 {
    write_config_addr(addr, off);
    inb(CONFIG_DATA + (off & 0b11) as u16)
}

fn read16(addr: u32, off: u8) -> u16 {
    write_config_addr(addr, off);
    inw(CONFIG_DATA + (off & 0b10) as u16)
}

fn read32(addr: u32, off: u8) -> u32 {
    write_config_addr(addr, off);
    inl(CONFIG_DATA)
}

fn write8(addr: u32, off: u8, val: u8) {
    write_config_addr(addr, off);
    outb(CONFIG_DATA + (off & 0b11) as u16, val);
}

fn write16(addr: u32, off: u8, val: u16) {
    write_config_addr(addr, off);
    outw(CONFIG_DATA + (off & 0b10) as u16, val);
}

fn write32(addr: u32, off: u8, val: u32) {
    write_config_addr(addr, off);
    outl(CONFIG_DATA, val);
}

fn read_header_type0(base_addr: u32) -> PCIDeviceType0 {
    PCIDeviceType0 {
        bar0: read32(base_addr, DEVICE_TYPE0_BAR0_OFF),
        bar1: read32(base_addr, DEVICE_TYPE0_BAR1_OFF),
        bar2: read32(base_addr, DEVICE_TYPE0_BAR2_OFF),
        bar3: read32(base_addr, DEVICE_TYPE0_BAR3_OFF),
        bar4: read32(base_addr, DEVICE_TYPE0_BAR4_OFF),
        bar5: read32(base_addr, DEVICE_TYPE0_BAR5_OFF),
        cardbus_cis_pointer: read32(base_addr, DEVICE_TYPE0_CARDBUS_POINTER_OFF),
        subsystem_vendor_id: read16(base_addr, DEVICE_TYPE0_SUBSYSTEM_VENDOR_ID_OFF),
        subsystem_id: read16(base_addr, DEVICE_TYPE0_SUBSYSTEM_ID_OFF),
        expansion_rom_base_address: read32(base_addr, DEVICE_TYPE0_EXPANSION_ROM_BASE_ADDRESS_OFF),
        capabilities_pointer: read8(base_addr, DEVICE_TYPE0_CAPABILITIES_POINTER_OFF),
        interrupt_line: read8(base_addr, DEVICE_TYPE0_INTERRUPT_LINE_OFF),
        interrupt_pin: read8(base_addr, DEVICE_TYPE0_INTERRUPT_PIN_OFF),
        min_grant: read8(base_addr, DEVICE_TYPE0_MIN_GRANT_OFF),
        max_latency: read8(base_addr, DEVICE_TYPE0_MAX_LATENCY_OFF),
    }
}

fn read_header_type1(base_addr: u32) -> PCIDeviceType1 {
    PCIDeviceType1 {
        bar0: read32(base_addr, DEVICE_TYPE1_BAR0_OFF),
        bar1: read32(base_addr, DEVICE_TYPE1_BAR1_OFF),
        primary_bus_number: read8(base_addr, DEVICE_TYPE1_PRIMARY_BUS_NUMBER_OFF),
        secondary_bus_number: read8(base_addr, DEVICE_TYPE1_SECONDARY_BUS_NUMBER_OFF),
        subordinate_bus_number: read8(base_addr, DEVICE_TYPE1_SUBORDINATE_BUS_NUMBER_OFF),
        secondary_latency_timer: read8(base_addr, DEVICE_TYPE1_SECONDARY_LATENCY_TIMER_OFF),
        io_base: read8(base_addr, DEVICE_TYPE1_IO_BASE_OFF),
        io_limit: read8(base_addr, DEVICE_TYPE1_IO_LIMIT_OFF),
        secondary_status: read16(base_addr, DEVICE_TYPE1_SECONDARY_STATUS_OFF),
        memory_base: read16(base_addr, DEVICE_TYPE1_MEMORY_BASE_OFF),
        memory_limit: read16(base_addr, DEVICE_TYPE1_MEMORY_LIMIT_OFF),
        prefetchable_memory_base: read16(base_addr, DEVICE_TYPE1_PREFETCHABLE_MEMORY_BASE_OFF),
        prefetchable_memory_limit: read16(base_addr, DEVICE_TYPE1_PREFETCHABLE_MEMORY_LIMIT_OFF),
        prefetchable_base_upper: read32(base_addr, DEVICE_TYPE1_PREFETCHABLE_MEMORY_UPPER_BASE_OFF),
        prefetchable_limit_upper: read32(
            base_addr,
            DEVICE_TYPE1_PREFETCHABLE_MEMORY_UPPER_LIMIT_OFF,
        ),
        io_base_upper: read16(base_addr, DEVICE_TYPE1_IO_UPPER_BASE_OFF),
        io_limit_upper: read16(base_addr, DEVICE_TYPE1_IO_UPPER_LIMIT_OFF),
        capability_pointer: read8(base_addr, DEVICE_TYPE1_CAPABILITY_POINTER_OFF),
        expansion_rom_base_address: read32(base_addr, DEVICE_TYPE1_EXPANSION_ROM_BASE_ADDRESS_OFF),
        interrupt_line: read8(base_addr, DEVICE_TYPE1_INTERRUPT_LINE_OFF),
        interrupt_pin: read8(base_addr, DEVICE_TYPE1_INTERRUPT_PIN_OFF),
        bridge_control: read16(base_addr, DEVICE_TYPE1_BRIDGE_CONTROL_OFF),
    }
}

fn read_header_type2(base_addr: u32) -> PCIDeviceType2 {
    PCIDeviceType2 {
        cardbus_socket_base_address: read32(
            base_addr,
            DEVICE_TYPE2_CARDBUS_SOCKET_BASE_ADDRESS_OFF,
        ),
        capabilites_list_off: read8(base_addr, DEVICE_TYPE2_CAPABILITIES_LIST_OFFSET_OFF),
        secondary_status: read16(base_addr, DEVICE_TYPE2_SECONDARY_STATUS_OFF),
        pci_bus_number: read8(base_addr, DEVICE_TYPE2_PCI_BUS_NUMBER_OFF),
        cardbus_bus_number: read8(base_addr, DEVICE_TYPE2_CARDBUS_BUS_NUMBER_OFF),
        subordinate_bus_number: read8(base_addr, DEVICE_TYPE2_SUBORDINATE_BUS_NUMBER_OFF),
        cardbus_latency_timer: read8(base_addr, DEVICE_TYPE2_CARDBUS_LATENCY_TIMER_OFF),
        memory_base_address_0: read32(base_addr, DEVICE_TYPE2_MEMORY_BASE_0_OFF),
        memory_limit_0: read32(base_addr, DEVICE_TYPE2_MEMORY_LIMIT_0_OFF),
        memory_base_address_1: read32(base_addr, DEVICE_TYPE2_MEMORY_BASE_1_OFF),
        memory_limit_1: read32(base_addr, DEVICE_TYPE2_MEMORY_LIMIT_1_OFF),
        io_base_address_0: read32(base_addr, DEVICE_TYPE2_IO_BASE_0_OFF),
        io_limit_0: read32(base_addr, DEVICE_TYPE2_IO_LIMIT_0_OFF),
        io_base_address_1: read32(base_addr, DEVICE_TYPE2_IO_BASE_1_OFF),
        io_limit_1: read32(base_addr, DEVICE_TYPE2_IO_LIMIT_1_OFF),
        interrupt_line: read8(base_addr, DEVICE_TYPE2_INTERRUPT_LINE_OFF),
        interrupt_pin: read8(base_addr, DEVICE_TYPE2_INTERRUPT_PIN_OFF),
        bridge_control: read16(base_addr, DEVICE_TYPE2_BRIDGE_CONTROL_OFF),
        subsystem_device_id: read16(base_addr, DEVICE_TYPE2_SUBSYSTEM_DEVICE_ID_OFF),
        subsystem_vendor_id: read16(base_addr, DEVICE_TYPE2_SUBSYSTEM_VENDOR_ID_OFF),
        pc_card_legacy_mode_base_address: read32(
            base_addr,
            DEVICE_TYPE2_PC_CARD_LEGACY_MODE_BASE_ADDRESS_OFF,
        ),
    }
}

fn read_function(devices: &mut Vec<PCIDevice>, bus: u8, dev: u8, func: u8) {
    let base_addr = construct_addr(bus, dev, func);

    let vendor_id = read16(base_addr, VENDOR_ID_OFF);
    if vendor_id == 0xFFFF {
        return;
    }

    let header_type = read8(base_addr, DEVICE_HEADER_TYPE_OFF) & 0b11;

    let classcode = read8(base_addr, DEVICE_CLASS_CODE_OFF);
    let subclass = read8(base_addr, DEVICE_SUBCLASS_OFF);

    let device = PCIDevice {
        bus,
        dev,
        function: func,
        vendor_id,
        device_id: read16(base_addr, DEVICE_ID_OFF),
        command: read16(base_addr, DEVICE_COMMAND_OFF),
        status: read16(base_addr, DEVICE_STATUS_OFF),
        revision_id: read8(base_addr, DEVICE_REVISION_ID_OFF),
        prog_if: read8(base_addr, DEVICE_PROG_IF_OFF),
        class: class_from_u8(classcode, subclass),
        cache_line_size: read8(base_addr, DEVICE_CACHE_LINE_SIZE_OFF),
        latency_timer: read8(base_addr, DEVICE_LATENCY_TIMER_OFF),
        header_type,
        bist: read8(base_addr, DEVICE_BIST_OFF),
        specific: match header_type {
            0x0 => PCIDeviceExtended {
                type0: read_header_type0(base_addr),
            },
            0x1 => PCIDeviceExtended {
                type1: read_header_type1(base_addr),
            },
            0x2 => PCIDeviceExtended {
                type2: read_header_type2(base_addr),
            },
            _ => unreachable!(),
        },
    };

    if let PCIClass::Bridge(ref bridge_type) = device.class {
        if *bridge_type == Bridge::PCIToPCIBridge {
            let secondary_bus = unsafe { device.specific.type1.secondary_bus_number };
            read_bus(devices, secondary_bus);
        }
    }

    devices.push(device);
}

fn read_device(devices: &mut Vec<PCIDevice>, bus: u8, dev: u8) {
    let base_addr = construct_addr(bus, dev, 0);

    let vendor_id = read16(base_addr, VENDOR_ID_OFF);
    if vendor_id == 0xFFFF {
        return;
    }

    let header_type = read8(base_addr, DEVICE_HEADER_TYPE_OFF);
    if header_type & (1 << 7) > 0 {
        for func in 0..8 {
            read_function(devices, bus, dev, func);
        }
    } else {
        read_function(devices, bus, dev, 0);
    }
}

fn read_bus(devices: &mut Vec<PCIDevice>, bus: u8) {
    for dev in 0..32 {
        read_device(devices, bus, dev);
    }
}

// TODO: avoid cloning
pub fn match_devices(class: PCIClass, func: fn(Vec<&PCIDevice>)) {
    let devices = PCI_DEVICES.lock();
    let mut matched: Vec<&PCIDevice> = Vec::new();
    for dev in devices.iter() {
        if dev.class == class {
            matched.push(dev);
        }
    }
    func(matched);
}

pub fn init() {
    let mut devices = PCI_DEVICES.lock();
    devices.clear();

    let bus0_base_addr = construct_addr(0, 0, 0);
    let header_type = read8(bus0_base_addr, DEVICE_HEADER_TYPE_OFF);

    if header_type & (1 << 7) == 0 {
        read_bus(&mut devices, 0);
    } else {
        for func in 0..8 {
            let base_addr = construct_addr(0, 0, func);
            let vendor_id = read32(base_addr, VENDOR_ID_OFF);
            if vendor_id == 0xFFF { break; }
            read_bus(&mut devices, func);
        }
    }
}

pub fn write_config8(bus: u8, dev: u8, func: u8, reg: u8, val: u8) {
    let base_addr = construct_addr(bus, dev, func);
    write8(base_addr, reg, val);
}

pub fn write_config16(bus: u8, dev: u8, func: u8, reg: u8, val: u16) {
    let base_addr = construct_addr(bus, dev, func);
    write16(base_addr, reg, val);
}

pub fn write_config32(bus: u8, dev: u8, func: u8, reg: u8, val: u32) {
    let base_addr = construct_addr(bus, dev, func);
    write32(base_addr, reg, val);
}
