use alloc::vec::Vec;
use spin::Mutex;

#[cfg(ata_module)]
mod ata;

#[cfg(pit_module)]
mod pit;

// TODO: vfs
#[cfg(serial_module)]
pub mod serial;

#[cfg(fat_module)]
pub mod fat;

// FIXME: dont include assembly files associated with disabled modules in the build

#[derive(Debug)]
pub enum KernelModuleLoadStatus {
    NotLoaded,
    Loaded,
    LoadFailed,
}

/// Kernel module
#[derive(Debug)]
struct KernelModule {
    /// Returns whether the function got initialized successfully
    init: fn() -> bool,
    name: &'static str,
    load_state: KernelModuleLoadStatus,
}

impl KernelModule {
    fn new(init: fn() -> bool, name: &'static str) -> KernelModule {
        KernelModule {
            init,
            name,
            load_state: KernelModuleLoadStatus::NotLoaded,
        }
    }

    fn load(&mut self) {
        let success = (self.init)();
        if success {
            self.load_state = KernelModuleLoadStatus::Loaded;
            if cfg!(driver_manager_debug) {
                println!("DRIVER MANAGER: loaded {} module", self.name);
            }
        } else {
            self.load_state = KernelModuleLoadStatus::LoadFailed;
            if cfg!(driver_manager_debug) {
                println!("DRIVER MANAGER: failed to load {} module", self.name);
            }
        }
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

    #[cfg(fat_module)]
    modules.push(KernelModule::new(fat::init, "fat"));
}

pub fn preload_driver(name: &str) {
    let mut modules = KERNEL_MODULES.lock();
    let pos = modules
        .iter()
        .position(|driver| driver.name == name)
        .unwrap();

    let module = &mut modules[pos];
    module.load();
}

pub fn load_drivers() {
    let mut modules = KERNEL_MODULES.lock();

    for module in modules.iter_mut() {
        match module.load_state {
            KernelModuleLoadStatus::NotLoaded => module.load(),
            _ => continue,
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
