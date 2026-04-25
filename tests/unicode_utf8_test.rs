// Port of go1.25.5 src/unicode/utf8/utf8_test.go — RuneCountInString,
// ValidString, DecodeRuneInString, EncodeRune, RuneLen.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestRuneCountInString(t) {
    let cases: slice<(&str, int)> = vec![
        ("", 0),
        ("abcd", 4),
        ("☺☻☹", 3),
        ("1,2,3,4", 7),
        ("日本語", 3),
        ("aé", 2),
    ].into();
    for (inp, want) in cases {
        let got = utf8::RuneCountInString(inp);
        if got != want {
            t.Errorf(Sprintf!("RuneCountInString(%q) = %d, want %d", inp, got, want));
        }
    }
}}

test!{ fn TestValidString(t) {
    // Valid UTF-8 strings.
    for s in ["", "a", "abc", "日本語", "☺", "\x7f", "\u{0080}"] {
        if !utf8::ValidString(s) {
            t.Errorf(Sprintf!("ValidString(%q) = false", s));
        }
    }
}}

test!{ fn TestRuneLen(t) {
    let cases: slice<(i32, int)> = vec![
        (0x00, 1),       // ASCII
        (0x7f, 1),
        (0x80, 2),       // two bytes
        (0x07ff, 2),
        (0x0800, 3),     // three bytes
        (0xffff, 3),
        (0x10000, 4),    // four bytes
        (0x10ffff, 4),   // max valid
        (-1, -1),        // invalid
        (0x110000, -1),  // out of range
    ].into();
    for (r, want) in cases {
        let got = utf8::RuneLen(r);
        if got != want {
            t.Errorf(Sprintf!("RuneLen(%x) = %d, want %d", r as i64, got, want));
        }
    }
}}

test!{ fn TestEncodeRune(t) {
    let cases: Vec<(i32, &[u8])> = vec![
        (0x41, &[0x41]),           // 'A'
        (0xe9, &[0xc3, 0xa9]),     // 'é'
        (0x4e2d, &[0xe4, 0xb8, 0xad]), // '中'
        (0x1f600, &[0xf0, 0x9f, 0x98, 0x80]), // '😀'
    ];
    for (r, want_bytes) in cases {
        let mut buf = [0u8; 4];
        let n = utf8::EncodeRune(&mut buf, r);
        if n as usize != want_bytes.len() {
            t.Errorf(Sprintf!("EncodeRune(%x) wrote %d bytes, want %d",
                r as i64, n, want_bytes.len() as i64));
            continue;
        }
        if &buf[..n as usize] != want_bytes {
            t.Errorf(Sprintf!("EncodeRune(%x) bytes mismatch", r as i64));
        }
    }
}}

test!{ fn TestDecodeRuneInString(t) {
    let cases: slice<(&str, i32, int)> = vec![
        ("A", 0x41, 1),
        ("é", 0xe9, 2),
        ("中", 0x4e2d, 3),
        ("😀", 0x1f600, 4),
    ].into();
    for (inp, want_rune, want_size) in cases {
        let (r, n) = utf8::DecodeRuneInString(inp);
        if r != want_rune || n != want_size {
            t.Errorf(Sprintf!("DecodeRuneInString(%q) = (%x, %d), want (%x, %d)",
                inp, r as i64, n, want_rune as i64, want_size));
        }
    }
}}

test!{ fn TestRoundTripEncodeDecode(t) {
    // Encode then decode should be a round-trip.
    for r in [0x41i32, 0xe9, 0x4e2d, 0x1f600] {
        let mut buf = [0u8; 4];
        let n = utf8::EncodeRune(&mut buf, r);
        let s = std::str::from_utf8(&buf[..n as usize]).unwrap();
        let (r2, n2) = utf8::DecodeRuneInString(s);
        if r2 != r || n2 != n {
            t.Errorf(Sprintf!("roundtrip r=%x: got (%x, %d) want (%x, %d)",
                r as i64, r2 as i64, n2, r as i64, n));
        }
    }
}}
