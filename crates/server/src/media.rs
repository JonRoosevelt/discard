pub fn sniff_content_type(data: &[u8]) -> String {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        "image/png".to_string()
    } else if data.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg".to_string()
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif".to_string()
    } else if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "image/webp".to_string()
    } else if data.starts_with(b"ID3") || data.starts_with(&[0xff, 0xfb]) {
        "audio/mpeg".to_string()
    } else if data.starts_with(b"OggS") {
        "audio/ogg".to_string()
    } else if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        "audio/wav".to_string()
    } else if data.len() > 12 && &data[4..8] == b"ftyp" {
        "video/mp4".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

pub fn image_dimensions(data: &[u8]) -> (Option<i32>, Option<i32>) {
    if data.len() >= 24 && data.starts_with(&[0x89, b'P', b'N', b'G']) {
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return (Some(width as i32), Some(height as i32));
    }
    if data.len() >= 10 && (data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) {
        let width = u16::from_le_bytes([data[6], data[7]]);
        let height = u16::from_le_bytes([data[8], data[9]]);
        return (Some(width as i32), Some(height as i32));
    }
    if data.len() >= 4 && data.starts_with(&[0xff, 0xd8]) {
        return jpeg_dimensions(data);
    }
    (None, None)
}

fn jpeg_dimensions(data: &[u8]) -> (Option<i32>, Option<i32>) {
    let mut i = 2usize;
    while i + 9 < data.len() {
        if data[i] != 0xff {
            i += 1;
            continue;
        }
        let marker = data[i + 1];
        i += 2;
        if marker == 0xd8 || marker == 0xd9 {
            continue;
        }
        if i + 1 >= data.len() {
            break;
        }
        let len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
        if matches!(marker, 0xc0..=0xcf) && marker != 0xc4 && marker != 0xc8 && marker != 0xcc {
            if i + 7 < data.len() {
                let height = u16::from_be_bytes([data[i + 3], data[i + 4]]);
                let width = u16::from_be_bytes([data[i + 5], data[i + 6]]);
                return (Some(width as i32), Some(height as i32));
            }
            break;
        }
        if len < 2 {
            break;
        }
        i += len;
    }
    (None, None)
}
