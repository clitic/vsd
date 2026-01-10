const PNG_HEADER: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub fn fake_png_header(data: &[u8]) -> &[u8] {
    if data.len() >= 8 && data[0..8] == PNG_HEADER {
        &data[8..]
    } else {
        data
    }
}
