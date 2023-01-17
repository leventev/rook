use alloc::vec::Vec;
use spin::Mutex;

#[cfg(module_ata)]
mod ata;

#[cfg(module_pit)]
mod pit;

// FIXME: dont include assembly files associated with disabled modules in the build

/// Kernel module
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

    #[cfg(module_ata)]
    modules.push(KernelModule::new(ata::init, "ata"));

    #[cfg(module_pit)]
    modules.push(KernelModule::new(pit::init, "pit"));

    for module in modules.iter() {
        let success = (module.init)();
        if success {
            println!("loaded {} module", module.name);
        } else {
            println!("failed to load {} module", module.name);
        }
    }
}
