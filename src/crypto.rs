// crypto: Go's crypto/md5, crypto/sha1, crypto/sha256 — hand-rolled.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   h := md5.Sum(data)                  let sum = crypto::md5::Sum(&data);
//   h := sha1.Sum(data)                 let sum = crypto::sha1::Sum(&data);
//   h := sha256.Sum256(data)            let sum = crypto::sha256::Sum256(&data);
//
//   h := md5.New()                      let mut h = crypto::md5::New();
//   h.Write(data)                       h.Write(&data);
//   out := h.Sum(nil)                   let out = h.Sum(&[]);
//
// All three hashers share a common Digest trait-ish shape with Write/Sum/Reset.
//
// These implementations are for ecosystem parity with Go, not for FIPS
// certification. MD5 and SHA-1 are broken for cryptographic use; use them
// only for checksums / interop.

pub mod md5 {
    use crate::types::{byte, int};

    pub const Size: int = 16;
    pub const BlockSize: int = 64;

    pub struct Digest {
        state: [u32; 4],
        buf: Vec<byte>,
        len: u64,
    }

    #[allow(non_snake_case)]
    pub fn New() -> Digest {
        Digest {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
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

        pub fn Sum(&self, b: &[byte]) -> Vec<byte> {
            // Clone internal state so Sum doesn't mutate the digest (matches Go).
            let mut state = self.state;
            let mut buf = self.buf.clone();
            let bit_len = self.len.wrapping_mul(8);
            buf.push(0x80);
            while buf.len() % 64 != 56 {
                buf.push(0);
            }
            buf.extend_from_slice(&bit_len.to_le_bytes());
            let mut i = 0;
            while i < buf.len() {
                let block: [u8; 64] = buf[i..i + 64].try_into().unwrap();
                process(&mut state, &block);
                i += 64;
            }
            let mut out = b.to_vec();
            for v in state.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out
        }

        pub fn Reset(&mut self) {
            self.state = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
            self.buf.clear();
            self.len = 0;
        }
    }

    #[allow(non_snake_case)]
    pub fn Sum(data: &[byte]) -> [byte; 16] {
        let mut h = New();
        h.Write(data);
        let v = h.Sum(&[]);
        let mut out = [0u8; 16];
        out.copy_from_slice(&v);
        out
    }

    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
        5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
        6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a,
        0xa8304613, 0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
        0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340,
        0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8,
        0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
        0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92,
        0xffeff47d, 0x85845dd1, 0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
        0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];

    fn process(state: &mut [u32; 4], block: &[u8; 64]) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([
                block[i * 4], block[i * 4 + 1], block[i * 4 + 2], block[i * 4 + 3]
            ]);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        for i in 0..64 {
            let (f, g) = match i {
                0..=15  => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _        => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            let sum = a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(S[i]));
            a = temp;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn md5_empty() {
            let h = Sum(b"");
            assert_eq!(
                hex(&h),
                "d41d8cd98f00b204e9800998ecf8427e"
            );
        }

        #[test]
        fn md5_abc() {
            let h = Sum(b"abc");
            assert_eq!(hex(&h), "900150983cd24fb0d6963f7d28e17f72");
        }

        #[test]
        fn md5_streaming_matches_single() {
            let mut a = New();
            a.Write(b"hello ");
            a.Write(b"world");
            let sb = Sum(b"hello world");
            assert_eq!(a.Sum(&[]), sb.to_vec());
        }

        fn hex(b: &[u8]) -> String {
            b.iter().map(|x| format!("{:02x}", x)).collect()
        }
    }
}

pub mod sha1 {
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

        pub fn Sum(&self, b: &[byte]) -> Vec<byte> {
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
            let mut out = b.to_vec();
            for v in state.iter() {
                out.extend_from_slice(&v.to_be_bytes());
            }
            out
        }

        pub fn Reset(&mut self) {
            self.state = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];
            self.buf.clear();
            self.len = 0;
        }
    }

    #[allow(non_snake_case)]
    pub fn Sum(data: &[byte]) -> [byte; 20] {
        let mut h = New();
        h.Write(data);
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
}

pub mod sha256 {
    use crate::types::{byte, int};

    pub const Size: int = 32;
    pub const BlockSize: int = 64;

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
        0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
        0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    pub struct Digest {
        state: [u32; 8],
        buf: Vec<byte>,
        len: u64,
    }

    #[allow(non_snake_case)]
    pub fn New() -> Digest {
        Digest {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
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

        pub fn Sum(&self, b: &[byte]) -> Vec<byte> {
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
            let mut out = b.to_vec();
            for v in state.iter() {
                out.extend_from_slice(&v.to_be_bytes());
            }
            out
        }

        pub fn Reset(&mut self) {
            self.state = [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ];
            self.buf.clear();
            self.len = 0;
        }
    }

    #[allow(non_snake_case)]
    pub fn Sum256(data: &[byte]) -> [byte; 32] {
        let mut h = New();
        h.Write(data);
        let v = h.Sum(&[]);
        let mut out = [0u8; 32];
        out.copy_from_slice(&v);
        out
    }

    fn process(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4], block[i * 4 + 1], block[i * 4 + 2], block[i * 4 + 3]
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let mj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(mj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn hex(b: &[u8]) -> String {
            b.iter().map(|x| format!("{:02x}", x)).collect()
        }

        #[test]
        fn sha256_empty() {
            let h = Sum256(b"");
            assert_eq!(
                hex(&h),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
        }

        #[test]
        fn sha256_abc() {
            let h = Sum256(b"abc");
            assert_eq!(
                hex(&h),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            );
        }

        #[test]
        fn sha256_long_message() {
            let msg = b"The quick brown fox jumps over the lazy dog";
            let h = Sum256(msg);
            assert_eq!(
                hex(&h),
                "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
            );
        }

        #[test]
        fn sha256_streaming_equals_single() {
            let mut a = New();
            a.Write(b"hello ");
            a.Write(b"world");
            let out_a = a.Sum(&[]);
            let out_b = Sum256(b"hello world").to_vec();
            assert_eq!(out_a, out_b);
        }
    }
}
