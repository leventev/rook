use std::{env, error::Error, collections::HashMap};

const ASM_OBJ_FILES: &'static [&str] = &["x86_64.o"];

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut debug_flags: HashMap<&str, bool> = HashMap::new();
    // to enable a flag just simply replace the false with a true
    debug_flags.insert("vmm_debug", false);
    debug_flags.insert("pfa_debug", false);
    debug_flags.insert("kalloc_debug", false);

    for (flag, enabled) in debug_flags {
        if !enabled { continue; }
        println!("cargo:rustc-cfg={}", flag);
    }

    let kernel_name = env::var("CARGO_PKG_NAME")?;

    println!("cargo:rustc-link-arg-bin={kernel_name}=--script=conf/linker.ld");
    for obj in ASM_OBJ_FILES {
        println!("cargo:rustc-link-arg-bin={kernel_name}=bin/{obj}");
    }

    // Have cargo rerun this script if the linker script or CARGO_PKG_ENV changes.
    println!("cargo:rerun-if-changed=conf/linker.ld");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_NAME");

    Ok(())
}
