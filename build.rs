use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=src/mp4parser/pssh/widevine.proto");

    let icon = env::var("VSD_ICON").is_ok()
        && env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows"
        && env::var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc";

    if icon {
        println!("cargo:rerun-if-changed=resources.rc");
        println!("cargo:rerun-if-changed=images/icon.ico");
    }

    prost_build::compile_protos(
        &["src/mp4parser/pssh/widevine.proto"],
        &["src/mp4parser/pssh/"],
    )
    .unwrap();

    if icon {
        Command::new("rc")
            .arg("/fo")
            .arg(&format!("{}/resources.lib", env::var("OUT_DIR").unwrap()))
            .arg("resources.rc")
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        println!("cargo:rustc-link-search={}", env::var("OUT_DIR").unwrap());
        println!("cargo:rustc-link-lib=resources");
    }
}
