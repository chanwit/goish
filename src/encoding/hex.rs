// encoding/hex: Go's encoding/hex, ported.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   s := hex.EncodeToString(b)          let s = hex::EncodeToString(b);
//   b, err := hex.DecodeString(s)       let (b, err) = hex::DecodeString(s);
//   n := hex.EncodedLen(len)            let n = hex::EncodedLen(len);
//   n := hex.DecodedLen(len)            let n = hex::DecodedLen(len);

use crate::errors::{error, nil, New};
use crate::types::{byte, int, string};

#[allow(non_snake_case)]
pub fn EncodedLen(n: int) -> int { n * 2 }

#[allow(non_snake_case)]
pub fn DecodedLen(n: int) -> int { n / 2 }

/// Encode bytes to lowercase hex.
#[allow(non_snake_case)]
pub fn EncodeToString(src: &[byte]) -> string {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(src.len() * 2);
    for &b in src {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out.into()
}

/// Decode a hex string. Odd length or non-hex chars return an error.
#[allow(non_snake_case)]
pub fn DecodeString(s: impl AsRef<str>) -> (crate::types::slice<byte>, error) {
    let s = s.as_ref();
    if s.len() % 2 != 0 {
        return (crate::types::slice::new(), New("encoding/hex: odd length hex string"));
    }
    let bytes = s.as_bytes();
    let mut out: Vec<byte> = Vec::with_capacity(s.len() / 2);
    for chunk in bytes.chunks(2) {
        let hi = match decode_nibble(chunk[0]) {
            Some(v) => v,
            None => return (crate::types::slice::new(), invalid(chunk[0])),
        };
        let lo = match decode_nibble(chunk[1]) {
            Some(v) => v,
            None => return (crate::types::slice::new(), invalid(chunk[1])),
        };
        out.push((hi << 4) | lo);
    }
    (out.into(), nil)
}

fn decode_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn invalid(b: u8) -> error {
    New(&format!("encoding/hex: invalid byte: {:#x}", b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_basic() {
        assert_eq!(EncodeToString(b"hi"), "6869");
        assert_eq!(EncodeToString(&[0x00, 0xff]), "00ff");
        assert_eq!(EncodeToString(b""), "");
    }

    #[test]
    fn decode_basic() {
        let (b, err) = DecodeString("6869");
        assert!(err == nil);
        assert_eq!(b, b"hi");

        let (b, err) = DecodeString("00FF");
        assert!(err == nil);
        assert_eq!(b, vec![0, 255]);
    }

    #[test]
    fn decode_errors() {
        let (_, err) = DecodeString("abc"); // odd
        assert!(err != nil);

        let (_, err) = DecodeString("zz"); // non-hex
        assert!(err != nil);
    }

    #[test]
    fn round_trip() {
        let src = [0u8, 1, 2, 3, 127, 255, 42];
        let s = EncodeToString(&src);
        let (b, err) = DecodeString(&s);
        assert!(err == nil);
        assert_eq!(b, src);
    }

    #[test]
    fn lengths() {
        assert_eq!(EncodedLen(5), 10);
        assert_eq!(DecodedLen(10), 5);
        assert_eq!(DecodedLen(11), 5);
    }
}
