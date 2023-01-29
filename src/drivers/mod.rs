use alloc::vec::Vec;
use spin::Mutex;

#[cfg(ata_module)]
mod ata;

#[cfg(pit_module)]
mod pit;

// TODO: vfs
#[cfg(serial_module)]
pub mod serial;

// FIXME: dont include assembly files associated with disabled modules in the build

/// Kernel module
#[derive(Debug)]
struct KernelModule {
    /// Returns whether the function got initialized successfully
    init: fn() -> bool,
    name: &'static str,
}

impl KernelModule {
    fn new(init: fn() -> bool, name: &'static str) -> KernelModule {
        KernelModule { init, name }
    }
}

static KERNEL_MODULES: Mutex<Vec<KernelModule>> = Mutex::new(Vec::new());

pub fn init() {
    let mut modules = KERNEL_MODULES.lock();

    #[cfg(ata_module)]
    modules.push(KernelModule::new(ata::init, "ata"));

    #[cfg(pit_module)]
    modules.push(KernelModule::new(pit::init, "pit"));

    #[cfg(serial_module)]
    modules.push(KernelModule::new(serial::init, "serial"));

    for module in modules.iter() {
        let success = (module.init)();
        if success {
            println!("DRIVER MANAGER: loaded {} module", module.name);
        } else {
            println!("DRIVER MANAGER: failed to load {} module", module.name);
        }
    }
}

pub fn is_loaded(lookup: &str) -> bool {
    if KERNEL_MODULES.is_locked() {
        return false;
    }

    let modules = KERNEL_MODULES.lock();
    modules
        .iter()
        .find(|driver| driver.name == lookup)
        .is_some()
}
