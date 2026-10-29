//! Decoding source files like `tokenize.open` (BOM, coding cookie, strict
//! decoding, universal newlines).

use std::sync::LazyLock;

use regex::bytes::Regex as BytesRegex;
use regex::Regex;

static COOKIE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ \t\x0c]*#.*?coding[:=][ \t]*([-\w.]+)").unwrap());
static BLANK_RE: LazyLock<BytesRegex> =
    LazyLock::new(|| BytesRegex::new(r"^[ \t\x0c]*(?:[#\r\n]|$)").unwrap());

/// Why a file could not be decoded. vulture prints the same message for all of
/// them, so the reason is only informative.
#[derive(Debug)]
pub struct DecodeError(pub String);

/// Decode a Python source file and translate newlines to `\n`.
pub fn decode_source(bytes: &[u8]) -> Result<String, DecodeError> {
    let (encoding, body) = detect_encoding(bytes)?;
    let text = decode(&encoding, body)?;
    Ok(universal_newlines(text))
}

/// `\r\n` and lone `\r` become `\n`.
pub fn universal_newlines(text: String) -> String {
    if !text.contains('\r') {
        return text;
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn first_lines(bytes: &[u8]) -> (&[u8], &[u8]) {
    let split = |b: &'_ [u8]| -> usize { b.iter().position(|&c| c == b'\n').map_or(b.len(), |p| p + 1) };
    let n1 = split(bytes);
    let n2 = split(&bytes[n1..]);
    (&bytes[..n1], &bytes[n1..n1 + n2])
}

/// `tokenize.detect_encoding`. Returns the normalised codec name and the bytes
/// to decode (without the UTF-8 BOM).
fn detect_encoding(bytes: &[u8]) -> Result<(String, &[u8]), DecodeError> {
    let (mut body, mut bom) = (bytes, false);
    if let Some(rest) = bytes.strip_prefix(b"\xef\xbb\xbf") {
        body = rest;
        bom = true;
    }
    let default = "utf-8".to_string();
    let (first, second) = first_lines(body);
    if first.is_empty() {
        return Ok((default, body));
    }
    if let Some(enc) = find_cookie(first, bom)? {
        return Ok((enc, body));
    }
    if !BLANK_RE.is_match(first) || second.is_empty() {
        return Ok((default, body));
    }
    Ok((find_cookie(second, bom)?.unwrap_or(default), body))
}

fn find_cookie(line: &[u8], bom: bool) -> Result<Option<String>, DecodeError> {
    let Ok(line) = std::str::from_utf8(line) else {
        return Err(DecodeError("invalid or missing encoding declaration".into()));
    };
    let Some(caps) = COOKIE_RE.captures(line) else { return Ok(None) };
    let raw = &caps[1];
    // `\w` with re.ASCII.
    if !raw.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        let ascii: String =
            raw.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')).collect();
        return check_codec(&ascii, bom).map(Some);
    }
    check_codec(raw, bom).map(Some)
}

fn check_codec(raw: &str, bom: bool) -> Result<String, DecodeError> {
    let enc = normal_name(raw);
    if codec(&enc).is_none() {
        return Err(DecodeError(format!("unknown encoding: {enc}")));
    }
    if bom && enc != "utf-8" {
        return Err(DecodeError("encoding problem: utf-8".into()));
    }
    Ok(enc)
}

/// `tokenize._get_normal_name`.
fn normal_name(orig: &str) -> String {
    let enc: String = orig.chars().take(12).collect::<String>().to_lowercase().replace('_', "-");
    if enc == "utf-8" || enc.starts_with("utf-8-") {
        return "utf-8".into();
    }
    let latin = ["latin-1", "iso-8859-1", "iso-latin-1"];
    if latin.contains(&enc.as_str()) || latin.iter().any(|l| enc.starts_with(&format!("{l}-"))) {
        return "iso-8859-1".into();
    }
    orig.to_string()
}

enum Codec {
    Utf8,
    Latin1,
    Ascii,
    Cp1252,
    Other(&'static encoding_rs::Encoding),
}

/// Codecs condor can decode. Python knows more; files using others are
/// reported as unreadable (documented divergence).
fn codec(name: &str) -> Option<Codec> {
    // encodings.normalize_encoding, roughly: lowercase, separators to '_'.
    let norm: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '_' })
        .collect();
    let norm = norm.trim_matches('_');
    match norm {
        "utf_8" | "utf8" | "u8" | "utf" | "cp65001" => return Some(Codec::Utf8),
        "latin_1" | "latin1" | "iso_8859_1" | "iso8859_1" | "8859" | "cp819" | "latin" | "l1" | "iso_ir_100" => {
            return Some(Codec::Latin1);
        }
        "ascii" | "us_ascii" | "646" | "us" | "ansi_x3.4_1968" | "ansi_x3_4_1968" | "iso646_us" => {
            return Some(Codec::Ascii);
        }
        "cp1252" | "windows_1252" | "1252" => return Some(Codec::Cp1252),
        _ => {}
    }
    // Multi-byte and ISO codecs whose WHATWG definitions match Python's.
    let whatwg = match norm {
        "iso_8859_2" | "iso8859_2" | "latin2" | "latin_2" | "l2" => "iso-8859-2",
        "iso_8859_15" | "iso8859_15" | "latin9" | "latin_9" | "l9" => "iso-8859-15",
        "koi8_r" => "koi8-r",
        "koi8_u" => "koi8-u",
        "cp1251" | "windows_1251" => "windows-1251",
        "cp1250" | "windows_1250" => "windows-1250",
        "euc_jp" | "eucjp" => "euc-jp",
        "shift_jis" | "sjis" | "shiftjis" | "s_jis" => "shift_jis",
        "gbk" | "cp936" | "ms936" => "gbk",
        "gb18030" => "gb18030",
        "big5" | "big5_tw" | "csbig5" => "big5",
        "euc_kr" | "euckr" => "euc-kr",
        _ => return None,
    };
    encoding_rs::Encoding::for_label(whatwg.as_bytes()).map(Codec::Other)
}

fn decode(encoding: &str, body: &[u8]) -> Result<String, DecodeError> {
    let err = |what: &str| DecodeError(format!("'{encoding}' codec can't decode: {what}"));
    match codec(encoding).ok_or_else(|| err("unknown codec"))? {
        Codec::Utf8 => String::from_utf8(body.to_vec()).map_err(|e| err(&e.to_string())),
        Codec::Latin1 => Ok(body.iter().map(|&b| b as char).collect()),
        Codec::Ascii => {
            if body.is_ascii() {
                Ok(body.iter().map(|&b| b as char).collect())
            } else {
                Err(err("ordinal not in range(128)"))
            }
        }
        Codec::Cp1252 => {
            // Python leaves these five bytes undefined; WHATWG maps them to C1 controls.
            if body.iter().any(|b| matches!(b, 0x81 | 0x8d | 0x8f | 0x90 | 0x9d)) {
                return Err(err("character maps to <undefined>"));
            }
            let (text, had_errors) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(body);
            if had_errors { Err(err("invalid data")) } else { Ok(text.into_owned()) }
        }
        Codec::Other(enc) => {
            let (text, had_errors) = enc.decode_without_bom_handling(body);
            if had_errors { Err(err("invalid data")) } else { Ok(text.into_owned()) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoding() {
        assert_eq!(decode_source(b"# -*- coding: latin-1 -*-\nx = '\xe9'\n").unwrap(), "# -*- coding: latin-1 -*-\nx = '\u{e9}'\n");
        assert_eq!(decode_source(b"\xef\xbb\xbfx = 1\r\n").unwrap(), "x = 1\n");
        assert!(decode_source(b"x = '\xe9'\n").is_err());
    }
}
