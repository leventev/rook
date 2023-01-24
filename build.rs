use std::{env, error::Error, fs, io::BufRead, path::Path, process::Command};

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

fn parse_kernel_config() -> Vec<String> {
    const KERNEL_CONFIG_FILE_NAME: &str = "kernel.cfg";
    let config_file: Vec<Vec<String>> = fs::read(KERNEL_CONFIG_FILE_NAME)
        .expect("Failed to read kernel config file")
        .lines()
        .map(|line| line.unwrap().split("=").map(|s| String::from(s)).collect())
        .collect();

    let mut options = Vec::new();
    for (i, l) in config_file.iter().enumerate() {
        if l.len() != 2 {
            println!("{}:{}: invalid entry", KERNEL_CONFIG_FILE_NAME, i + 1);
            continue;
        }

        match l[1].as_str() {
            "yes" | "y" => {
                options.push(l[0].clone());
                println!("CONFIG: {} enabled", l[0]);
            }
            "no" | "n" => {
                println!("CONFIG: {} disabled", l[0]);
            }
            _ => {
                println!("{}:{}: invalid entry", KERNEL_CONFIG_FILE_NAME, i + 1);
                continue;
            }
        }
    }

    options
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut asm_source_files: Vec<String> = Vec::new();
    let mut asm_obj_files: Vec<String> = Vec::new();
    
    find_asm_files(&mut asm_source_files, String::from("src"));
    build_asm_files(&asm_source_files, &mut asm_obj_files);

    let kernel_config = parse_kernel_config();
    for flag in kernel_config {
        println!("cargo:rustc-cfg={}", flag);
    }

    let kernel_name = env::var("CARGO_PKG_NAME")?;

    for asm_file in asm_source_files {
        println!("cargo:rerun-if-changed={asm_file}");
    }
    println!("cargo:rustc-link-arg-bin={kernel_name}=--script=conf/linker.ld");
    for obj in asm_obj_files {
        println!("cargo:rustc-link-arg-bin={kernel_name}={obj}");
    }

    println!("cargo:rerun-if-changed=conf/linker.ld");
    println!("cargo:rerun-if-changed=kernel.cfg");

    println!("cargo:rerun-if-env-changed=CARGO_PKG_NAME");

    Ok(())
}
