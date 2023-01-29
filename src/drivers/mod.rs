use alloc::vec::Vec;
use spin::Mutex;

#[cfg(ata_module)]
mod ata;

#[cfg(pit_module)]
mod pit;

// FIXME: dont include assembly files associated with disabled modules in the build

/// Kernel module
#[derive(Debug)]
struct KernelModule<'a> {
    /// Returns whether the function got initialized successfully
    init: fn() -> bool,
    name: &'a str,
}

impl<'a> KernelModule<'a> {
    fn new(init: fn() -> bool, name: &str) -> KernelModule {
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

    for module in modules.iter() {
        let success = (module.init)();
        if success {
            println!("DRIVER MANAGER: loaded {} module", module.name);
        } else {
            println!("DRIVER MANAGER: failed to load {} module", module.name);
        }
    }
}
