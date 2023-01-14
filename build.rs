use std::{collections::HashMap, env, error::Error, fs, path::Path, process::Command};

fn find_asm_files(files: &mut Vec<String>, path: String) {
    let entries = fs::read_dir(path).unwrap();
    for f in entries {
        let file = f.unwrap();
        let file_type = file.file_type().unwrap();
        let file_name = String::from(file.file_name().to_str().unwrap());
        let file_path = String::from(file.path().to_str().unwrap());

        if file_type.is_dir() {
            find_asm_files(files, file_path);
        } else if file_type.is_file() {
            let extension = Path::new(&file_name).extension().unwrap();
            if extension != "s" && extension != "asm" {
                continue;
            }
            files.push(file_path);
        }
    }
}

fn build_asm_files(src_files: &Vec<String>, obj_files: &mut Vec<String>) {
    for file in src_files {
        let base_name = Path::new(&file).file_stem().unwrap().to_str().unwrap();
        let obj_name = format!("bin/{}.o", base_name);
        obj_files.push(obj_name.clone());
        Command::new("nasm")
            .arg("-felf64")
            .arg("-g")
            .arg(&file)
            .arg("-o")
            .arg(obj_name)
            .output()
            .expect("failed to build asm file");
    }
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut asm_source_files: Vec<String> = Vec::new();
    let mut asm_obj_files: Vec<String> = Vec::new();

    find_asm_files(&mut asm_source_files, String::from("src"));
    build_asm_files(&asm_source_files, &mut asm_obj_files);

    let mut debug_flags: HashMap<&str, bool> = HashMap::new();
    // to enable a flag just simply replace the false with a true
    debug_flags.insert("vmm_debug", false);
    debug_flags.insert("pfa_debug", false);
    debug_flags.insert("kalloc_debug", false);

    for (flag, enabled) in debug_flags {
        if !enabled {
            continue;
        }
        println!("cargo:rustc-cfg={}", flag);
    }

    let kernel_name = env::var("CARGO_PKG_NAME")?;

    println!("cargo:rustc-link-arg-bin={kernel_name}=--script=conf/linker.ld");
    for obj in asm_obj_files {
        println!("cargo:rustc-link-arg-bin={kernel_name}={obj}");
        println!("cargo:rerun-if-changed={obj}");
    }

    // Have cargo rerun this script if the linker script or CARGO_PKG_ENV changes.
    println!("cargo:rerun-if-changed=conf/linker.ld");

    println!("cargo:rerun-if-env-changed=CARGO_PKG_NAME");

    Ok(())
}
