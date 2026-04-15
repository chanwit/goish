//! hash/fnv: Fowler-Noll-Vo 32/64-bit hash (FNV-1 and FNV-1a).

use crate::types::{byte, int};

const OFFSET32: u32 = 0x811c_9dc5;
const PRIME32: u32 = 0x0100_0193;
const OFFSET64: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME64: u64 = 0x0000_0100_0000_01b3;

pub struct Hash32 { state: u32, is_a: bool }
pub struct Hash64 { state: u64, is_a: bool }

impl Hash32 {
    pub fn Write(&mut self, p: &[byte]) -> (int, crate::errors::error) {
        for &b in p {
            if self.is_a {
                self.state ^= b as u32;
                self.state = self.state.wrapping_mul(PRIME32);
            } else {
                self.state = self.state.wrapping_mul(PRIME32);
                self.state ^= b as u32;
            }
        }
        (p.len() as int, crate::errors::nil)
    }
    pub fn Sum32(&self) -> u32 { self.state }
    pub fn Reset(&mut self) { self.state = OFFSET32; }
}

impl Hash64 {
    pub fn Write(&mut self, p: &[byte]) -> (int, crate::errors::error) {
        for &b in p {
            if self.is_a {
                self.state ^= b as u64;
                self.state = self.state.wrapping_mul(PRIME64);
            } else {
                self.state = self.state.wrapping_mul(PRIME64);
                self.state ^= b as u64;
            }
        }
        (p.len() as int, crate::errors::nil)
    }
    pub fn Sum64(&self) -> u64 { self.state }
    pub fn Reset(&mut self) { self.state = OFFSET64; }
}

#[allow(non_snake_case)] pub fn New32() -> Hash32 { Hash32 { state: OFFSET32, is_a: false } }
#[allow(non_snake_case)] pub fn New32a() -> Hash32 { Hash32 { state: OFFSET32, is_a: true } }
#[allow(non_snake_case)] pub fn New64() -> Hash64 { Hash64 { state: OFFSET64, is_a: false } }
#[allow(non_snake_case)] pub fn New64a() -> Hash64 { Hash64 { state: OFFSET64, is_a: true } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv32a_hello() {
        let mut h = New32a();
        h.Write(b"hello");
        assert_eq!(h.Sum32(), 0x4f9f2cab);
    }

    #[test]
    fn fnv64a_hello() {
        let mut h = New64a();
        h.Write(b"hello");
        assert_eq!(h.Sum64(), 0xa430d84680aabd0b);
    }

    #[test]
    fn fnv_streaming_matches_single_write() {
        let mut h1 = New64a();
        h1.Write(b"hello world");
        let mut h2 = New64a();
        h2.Write(b"hello ");
        h2.Write(b"world");
        assert_eq!(h1.Sum64(), h2.Sum64());
    }

    #[test]
    fn fnv32_not_same_as_fnv32a() {
        let mut a = New32();
        a.Write(b"x");
        let mut b = New32a();
        b.Write(b"x");
        assert_ne!(a.Sum32(), b.Sum32());
    }
}
