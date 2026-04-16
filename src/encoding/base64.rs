// encoding/base64: Go's encoding/base64, ported.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   base64.StdEncoding.EncodeToString(b)    base64::StdEncoding.EncodeToString(b)
//   base64.StdEncoding.DecodeString(s)      base64::StdEncoding.DecodeString(s)
//   base64.URLEncoding                       base64::URLEncoding
//   base64.RawStdEncoding                    base64::RawStdEncoding  (no padding)
//   base64.RawURLEncoding                    base64::RawURLEncoding

use crate::errors::{error, nil, New};
use crate::types::{byte, string};

const STD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub struct Encoding {
    alphabet: &'static [u8; 64],
    pad: Option<u8>,
    decode_table: [i8; 256],
}

impl Encoding {
    const fn new(alphabet: &'static [u8; 64], pad: Option<u8>) -> Self {
        let mut t = [-1i8; 256];
        let mut i = 0;
        while i < 64 {
            t[alphabet[i] as usize] = i as i8;
            i += 1;
        }
        Encoding { alphabet, pad, decode_table: t }
    }

    #[allow(non_snake_case)]
    pub fn EncodeToString(&self, src: &[byte]) -> string {
        let mut out = String::with_capacity((src.len() + 2) / 3 * 4);
        let mut i = 0;
        while i + 3 <= src.len() {
            let b0 = src[i];
            let b1 = src[i + 1];
            let b2 = src[i + 2];
            out.push(self.alphabet[(b0 >> 2) as usize] as char);
            out.push(self.alphabet[((b0 & 0x03) << 4 | b1 >> 4) as usize] as char);
            out.push(self.alphabet[((b1 & 0x0f) << 2 | b2 >> 6) as usize] as char);
            out.push(self.alphabet[(b2 & 0x3f) as usize] as char);
            i += 3;
        }
        let rem = src.len() - i;
        if rem == 1 {
            let b0 = src[i];
            out.push(self.alphabet[(b0 >> 2) as usize] as char);
            out.push(self.alphabet[((b0 & 0x03) << 4) as usize] as char);
            if let Some(p) = self.pad {
                out.push(p as char);
                out.push(p as char);
            }
        } else if rem == 2 {
            let b0 = src[i];
            let b1 = src[i + 1];
            out.push(self.alphabet[(b0 >> 2) as usize] as char);
            out.push(self.alphabet[((b0 & 0x03) << 4 | b1 >> 4) as usize] as char);
            out.push(self.alphabet[((b1 & 0x0f) << 2) as usize] as char);
            if let Some(p) = self.pad {
                out.push(p as char);
            }
        }
        out.into()
    }

    #[allow(non_snake_case)]
    pub fn DecodeString(&self, s: impl AsRef<str>) -> (Vec<byte>, error) {
        let src = s.as_ref().as_bytes();
        // Strip optional padding for raw variants that might still receive it.
        let mut end = src.len();
        if let Some(pad) = self.pad {
            while end > 0 && src[end - 1] == pad {
                end -= 1;
            }
        }
        let src = &src[..end];

        let mut out = Vec::with_capacity(src.len() * 3 / 4);
        let mut buf = [0i32; 4];
        let mut bi = 0;
        for &b in src {
            let v = self.decode_table[b as usize];
            if v < 0 {
                return (Vec::new(), New(&format!("encoding/base64: invalid byte: {:#x}", b)));
            }
            buf[bi] = v as i32;
            bi += 1;
            if bi == 4 {
                out.push(((buf[0] << 2) | (buf[1] >> 4)) as u8);
                out.push(((buf[1] << 4) | (buf[2] >> 2)) as u8);
                out.push(((buf[2] << 6) | buf[3]) as u8);
                bi = 0;
            }
        }
        match bi {
            0 => {}
            2 => out.push(((buf[0] << 2) | (buf[1] >> 4)) as u8),
            3 => {
                out.push(((buf[0] << 2) | (buf[1] >> 4)) as u8);
                out.push(((buf[1] << 4) | (buf[2] >> 2)) as u8);
            }
            _ => return (Vec::new(), New("encoding/base64: unexpected EOF")),
        }
        (out, nil)
    }
}

#[allow(non_upper_case_globals)]
pub static StdEncoding: Encoding = Encoding::new(STD_ALPHABET, Some(b'='));
#[allow(non_upper_case_globals)]
pub static URLEncoding: Encoding = Encoding::new(URL_ALPHABET, Some(b'='));
#[allow(non_upper_case_globals)]
pub static RawStdEncoding: Encoding = Encoding::new(STD_ALPHABET, None);
#[allow(non_upper_case_globals)]
pub static RawURLEncoding: Encoding = Encoding::new(URL_ALPHABET, None);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_std() {
        assert_eq!(StdEncoding.EncodeToString(b""), "");
        assert_eq!(StdEncoding.EncodeToString(b"f"), "Zg==");
        assert_eq!(StdEncoding.EncodeToString(b"fo"), "Zm8=");
        assert_eq!(StdEncoding.EncodeToString(b"foo"), "Zm9v");
        assert_eq!(StdEncoding.EncodeToString(b"Hello, World!"), "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn decode_std() {
        let (b, err) = StdEncoding.DecodeString("SGVsbG8sIFdvcmxkIQ==");
        assert!(err == nil);
        assert_eq!(b, b"Hello, World!");
    }

    #[test]
    fn round_trip() {
        let src: Vec<u8> = (0..=255).collect();
        let s = StdEncoding.EncodeToString(&src);
        let (b, err) = StdEncoding.DecodeString(&s);
        assert!(err == nil);
        assert_eq!(b, src);
    }

    #[test]
    fn raw_has_no_padding() {
        let s = RawStdEncoding.EncodeToString(b"f");
        assert_eq!(s, "Zg");
        let (b, err) = RawStdEncoding.DecodeString("Zg");
        assert!(err == nil);
        assert_eq!(b, b"f");
    }

    #[test]
    fn url_uses_dash_underscore() {
        // value that generates + and / in std
        let s = StdEncoding.EncodeToString(&[0xff, 0xff]);
        assert!(s.contains('/') || s.contains('+'));
        let u = URLEncoding.EncodeToString(&[0xff, 0xff]);
        assert!(!u.contains('/') && !u.contains('+'));
    }

    #[test]
    fn decode_invalid() {
        let (_, err) = StdEncoding.DecodeString("!!!!");
        assert!(err != nil);
    }
}
