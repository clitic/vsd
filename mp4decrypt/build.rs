fn main() {
    println!("cargo:rerun-if-changed=src/mp4decrypt.h");
    println!("cargo:rerun-if-changed=src/mp4decrypt.cpp");

    println!("Building Bento4 v{}", bento4_src::version());
    bento4_src::build();

    bento4_src::cc::Build::new()
        .cpp(true)
        .warnings(false)
        .extra_warnings(false)
        .includes(bento4_src::includes())
        .file("src/mp4decrypt.cpp")
        .compile("mp4decrypt");

    // let bindings = bindgen::Builder::default()
    //     .header("src/mp4decrypt.h")
    //     .generate()
    //     .unwrap()
    //     .write_to_file("bindings.rs")
    //     .unwrap();
}
