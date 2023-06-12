fn main() {
    println!("cargo:rerun-if-changed=src/pssh/widevine.proto");

    prost_build::compile_protos(
        &["src/pssh/widevine.proto"],
        &["src/pssh/"],
    )
    .unwrap();
}
