fn main() {
    prost_build::compile_protos(
        &["src/mp4parser/pssh/widevine.proto"],
        &["src/mp4parser/pssh/"],
    )
    .unwrap();

    // if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows"
    //     && std::env::var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc"
    // {
    //     std::process::Command::new("rc")
    //         .arg("/fo")
    //         .arg(&format!(
    //             "{}/resources.lib",
    //             std::env::var("OUT_DIR").unwrap()
    //         ))
    //         .arg("resources.rc")
    //         .spawn()
    //         .unwrap()
    //         .wait()
    //         .unwrap();
    //     println!(
    //         "cargo:rustc-link-search={}",
    //         std::env::var("OUT_DIR").unwrap()
    //     );
    //     println!("cargo:rustc-link-lib=resources");
    // }
}
