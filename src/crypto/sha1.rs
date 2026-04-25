//! crypto/sha1: hand-rolled SHA-1.
//!
//! SHA-1 is broken for cryptographic use; for checksums / interop only.

use crate::types::{byte, int};

pub const Size: int = 20;
pub const BlockSize: int = 64;

pub struct Digest {
    state: [u32; 5],
    buf: Vec<byte>,
    len: u64,
}

#[allow(non_snake_case)]
pub fn New() -> Digest {
    Digest {
        state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0],
        buf: Vec::with_capacity(64),
        len: 0,
    }
}

impl Digest {
    pub fn Write(&mut self, p: &[byte]) -> (int, crate::errors::error) {
        self.len = self.len.wrapping_add(p.len() as u64);
        self.buf.extend_from_slice(p);
        while self.buf.len() >= 64 {
            let block: [u8; 64] = self.buf[..64].try_into().unwrap();
            process(&mut self.state, &block);
            self.buf.drain(..64);
        }
        (p.len() as int, crate::errors::nil)
    }

    pub fn Sum(&self, b: impl AsRef<[byte]>) -> crate::types::slice<byte> {
        let mut state = self.state;
        let mut buf = self.buf.clone();
        let bit_len = self.len.wrapping_mul(8);
        buf.push(0x80);
        while buf.len() % 64 != 56 {
            buf.push(0);
        }
        buf.extend_from_slice(&bit_len.to_be_bytes());
        let mut i = 0;
        while i < buf.len() {
            let block: [u8; 64] = buf[i..i + 64].try_into().unwrap();
            process(&mut state, &block);
            i += 64;
        }
        let mut out: Vec<byte> = b.as_ref().to_vec();
        for v in state.iter() {
            out.extend_from_slice(&v.to_be_bytes());
        }
        out.into()
    }

    pub fn Reset(&mut self) {
        self.state = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];
        self.buf.clear();
        self.len = 0;
    }
}

#[allow(non_snake_case)]
pub fn Sum(data: impl AsRef<[byte]>) -> [byte; 20] {
    let mut h = New();
    h.Write(data.as_ref());
    let v = h.Sum(&[]);
    let mut out = [0u8; 20];
    out.copy_from_slice(&v);
    out
}

fn process(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            block[i * 4], block[i * 4 + 1], block[i * 4 + 2], block[i * 4 + 3]
        ]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    for i in 0..80 {
        let (f, k) = match i {
            0..=19  => ((b & c) | (!b & d), 0x5a827999u32),
            20..=39 => (b ^ c ^ d, 0x6ed9eba1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
            _        => (b ^ c ^ d, 0xca62c1d6),
        };
        let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(w[i]);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{:02x}", x)).collect()
    }

    #[test]
    fn sha1_empty() {
        let h = Sum(b"");
        assert_eq!(hex(&h), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn sha1_abc() {
        let h = Sum(b"abc");
        assert_eq!(hex(&h), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn sha1_quick_brown_fox() {
        let h = Sum(b"The quick brown fox jumps over the lazy dog");
        assert_eq!(hex(&h), "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
    }

    #[test]
    fn sha1_streaming_equiv() {
        let mut h = New();
        h.Write(b"abc");
        let out = h.Sum(&[]);
        assert_eq!(hex(&out), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }
}
