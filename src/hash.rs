// hash: Go's hash/crc32 and hash/fnv packages — non-cryptographic hashers.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   crc32.ChecksumIEEE(data)            hash::crc32::ChecksumIEEE(&data)
//   t := crc32.MakeTable(crc32.IEEE)    let t = hash::crc32::MakeTable(hash::crc32::IEEE);
//   crc32.Checksum(data, t)             hash::crc32::Checksum(&data, &t)
//
//   h := fnv.New32a()                   let mut h = hash::fnv::New32a();
//   h.Write(data)                       h.Write(&data);
//   h.Sum32()                           h.Sum32();
//
// Namespaced as nested modules to match Go's package layout.

pub mod crc32 {
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
}

pub mod fnv {
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
            // FNV-1a 32 of "hello" = 0x4f9f2cab
            assert_eq!(h.Sum32(), 0x4f9f2cab);
        }

        #[test]
        fn fnv64a_hello() {
            let mut h = New64a();
            h.Write(b"hello");
            // FNV-1a 64 of "hello" = 0xa430d84680aabd0b
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
}
