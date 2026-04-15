// unicode/utf8: Go's unicode/utf8 package.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   utf8.RuneCountInString(s)           utf8::RuneCountInString(s)
//   utf8.ValidString(s)                 utf8::ValidString(s)
//   utf8.DecodeRuneInString(s)          utf8::DecodeRuneInString(s) → (rune, int)
//   utf8.EncodeRune(buf, r)             utf8::EncodeRune(&mut buf, r)
//   utf8.RuneLen(r)                     utf8::RuneLen(r)
//
// Rust's &str is UTF-8 by construction, so most of these are thin wrappers
// over char methods and slice operations.

use crate::types::{byte, int, rune};

pub const UTFMax: int = 4;

#[allow(non_snake_case)]
pub fn RuneCountInString(s: impl AsRef<str>) -> int {
    s.as_ref().chars().count() as int
}

#[allow(non_snake_case)]
pub fn RuneCount(p: &[byte]) -> int {
    // Decode as UTF-8 ignoring errors; invalid bytes count as one RuneError each.
    let mut n = 0;
    let mut i = 0;
    while i < p.len() {
        match std::str::from_utf8(&p[i..]) {
            Ok(s) => {
                n += s.chars().count() as int;
                break;
            }
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                if valid_up_to > 0 {
                    n += std::str::from_utf8(&p[i..i + valid_up_to]).unwrap().chars().count() as int;
                }
                // Skip one invalid byte as a RuneError.
                n += 1;
                i += valid_up_to + 1;
            }
        }
    }
    n
}

#[allow(non_snake_case)]
pub fn ValidString(s: impl AsRef<str>) -> bool {
    // &str is always valid UTF-8.
    let _ = s;
    true
}

#[allow(non_snake_case)]
pub fn Valid(p: &[byte]) -> bool {
    std::str::from_utf8(p).is_ok()
}

/// utf8.DecodeRuneInString(s) → (r rune, size int).
/// On empty input returns (RuneError, 0); on invalid returns (RuneError, 1).
#[allow(non_snake_case)]
pub fn DecodeRuneInString(s: impl AsRef<str>) -> (rune, int) {
    let s = s.as_ref();
    match s.chars().next() {
        Some(c) => (c as rune, c.len_utf8() as int),
        None => (crate::unicode::RuneError, 0),
    }
}

/// utf8.EncodeRune(buf, r) → n bytes written. Panics if buf is too small.
#[allow(non_snake_case)]
pub fn EncodeRune(buf: &mut [byte], r: rune) -> int {
    let c = char::from_u32(r as u32).unwrap_or(char::from_u32(crate::unicode::RuneError as u32).unwrap());
    let s = c.encode_utf8(&mut [0u8; 4]).to_string();
    let bytes = s.as_bytes();
    if buf.len() < bytes.len() {
        panic!("utf8.EncodeRune: buffer too small");
    }
    buf[..bytes.len()].copy_from_slice(bytes);
    bytes.len() as int
}

/// utf8.RuneLen(r) → byte length when encoded, or -1 if r is not valid.
#[allow(non_snake_case)]
pub fn RuneLen(r: rune) -> int {
    match char::from_u32(r as u32) {
        Some(c) => c.len_utf8() as int,
        None => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rune_count() {
        assert_eq!(RuneCountInString("hello"), 5);
        assert_eq!(RuneCountInString("漢字"), 2);
        assert_eq!(RuneCountInString(""), 0);
    }

    #[test]
    fn valid_string() {
        assert!(ValidString("hello"));
        assert!(Valid(b"hello"));
        assert!(!Valid(&[0xffu8, 0xfe])); // invalid UTF-8
    }

    #[test]
    fn decode_rune() {
        let (r, n) = DecodeRuneInString("漢字");
        assert_eq!(r, '漢' as rune);
        assert_eq!(n, 3);
        let (_, n) = DecodeRuneInString("");
        assert_eq!(n, 0);
    }

    #[test]
    fn encode_rune() {
        let mut buf = [0u8; 4];
        let n = EncodeRune(&mut buf, 'λ' as rune);
        assert_eq!(n, 2);
        assert_eq!(&buf[..2], "λ".as_bytes());
    }

    #[test]
    fn rune_len() {
        assert_eq!(RuneLen('a' as rune), 1);
        assert_eq!(RuneLen('λ' as rune), 2);
        assert_eq!(RuneLen('漢' as rune), 3);
        assert_eq!(RuneLen(0x110000), -1); // out of range
    }
}
