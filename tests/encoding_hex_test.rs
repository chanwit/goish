// Port of go1.25.5 src/encoding/hex/hex_test.go — EncodeToString,
// DecodeString, EncodedLen/DecodedLen.
//
// Skipped: Encoder/Decoder streaming, Dump/Dumper (not in goish v0.11).

#![allow(non_snake_case)]
use goish::prelude::*;

struct Pair {
    decoded: &'static [u8],
    encoded: &'static str,
}

fn pairs() -> slice<Pair> {
    vec![
        Pair { decoded: &[0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7],         encoded: "0001020304050607" },
        Pair { decoded: &[0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf],         encoded: "08090a0b0c0d0e0f" },
        Pair { decoded: &[0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7], encoded: "f0f1f2f3f4f5f6f7" },
        Pair { decoded: &[0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff], encoded: "f8f9fafbfcfdfeff" },
        Pair { decoded: b"",                                                encoded: "" },
        Pair { decoded: b"\x67",                                            encoded: "67" },
    ].into()
}

test!{ fn TestEncode(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let got = encoding::hex::EncodeToString(p.decoded);
        if got != p.encoded {
            t.Errorf(Sprintf!("EncodeToString = %q, want %q", got, p.encoded));
        }
    }
}}

test!{ fn TestDecodeString(t) {
    let __pairs = pairs();
    for (_, p) in range!(__pairs) {
        let (got, err) = encoding::hex::DecodeString(p.encoded);
        if err != nil {
            t.Errorf(Sprintf!("DecodeString(%q) error = %s", p.encoded, err));
            continue;
        }
        if got != p.decoded {
            t.Errorf(Sprintf!("DecodeString(%q) mismatch", p.encoded));
        }
    }
}}

test!{ fn TestDecodeErr(t) {
    // Odd-length input is an error.
    let (_, err) = encoding::hex::DecodeString("1");
    if err == nil {
        t.Errorf(Sprintf!("DecodeString(\"1\") expected error"));
    }
    // Non-hex character.
    let (_, err) = encoding::hex::DecodeString("zz");
    if err == nil {
        t.Errorf(Sprintf!("DecodeString(\"zz\") expected error"));
    }
}}

test!{ fn TestEncodedLen(t) {
    for (n, want) in [(0i64, 0i64), (1, 2), (2, 4), (10, 20)] {
        let got = encoding::hex::EncodedLen(n);
        if got != want {
            t.Errorf(Sprintf!("EncodedLen(%d) = %d, want %d", n, got, want));
        }
    }
}}

test!{ fn TestDecodedLen(t) {
    for (n, want) in [(0i64, 0i64), (2, 1), (4, 2), (20, 10)] {
        let got = encoding::hex::DecodedLen(n);
        if got != want {
            t.Errorf(Sprintf!("DecodedLen(%d) = %d, want %d", n, got, want));
        }
    }
}}
