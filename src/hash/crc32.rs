//! hash/crc32: cyclic redundancy check, reflected polynomials (IEEE standard).

use crate::types::{byte, int};

pub const IEEE: u32 = 0xedb88320;
pub const Castagnoli: u32 = 0x82f63b78;
pub const Koopman: u32 = 0xeb31d82e;

#[derive(Clone)]
pub struct Table(pub [u32; 256]);

/// Generate a 256-entry CRC-32 table for the given reflected polynomial.
#[allow(non_snake_case)]
pub fn MakeTable(poly: u32) -> Table {
    let mut t = [0u32; 256];
    for i in 0..256u32 {
        let mut crc = i;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ poly;
            } else {
                crc >>= 1;
            }
        }
        t[i as usize] = crc;
    }
    Table(t)
}

#[allow(non_upper_case_globals)]
pub fn IEEETable() -> &'static Table {
    use std::sync::OnceLock;
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| MakeTable(IEEE))
}

#[allow(non_snake_case)]
pub fn Checksum(data: &[byte], tab: &Table) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    for &b in data {
        crc = tab.0[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

#[allow(non_snake_case)]
pub fn ChecksumIEEE(data: &[byte]) -> u32 {
    Checksum(data, IEEETable())
}

/// hash.Hash32 interface — state machine style.
pub struct Hash32 {
    tab: &'static Table,
    crc: u32,
}

impl Hash32 {
    pub fn Write(&mut self, p: &[byte]) -> (int, crate::errors::error) {
        for &b in p {
            self.crc = self.tab.0[((self.crc ^ b as u32) & 0xff) as usize] ^ (self.crc >> 8);
        }
        (p.len() as int, crate::errors::nil)
    }
    pub fn Sum32(&self) -> u32 { self.crc ^ 0xffff_ffff }
    pub fn Reset(&mut self) { self.crc = 0xffff_ffff; }
    pub fn Size(&self) -> int { 4 }
    pub fn BlockSize(&self) -> int { 1 }
}

#[allow(non_snake_case)]
pub fn NewIEEE() -> Hash32 {
    Hash32 { tab: IEEETable(), crc: 0xffff_ffff }
}

#[allow(non_snake_case)]
pub fn New(tab: &'static Table) -> Hash32 {
    Hash32 { tab, crc: 0xffff_ffff }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ieee_checksum_hello_world() {
        assert_eq!(ChecksumIEEE(b"hello world"), 0x0d4a1185);
    }

    #[test]
    fn empty_checksum_is_zero() {
        assert_eq!(ChecksumIEEE(b""), 0);
    }

    #[test]
    fn hash32_matches_checksum() {
        let mut h = NewIEEE();
        h.Write(b"hello ");
        h.Write(b"world");
        assert_eq!(h.Sum32(), 0x0d4a1185);
        h.Reset();
        assert_eq!(h.Sum32(), 0);
    }
}
