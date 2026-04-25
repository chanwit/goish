// Port of go1.25.5/src/strings/builder_test.go — core Builder test cases.
// Allocation-observation tests, internal asan hooks, and Copy-on-grow
// micro-behaviour are Go-runtime-specific and elided.

#![allow(non_snake_case)]
use goish::prelude::*;

fn check(t: &testing::T, b: &strings::Builder, want: &str) {
    let got = b.String();
    if got != want {
        t.Errorf(Sprintf!("String: got %q; want %q", got, want));
        return;
    }
    let n = b.Len();
    if n != got.len() as i64 {
        t.Errorf(Sprintf!("Len: got %d; but len(String()) is %d", n, got.len()));
    }
    let c = b.Cap();
    if c < got.len() as i64 {
        t.Errorf(Sprintf!("Cap: got %d; but len(String()) is %d", c, got.len()));
    }
}

test!{ fn TestBuilder(t) {
    let mut b = strings::Builder::new();
    check(t, &b, "");
    let (n, err) = b.WriteString("hello");
    if err != nil || n != 5 {
        t.Errorf(Sprintf!("WriteString: got %d,%s; want 5,nil", n, err));
    }
    check(t, &b, "hello");
    let err = b.WriteByte(b' ');
    if err != nil { t.Errorf(Sprintf!("WriteByte: %s", err)); }
    check(t, &b, "hello ");
    let (n, err) = b.WriteString("world");
    if err != nil || n != 5 {
        t.Errorf(Sprintf!("WriteString: got %d,%s; want 5,nil", n, err));
    }
    check(t, &b, "hello world");
}}

test!{ fn TestBuilderString(t) {
    let mut b = strings::Builder::new();
    b.WriteString("alpha");
    let s1 = b.String();
    b.WriteString(", beta");
    let s2 = b.String();
    b.WriteString(", gamma");
    let s3 = b.String();
    // All three should still hold their captured values.
    if s1 != "alpha" { t.Errorf(Sprintf!("want %q got %q", "alpha", s1)); }
    if s2 != "alpha, beta" { t.Errorf(Sprintf!("want %q got %q", "alpha, beta", s2)); }
    if s3 != "alpha, beta, gamma" { t.Errorf(Sprintf!("want %q got %q", "alpha, beta, gamma", s3)); }
}}

test!{ fn TestBuilderReset(t) {
    let mut b = strings::Builder::new();
    check(t, &b, "");
    b.WriteString("aaa");
    let s = b.String();
    check(t, &b, "aaa");
    b.Reset();
    check(t, &b, "");

    // Subsequent writes start fresh.
    b.WriteString("bbb");
    check(t, &b, "bbb");
    if s != "aaa" {
        t.Errorf(Sprintf!("captured string lost Reset: got %q", s));
    }
}}

test!{ fn TestBuilderGrow(t) {
    for grow_len in [0i64, 100, 1000, 10000, 100000] {
        let mut b = strings::Builder::new();
        b.Grow(grow_len);
        if b.Cap() < grow_len {
            t.Errorf(Sprintf!("growLen=%d: Cap() = %d, want >= %d", grow_len, b.Cap(), grow_len));
        }
        let p = strings::Repeat("a", grow_len);
        b.WriteString(&p);
        if b.String() != p {
            t.Errorf(Sprintf!("growLen=%d: String mismatch", grow_len));
        }
    }
}}

test!{ fn TestBuilderWriteByte(t) {
    let mut b = strings::Builder::new();
    let err = b.WriteByte(b'a');
    if err != nil { t.Errorf(Sprintf!("WriteByte: %s", err)); }
    let err = b.WriteByte(0);
    if err != nil { t.Errorf(Sprintf!("WriteByte NUL: %s", err)); }
    check(t, &b, "a\x00");
}}
