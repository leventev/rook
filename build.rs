use std::{env, error::Error};

const ASM_OBJ_FILES: &'static [&str] = &["x86_64.o"];

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Get the name of the package.
    let kernel_name = env::var("CARGO_PKG_NAME")?;

    // Tell rustc to pass the linker script to the linker.
    println!("cargo:rustc-link-arg-bin={kernel_name}=--script=conf/linker.ld");
    for obj in ASM_OBJ_FILES {
        println!("cargo:rustc-link-arg-bin={kernel_name}=bin/{obj}");
    }

    // Have cargo rerun this script if the linker script or CARGO_PKG_ENV changes.
    println!("cargo:rerun-if-changed=conf/linker.ld");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_NAME");

    Ok(())
}