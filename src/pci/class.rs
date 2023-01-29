use core::fmt::Debug;

/*pub trait PCIClass: Debug + Send {
    pub fn from_subclass(subclass: u8) -> Self
    where
        Self: Sized;
}

impl Debug, PartialEq) for dyn PCIClass {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }

    fn ne(&self, other: &Self) -> bool {
        self != other
    }
}*/

#[derive(Debug, PartialEq)]
pub enum Unclassified {
    NonVGACompatibleDevice,
    VGACompatbileDevice,
}

impl Unclassified {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::NonVGACompatibleDevice,
            0x1 => Self::VGACompatbileDevice,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MassStorageController {
    SCSIBusController,
    IDEController,
    FloppyDiskController,
    IPIBusController,
    RAIDController,
    ATAController,
    SerialATAController,
    SerialAttachedSCSIController,
    NonVolatileMemoryContoller,
    Other,
}

impl MassStorageController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::SCSIBusController,
            0x1 => Self::IDEController,
            0x2 => Self::FloppyDiskController,
            0x3 => Self::IPIBusController,
            0x4 => Self::RAIDController,
            0x5 => Self::ATAController,
            0x6 => Self::SerialATAController,
            0x7 => Self::SerialAttachedSCSIController,
            0x8 => Self::NonVolatileMemoryContoller,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum NetworkController {
    EthernetController,
    TokenRingController,
    FDDIController,
    ATMController,
    ISDNController,
    WorldFlipController,
    PCIMG2_14MultiComputingController,
    InfibandController,
    FabricController,
    Other,
}

impl NetworkController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::EthernetController,
            0x1 => Self::TokenRingController,
            0x2 => Self::FDDIController,
            0x3 => Self::ATMController,
            0x4 => Self::ISDNController,
            0x5 => Self::WorldFlipController,
            0x6 => Self::PCIMG2_14MultiComputingController,
            0x7 => Self::InfibandController,
            0x8 => Self::FabricController,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DisplayController {
    VGACompatibleController,
    XGAController,
    _3DController,
    Other,
}

impl DisplayController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::VGACompatibleController,
            0x1 => Self::XGAController,
            0x2 => Self::_3DController,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MultimediaController {
    MultimediaVideo,
    MultimediaAudio,
    ComputerTelephonyDevice,
    AudioDevice,
    Other,
}

impl MultimediaController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::MultimediaVideo,
            0x1 => Self::AudioDevice,
            0x2 => Self::ComputerTelephonyDevice,
            0x3 => Self::AudioDevice,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MemoryController {
    RAMController,
    FlashController,
    Other,
}

impl MemoryController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::RAMController,
            0x1 => Self::FlashController,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Bridge {
    HostBridge,
    ISABridge,
    EISABridge,
    MCABridge,
    PCIToPCIBridge,
    PCMCIABridge,
    NuBusBridge,
    CardBusBridge,
    RACEwayBridge,
    PCItoPCIBridge2,
    InfiniBandToPCIHostBridge,
    Other,
}

impl Bridge {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::HostBridge,
            0x1 => Self::ISABridge,
            0x2 => Self::EISABridge,
            0x3 => Self::MCABridge,
            0x4 => Self::PCIToPCIBridge,
            0x5 => Self::PCMCIABridge,
            0x6 => Self::NuBusBridge,
            0x7 => Self::CardBusBridge,
            0x8 => Self::RACEwayBridge,
            0x9 => Self::PCItoPCIBridge2,
            0xA => Self::InfiniBandToPCIHostBridge,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SimpleCommunicationController {
    SerialController,
    ParallelController,
    MultiportSerialController,
    Modem,
    IEEE488_1_2Controller,
    SmartCardController,
    Other,
}

impl SimpleCommunicationController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::SerialController,
            0x1 => Self::ParallelController,
            0x2 => Self::MultiportSerialController,
            0x3 => Self::Modem,
            0x4 => Self::IEEE488_1_2Controller,
            0x5 => Self::SmartCardController,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BaseSystemPeripheral {
    PIC,
    DMAController,
    Timer,
    RTCController,
    PCIHotPlugController,
    SDHostController,
    IOMMU,
    Other,
}

impl BaseSystemPeripheral {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::PIC,
            0x1 => Self::DMAController,
            0x2 => Self::Timer,
            0x3 => Self::RTCController,
            0x4 => Self::PCIHotPlugController,
            0x5 => Self::SDHostController,
            0x6 => Self::IOMMU,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum InputDeviceController {
    KeyboardController,
    DigitizerPen,
    MouseController,
    ScannerController,
    GameportController,
    Other,
}

impl InputDeviceController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::KeyboardController,
            0x1 => Self::DigitizerPen,
            0x2 => Self::MouseController,
            0x3 => Self::ScannerController,
            0x4 => Self::GameportController,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DockingStation {
    Generic,
    Other,
}

impl DockingStation {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::Generic,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Processor {
    I386,
    I486,
    Pentium,
    PentiumPro,
    Alpha,
    PowerPC,
    MIPS,
    CoProcessor,
    Other,
}

impl Processor {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::I386,
            0x1 => Self::I486,
            0x2 => Self::Pentium,
            0x3 => Self::PentiumPro,
            0x10 => Self::Alpha,
            0x20 => Self::PowerPC,
            0x30 => Self::MIPS,
            0x40 => Self::CoProcessor,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SerialBusController {
    FireWireController,
    ACCESSBusController,
    SSA,
    USBController,
    FibreChannel,
    SMBusController,
    InfiniBandController,
    IPMIInterface,
    SERCOSInterface,
    CANBusController,
    Other,
}

impl SerialBusController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::FireWireController,
            0x1 => Self::ACCESSBusController,
            0x2 => Self::SSA,
            0x3 => Self::USBController,
            0x4 => Self::FibreChannel,
            0x5 => Self::SMBusController,
            0x6 => Self::InfiniBandController,
            0x7 => Self::IPMIInterface,
            0x8 => Self::SERCOSInterface,
            0x9 => Self::CANBusController,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum WirelessController {
    IRDACompatibleController,
    ConsumerIRController,
    RFController,
    BluetoothController,
    BroadbandController,
    EthernetControllerA,
    EthernetControllerB,
    Other,
}

impl WirelessController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::IRDACompatibleController,
            0x1 => Self::ConsumerIRController,
            0x10 => Self::RFController,
            0x11 => Self::BluetoothController,
            0x12 => Self::BroadbandController,
            0x20 => Self::EthernetControllerA,
            0x21 => Self::EthernetControllerB,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum IntelligentController {
    I20,
}

impl IntelligentController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::I20,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SatelliteCommunicationController {
    SatelliteTVController,
    SatelliteAudioController,
    SatelliteVoiceController,
    SatelliteDataController,
}

impl SatelliteCommunicationController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x1 => Self::SatelliteTVController,
            0x2 => Self::SatelliteAudioController,
            0x3 => Self::SatelliteVoiceController,
            0x4 => Self::SatelliteDataController,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EncryptionController {
    NetworkAndComputingEncryptionDecryption,
    EntertainmentEncryptionDecryption,
    Other,
}

impl EncryptionController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::NetworkAndComputingEncryptionDecryption,
            0x10 => Self::EntertainmentEncryptionDecryption,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum SignalProcessingController {
    DPIOModules,
    PerformanceCounters,
    CommunicationSynchronizer,
    SignalProcessingManagement,
    Other,
}

impl SignalProcessingController {
    pub fn from_subclass(subclass: u8) -> Self {
        match subclass {
            0x0 => Self::DPIOModules,
            0x1 => Self::PerformanceCounters,
            0x10 => Self::CommunicationSynchronizer,
            0x20 => Self::SignalProcessingManagement,
            0x80 => Self::Other,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ProcessingAccelerator {
    Placeholder,
}

impl ProcessingAccelerator {
    pub fn from_subclass(_subclass: u8) -> Self {
        Self::Placeholder
    }
}

#[derive(Debug, PartialEq)]
pub enum NonEssentialInstrumentation {
    Placeholder,
}

impl NonEssentialInstrumentation {
    pub fn from_subclass(_subclass: u8) -> Self {
        Self::Placeholder
    }
}

#[derive(Debug, PartialEq)]
pub enum CoProcessor {
    Placeholder,
}

impl CoProcessor {
    pub fn from_subclass(_subclass: u8) -> Self {
        Self::Placeholder
    }
}

#[derive(Debug, PartialEq)]
pub enum PCIClass {
    Unclassified(Unclassified),
    MassStorageController(MassStorageController),
    NetworkController(NetworkController),
    DisplayController(DisplayController),
    MultimediaController(MultimediaController),
    MemoryController(MemoryController),
    Bridge(Bridge),
    SimpleCommunicationController(SimpleCommunicationController),
    BaseSystemPeripheral(BaseSystemPeripheral),
    InputDeviceController(InputDeviceController),
    DockingStation(DockingStation),
    Processor(Processor),
    SerialBusController(SerialBusController),
    WirelessController(WirelessController),
    IntelligentController(IntelligentController),
    SatelliteCommunicationController(SatelliteCommunicationController),
    EncryptionController(EncryptionController),
    SignalProcessingController(SignalProcessingController),
    ProcessingAccelerator(ProcessingAccelerator),
    NonEssentialInstrumentation(NonEssentialInstrumentation),
    CoProcessor(CoProcessor),
}
