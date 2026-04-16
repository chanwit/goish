// Port of go1.25.5 src/encoding/binary/binary_test.go — BigEndian,
// LittleEndian uint16/32/64 round-trips + Uvarint / Varint.
//
// Skipped: Read/Write reflection-based (not in goish), NativeEndian.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestBigEndianUint16(t) {
    let mut b = [0u8; 2];
    encoding::binary::BigEndian.PutUint16(&mut b, 0x1234);
    if b != [0x12, 0x34] {
        t.Errorf(Sprintf!("PutUint16 big endian = %v, want [0x12, 0x34]", format!("{:?}", b)));
    }
    let v = encoding::binary::BigEndian.Uint16(&b);
    if v != 0x1234 {
        t.Errorf(Sprintf!("Uint16 = %x, want 0x1234", v as i64));
    }
}}

test!{ fn TestLittleEndianUint16(t) {
    let mut b = [0u8; 2];
    encoding::binary::LittleEndian.PutUint16(&mut b, 0x1234);
    if b != [0x34, 0x12] {
        t.Errorf(Sprintf!("PutUint16 little endian = %v, want [0x34, 0x12]", format!("{:?}", b)));
    }
    let v = encoding::binary::LittleEndian.Uint16(&b);
    if v != 0x1234 {
        t.Errorf(Sprintf!("Uint16 = %x, want 0x1234", v as i64));
    }
}}

test!{ fn TestBigEndianUint32(t) {
    let mut b = [0u8; 4];
    encoding::binary::BigEndian.PutUint32(&mut b, 0x12345678);
    if b != [0x12, 0x34, 0x56, 0x78] {
        t.Errorf(Sprintf!("PutUint32 big endian mismatch: %v", format!("{:?}", b)));
    }
    let v = encoding::binary::BigEndian.Uint32(&b);
    if v != 0x12345678 {
        t.Errorf(Sprintf!("Uint32 = %x, want 0x12345678", v as i64));
    }
}}

test!{ fn TestLittleEndianUint32(t) {
    let mut b = [0u8; 4];
    encoding::binary::LittleEndian.PutUint32(&mut b, 0x12345678);
    if b != [0x78, 0x56, 0x34, 0x12] {
        t.Errorf(Sprintf!("PutUint32 little endian mismatch: %v", format!("{:?}", b)));
    }
}}

test!{ fn TestBigEndianUint64(t) {
    let mut b = [0u8; 8];
    encoding::binary::BigEndian.PutUint64(&mut b, 0x0102030405060708);
    if b != [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08] {
        t.Errorf(Sprintf!("PutUint64 big endian mismatch: %v", format!("{:?}", b)));
    }
    let v = encoding::binary::BigEndian.Uint64(&b);
    if v != 0x0102030405060708 {
        t.Errorf(Sprintf!("Uint64 mismatch"));
    }
}}

test!{ fn TestLittleEndianUint64(t) {
    let mut b = [0u8; 8];
    encoding::binary::LittleEndian.PutUint64(&mut b, 0x0102030405060708);
    if b != [0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01] {
        t.Errorf(Sprintf!("PutUint64 little endian mismatch: %v", format!("{:?}", b)));
    }
}}

// Go's Uvarint tests: empty slice → (0, 0), overlong → (0, -n).
test!{ fn TestUvarint(t) {
    // Simple values 0..127 fit in one byte with top bit clear.
    for v in [0u64, 1, 127] {
        let mut buf = [0u8; 10];
        let n = encoding::binary::PutUvarint(&mut buf, v);
        if n != 1 {
            t.Errorf(Sprintf!("PutUvarint(%d) wrote %d bytes, want 1", v as i64, n));
        }
        let (got, consumed) = encoding::binary::Uvarint(&buf[..n as usize]);
        if got != v || consumed != n {
            t.Errorf(Sprintf!("Uvarint round-trip: %d vs %d", got as i64, v as i64));
        }
    }
    // 128 requires 2 bytes.
    let mut buf = [0u8; 10];
    let n = encoding::binary::PutUvarint(&mut buf, 128);
    if n != 2 {
        t.Errorf(Sprintf!("PutUvarint(128) wrote %d bytes, want 2", n));
    }
    // Max u64 requires 10 bytes.
    let n = encoding::binary::PutUvarint(&mut buf, u64::MAX);
    if n != 10 {
        t.Errorf(Sprintf!("PutUvarint(u64::MAX) wrote %d bytes, want 10", n));
    }
    let (got, _) = encoding::binary::Uvarint(&buf[..n as usize]);
    if got != u64::MAX {
        t.Errorf(Sprintf!("Uvarint(u64::MAX) = %d", got as i64));
    }
}}

test!{ fn TestVarint(t) {
    // Varint round-trips for a handful of signed values.
    for v in [0i64, 1, -1, 63, -64, 64, -65, i64::MAX, i64::MIN] {
        let mut buf = [0u8; 10];
        let n = encoding::binary::PutVarint(&mut buf, v);
        if n <= 0 {
            t.Errorf(Sprintf!("PutVarint(%d) returned %d", v, n));
            continue;
        }
        let (got, consumed) = encoding::binary::Varint(&buf[..n as usize]);
        if got != v || consumed != n {
            t.Errorf(Sprintf!("Varint round-trip: got %d consumed %d want %d / %d",
                got, consumed, v, n));
        }
    }
}}

test!{ fn TestMaxVarintLens(t) {
    if encoding::binary::MaxVarintLen16 != 3 {
        t.Errorf(Sprintf!("MaxVarintLen16 = %d, want 3", encoding::binary::MaxVarintLen16 as i64));
    }
    if encoding::binary::MaxVarintLen32 != 5 {
        t.Errorf(Sprintf!("MaxVarintLen32 = %d, want 5", encoding::binary::MaxVarintLen32 as i64));
    }
    if encoding::binary::MaxVarintLen64 != 10 {
        t.Errorf(Sprintf!("MaxVarintLen64 = %d, want 10", encoding::binary::MaxVarintLen64 as i64));
    }
}}
