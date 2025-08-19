use std::{env, path::Path, process};

fn main() {
    println!("cargo:rerun-if-changed=src/mp4decrypt.h");
    println!("cargo:rerun-if-changed=src/mp4decrypt.cpp");

    let target = env::var("TARGET")
        .expect("TARGET env variable not set by cargo?")
        .replace('-', "_")
        .to_uppercase();

    let target_bento4_dir_env = target.clone() + "_BENTO4_DIR";
    let bento4_dir_env = "BENTO4_DIR";
    let target_bento4_vendor_env = target + "_BENTO4_VENDOR";
    let bento4_vendor_env = "BENTO4_VENDOR";

    println!("cargo:rerun-if-env-changed={bento4_dir_env}");
    println!("cargo:rerun-if-env-changed={bento4_dir_env}");
    println!("cargo:rerun-if-env-changed={bento4_vendor_env}");
    println!("cargo:rerun-if-env-changed={bento4_vendor_env}");

    let includes;

    if let (Ok(bento4_dir), Err(_)) = (
        env::var(target_bento4_dir_env).or(env::var(bento4_dir_env)),
        env::var(target_bento4_vendor_env).or(env::var(bento4_vendor_env)),
    ) {
        let bento4_include = Path::new(&bento4_dir).join("include");

        if bento4_include.exists() {
            includes = vec![bento4_include.clone(), bento4_include.join("bento4")];
        } else {
            println!(
                "{} directory doesn't exists.",
                bento4_include.to_string_lossy()
            );
            process::exit(1);
        }

        let bento4_lib = Path::new(&bento4_dir).join("lib");

        if bento4_lib.exists() {
            println!(
                "cargo:rustc-link-search=native={}",
                bento4_lib.to_string_lossy()
            );
            println!("cargo:rustc-link-lib=ap4");
        } else {
            println!(
                "{} directory doesn't exists.",
                bento4_include.to_string_lossy()
            );
            process::exit(1);
        }
    } else {
        println!("Building Bento4 v{}", bento4_src::version());
        bento4_src::build();
        includes = bento4_src::includes();
    }

    println!("Building mp4decrypt wrapper");
    bento4_src::cc::Build::new()
        .cpp(true)
        .warnings(false)
        .extra_warnings(false)
        .includes(includes)
        .file("src/mp4decrypt.cpp")
        .compile("ap4_mp4decrypt");

    // let bindings = bindgen::Builder::default()
    //     .header("src/mp4decrypt.h")
    //     .generate()
    //     .unwrap()
    //     .write_to_file("bindings.rs")
    //     .unwrap();
}
