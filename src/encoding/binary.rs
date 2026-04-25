// binary: Go's encoding/binary package — big/little endian encoding of
// fixed-width numeric types.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   binary.BigEndian.Uint16(b)          binary::BigEndian.Uint16(&b)
//   binary.BigEndian.PutUint32(b, v)    binary::BigEndian.PutUint32(&mut b, v)
//   binary.LittleEndian.Uint64(b)       binary::LittleEndian.Uint64(&b)
//
// `ByteOrder` is a struct with the same methods in both big/little variants
// so callers can choose. Bounds are checked and panic on out-of-range access
// (matches Go's `binary` panic-on-short-input behavior).

use crate::types::byte;

pub struct ByteOrder {
    big: bool,
}

#[allow(non_upper_case_globals)]
pub const BigEndian: ByteOrder = ByteOrder { big: true };
#[allow(non_upper_case_globals)]
pub const LittleEndian: ByteOrder = ByteOrder { big: false };

impl ByteOrder {
    pub fn Uint16(&self, b: &[byte]) -> u16 {
        if self.big {
            u16::from_be_bytes([b[0], b[1]])
        } else {
            u16::from_le_bytes([b[0], b[1]])
        }
    }

    pub fn Uint32(&self, b: &[byte]) -> u32 {
        let bytes: [u8; 4] = [b[0], b[1], b[2], b[3]];
        if self.big { u32::from_be_bytes(bytes) } else { u32::from_le_bytes(bytes) }
    }

    pub fn Uint64(&self, b: &[byte]) -> u64 {
        let bytes: [u8; 8] = [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]];
        if self.big { u64::from_be_bytes(bytes) } else { u64::from_le_bytes(bytes) }
    }

    pub fn PutUint16(&self, b: &mut [byte], v: u16) {
        let bytes = if self.big { v.to_be_bytes() } else { v.to_le_bytes() };
        b[0] = bytes[0]; b[1] = bytes[1];
    }

    pub fn PutUint32(&self, b: &mut [byte], v: u32) {
        let bytes = if self.big { v.to_be_bytes() } else { v.to_le_bytes() };
        b[..4].copy_from_slice(&bytes);
    }

    pub fn PutUint64(&self, b: &mut [byte], v: u64) {
        let bytes = if self.big { v.to_be_bytes() } else { v.to_le_bytes() };
        b[..8].copy_from_slice(&bytes);
    }

    pub fn AppendUint16(&self, b: crate::types::slice<byte>, v: u16) -> crate::types::slice<byte> {
        let bytes = if self.big { v.to_be_bytes() } else { v.to_le_bytes() };
        let mut out: Vec<byte> = b.into();
        out.extend_from_slice(&bytes);
        out.into()
    }

    pub fn AppendUint32(&self, b: crate::types::slice<byte>, v: u32) -> crate::types::slice<byte> {
        let bytes = if self.big { v.to_be_bytes() } else { v.to_le_bytes() };
        let mut out: Vec<byte> = b.into();
        out.extend_from_slice(&bytes);
        out.into()
    }

    pub fn AppendUint64(&self, b: crate::types::slice<byte>, v: u64) -> crate::types::slice<byte> {
        let bytes = if self.big { v.to_be_bytes() } else { v.to_le_bytes() };
        let mut out: Vec<byte> = b.into();
        out.extend_from_slice(&bytes);
        out.into()
    }

    pub fn String(&self) -> &'static str {
        if self.big { "BigEndian" } else { "LittleEndian" }
    }
}

// ── Variable-length integer encoding ──────────────────────────────────
//
// Go's binary.PutUvarint / binary.Uvarint — LEB128 unsigned variable encoding
// used by gob and protobuf.

/// Upper bound for encoded Uvarint (u64): 10 bytes.
pub const MaxVarintLen64: usize = 10;
pub const MaxVarintLen32: usize = 5;
pub const MaxVarintLen16: usize = 3;

#[allow(non_snake_case)]
pub fn PutUvarint(buf: &mut [byte], mut x: u64) -> crate::types::int {
    let mut i = 0;
    while x >= 0x80 {
        buf[i] = (x as u8) | 0x80;
        x >>= 7;
        i += 1;
    }
    buf[i] = x as u8;
    (i + 1) as crate::types::int
}

/// Uvarint(buf) — returns (value, nbytes_consumed). nbytes <= 0 on error:
/// 0 = insufficient bytes; < 0 = overflow.
#[allow(non_snake_case)]
pub fn Uvarint(buf: &[byte]) -> (u64, crate::types::int) {
    let mut x: u64 = 0;
    let mut s: u32 = 0;
    for (i, &b) in buf.iter().enumerate() {
        if i == MaxVarintLen64 {
            return (0, -((i as crate::types::int) + 1));
        }
        if b < 0x80 {
            if i == MaxVarintLen64 - 1 && b > 1 {
                return (0, -((i as crate::types::int) + 1));
            }
            return (x | ((b as u64) << s), (i + 1) as crate::types::int);
        }
        x |= ((b & 0x7f) as u64) << s;
        s += 7;
    }
    (0, 0)
}

#[allow(non_snake_case)]
pub fn PutVarint(buf: &mut [byte], x: i64) -> crate::types::int {
    let ux = if x < 0 { (!((x as u64) << 1)).wrapping_add(1) | 1 } else { (x as u64) << 1 };
    // Simpler: zig-zag encoding.
    let zig = ((x << 1) ^ (x >> 63)) as u64;
    let _ = ux;
    PutUvarint(buf, zig)
}

#[allow(non_snake_case)]
pub fn Varint(buf: &[byte]) -> (i64, crate::types::int) {
    let (u, n) = Uvarint(buf);
    let x = ((u >> 1) as i64) ^ -((u & 1) as i64);
    (x, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_endian_round_trip() {
        let mut b = [0u8; 8];
        BigEndian.PutUint64(&mut b, 0x0102_0304_0506_0708);
        assert_eq!(b, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(BigEndian.Uint64(&b), 0x0102_0304_0506_0708);
    }

    #[test]
    fn little_endian_round_trip() {
        let mut b = [0u8; 4];
        LittleEndian.PutUint32(&mut b, 0xdead_beef);
        assert_eq!(b, [0xef, 0xbe, 0xad, 0xde]);
        assert_eq!(LittleEndian.Uint32(&b), 0xdead_beef);
    }

    #[test]
    fn u16_round_trip_both_orders() {
        let mut b = [0u8; 2];
        BigEndian.PutUint16(&mut b, 0x1234);
        assert_eq!(BigEndian.Uint16(&b), 0x1234);
        LittleEndian.PutUint16(&mut b, 0x1234);
        assert_eq!(LittleEndian.Uint16(&b), 0x1234);
    }

    #[test]
    fn append_uint_helpers() {
        let out = BigEndian.AppendUint32(vec![0xaau8].into(), 1u32);
        assert_eq!(out, vec![0xaa, 0, 0, 0, 1]);
    }

    #[test]
    fn uvarint_encodes_small() {
        let mut b = [0u8; MaxVarintLen64];
        let n = PutUvarint(&mut b, 1) as usize;
        assert_eq!(n, 1);
        assert_eq!(b[0], 1);
        let (v, read) = Uvarint(&b[..n]);
        assert_eq!(v, 1);
        assert_eq!(read, 1);
    }

    #[test]
    fn uvarint_encodes_large() {
        let mut b = [0u8; MaxVarintLen64];
        let n = PutUvarint(&mut b, 300) as usize;
        // 300 = 0xAC 0x02 (lsb first, with MSBs)
        assert_eq!(&b[..n], &[0xac, 0x02]);
        let (v, _) = Uvarint(&b[..n]);
        assert_eq!(v, 300);
    }

    #[test]
    fn varint_round_trips_signed() {
        for &v in &[0i64, 1, -1, 150, -150, i64::MAX, i64::MIN] {
            let mut b = [0u8; MaxVarintLen64];
            let n = PutVarint(&mut b, v) as usize;
            let (r, _) = Varint(&b[..n]);
            assert_eq!(r, v, "varint roundtrip {} != {}", r, v);
        }
    }

    #[test]
    fn strings() {
        assert_eq!(BigEndian.String(), "BigEndian");
        assert_eq!(LittleEndian.String(), "LittleEndian");
    }
}
