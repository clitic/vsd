pub fn _fake_png_header(data: &[u8]) -> Vec<u8> {
    let png_end_marker = b"IEND";
    let media_start_marker = b"ID3";

    let iend_pos = data
        .windows(png_end_marker.len())
        .position(|window| window == png_end_marker);

    match iend_pos {
        Some(iend_index) => {
            let search_slice = &data[iend_index..];

            let id3_relative_pos = search_slice
                .windows(media_start_marker.len())
                .position(|window| window == media_start_marker);

            let start_index = match id3_relative_pos {
                Some(rel_idx) => iend_index + rel_idx,
                None => {
                    println!(
                        "Warning: ID3 tag not found. Attempting to strip standard PNG footer..."
                    );
                    iend_index + 8
                }
            };

            return data[start_index..].to_vec();
        }
        None => {
            eprintln!("Error: Could not find PNG header end (IEND). Is this the right file?");
        }
    }

    data.to_owned()
}
