use std::{collections::HashMap, fs, fs::File, io::Write, path::PathBuf};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut input = fs::read(root.join("examples/sample/init.mp4")).unwrap();
    input.extend(fs::read(root.join("examples/sample/segment_0.m4s")).unwrap());

    let kid_key_pairs = HashMap::from([(
        "eb676abbcb345e96bbcf616630f1a3da".to_owned(),
        "100b6c20940f779a4589152b57d2dacb".to_owned(),
    )]);

    let decrypted_data = mp4decrypt::mp4decrypt(&input, &kid_key_pairs).unwrap();

    File::create("decrypted.mp4")
        .unwrap()
        .write_all(&decrypted_data)
        .unwrap();
}
