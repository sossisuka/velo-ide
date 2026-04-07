use std::{fs, path::Path};

use chardetng::EncodingDetector;
use encoding_rs::Encoding;

const FILE_ANALYSIS_BYTES: usize = 64 * 1024;

pub struct DecodedText {
    pub text: String,
    pub encoding: &'static Encoding,
    pub has_bom: bool,
    pub had_errors: bool,
}

#[derive(Debug, PartialEq)]
enum ByteContent {
    Utf16Le,
    Utf16Be,
    Binary,
    Unknown,
}

pub fn decode_text_file(path: &Path) -> std::io::Result<DecodedText> {
    let bytes = fs::read(path)?;
    Ok(decode_bytes(bytes))
}

fn decode_bytes(bytes: Vec<u8>) -> DecodedText {
    let (bom_encoding, bom_len) =
        Encoding::for_bom(&bytes).map_or((None, 0), |(enc, len)| (Some(enc), len));
    if let Some(encoding) = bom_encoding {
        let (text, had_errors) = encoding.decode_with_bom_removal(&bytes);
        return DecodedText {
            text: text.into_owned(),
            encoding,
            has_bom: bom_len > 0,
            had_errors,
        };
    }

    let probe_len = bytes.len().min(FILE_ANALYSIS_BYTES);
    let content_kind = analyze_byte_content(&bytes[..probe_len]);
    match content_kind {
        ByteContent::Utf16Le => {
            let encoding = encoding_rs::UTF_16LE;
            let (text, _, had_errors) = encoding.decode(&bytes);
            return DecodedText {
                text: text.into_owned(),
                encoding,
                has_bom: false,
                had_errors,
            };
        }
        ByteContent::Utf16Be => {
            let encoding = encoding_rs::UTF_16BE;
            let (text, _, had_errors) = encoding.decode(&bytes);
            return DecodedText {
                text: text.into_owned(),
                encoding,
                has_bom: false,
                had_errors,
            };
        }
        ByteContent::Binary => {
            return DecodedText {
                text: "Binary file preview is not supported".to_string(),
                encoding: encoding_rs::UTF_8,
                has_bom: false,
                had_errors: false,
            };
        }
        ByteContent::Unknown => {}
    }

    match String::from_utf8(bytes.clone()) {
        Ok(text) => DecodedText {
            text,
            encoding: encoding_rs::UTF_8,
            has_bom: false,
            had_errors: false,
        },
        Err(_) => {
            let mut detector = EncodingDetector::new();
            detector.feed(&bytes, true);
            let encoding = detector.guess(None, true);
            let (text, _, had_errors) = encoding.decode(&bytes);
            DecodedText {
                text: text.into_owned(),
                encoding,
                has_bom: false,
                had_errors,
            }
        }
    }
}

fn analyze_byte_content(bytes: &[u8]) -> ByteContent {
    if bytes.is_empty() {
        return ByteContent::Unknown;
    }

    if is_known_binary_header(bytes) {
        return ByteContent::Binary;
    }

    if bytes.len() < 2 {
        return ByteContent::Unknown;
    }

    let mut even_null_count = 0usize;
    let mut odd_null_count = 0usize;
    let mut non_text_like_count = 0usize;

    for (i, &byte) in bytes.iter().enumerate() {
        if byte == 0 {
            if i % 2 == 0 {
                even_null_count += 1;
            } else {
                odd_null_count += 1;
            }
            non_text_like_count += 1;
            continue;
        }

        let is_text_like = matches!(
            byte,
            b'\t' | b'\n' | b'\r' | 0x0C | 0x20..=0x7E | 0x80..=0xBF | 0xC2..=0xF4
        );
        if !is_text_like {
            non_text_like_count += 1;
        }
    }

    let total_null_count = even_null_count + odd_null_count;
    if total_null_count == 0 {
        if non_text_like_count * 100 < bytes.len() * 8 {
            return ByteContent::Unknown;
        }
        return ByteContent::Binary;
    }

    let has_significant_nulls = total_null_count >= bytes.len() / 16;
    let nulls_skew_to_even = even_null_count > odd_null_count * 4;
    let nulls_skew_to_odd = odd_null_count > even_null_count * 4;

    if has_significant_nulls {
        if nulls_skew_to_even && is_plausible_utf16_text(bytes, false) {
            return ByteContent::Utf16Be;
        }
        if nulls_skew_to_odd && is_plausible_utf16_text(bytes, true) {
            return ByteContent::Utf16Le;
        }
        return ByteContent::Binary;
    }

    if non_text_like_count * 100 < bytes.len() * 8 {
        ByteContent::Unknown
    } else {
        ByteContent::Binary
    }
}

fn is_known_binary_header(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF-")
        || bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"PK\x05\x06")
        || bytes.starts_with(b"PK\x07\x08")
        || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xFF\xD8\xFF")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || bytes.starts_with(b"RIFF")
        || bytes.starts_with(b"OggS")
        || bytes.starts_with(b"fLaC")
        || bytes.starts_with(b"ID3")
}

fn is_plausible_utf16_text(bytes: &[u8], little_endian: bool) -> bool {
    let mut suspicious_count = 0usize;
    let mut total = 0usize;
    let mut i = 0usize;

    while let Some(code_unit) = read_u16(bytes, i, little_endian) {
        total += 1;
        match code_unit {
            0x0009 | 0x000A | 0x000C | 0x000D => {}
            0x0000..=0x001F | 0x007F..=0x009F | 0xFFFE | 0xFFFF => suspicious_count += 1,
            0xD800..=0xDBFF => {
                let next_offset = i + 2;
                let has_low_surrogate = read_u16(bytes, next_offset, little_endian)
                    .is_some_and(|next| (0xDC00..=0xDFFF).contains(&next));
                if has_low_surrogate {
                    total += 1;
                    i += 2;
                } else {
                    suspicious_count += 1;
                }
            }
            0xDC00..=0xDFFF => suspicious_count += 1,
            _ => {}
        }
        i += 2;
    }

    total > 0 && suspicious_count * 100 < total * 2
}

fn read_u16(bytes: &[u8], offset: usize, little_endian: bool) -> Option<u16> {
    let pair = [*bytes.get(offset)?, *bytes.get(offset + 1)?];
    if little_endian {
        Some(u16::from_le_bytes(pair))
    } else {
        Some(u16::from_be_bytes(pair))
    }
}
