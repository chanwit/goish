// port of go/src/bytes/bytes_test.go (subset)
//
// Exercises bytes::Equal, bytes::Index, bytes::LastIndex on a small sample
// of Go's fixtures. The exhaustive table-data / allocation-measurement tests
// are out of scope for the first port.

#![allow(non_camel_case_types, non_snake_case)]
use goish::prelude::*;

Struct!{ type BinOpTest struct { a, b string; i int } }

// ── TestEqual (data-table subset) ──────────────────────────────────────

fn compareTests() -> slice<BinOpTest> { slice!([]BinOpTest{
    BinOpTest!("",           "",             0),
    BinOpTest!("a",          "",             1),
    BinOpTest!("",           "a",            -1),
    BinOpTest!("abc",        "abc",          0),
    BinOpTest!("ab",         "abc",          -1),
    BinOpTest!("abc",        "ab",           1),
})}

test!{ fn TestEqual(t) {
    for tt in &compareTests() {
        let eql = bytes::Equal(&tt.a, &tt.b);
        if eql != (tt.i == 0) {
            t.Errorf(Sprintf!("Equal(%q, %q) = %v", tt.a, tt.b, eql));
        }
    }
}}

// ── TestIndex ──────────────────────────────────────────────────────────

fn indexTests() -> slice<BinOpTest> { slice!([]BinOpTest{
    BinOpTest!("",              "",          0),
    BinOpTest!("",              "a",         -1),
    BinOpTest!("",              "foo",       -1),
    BinOpTest!("fo",            "foo",       -1),
    BinOpTest!("foo",           "baz",       -1),
    BinOpTest!("foo",           "foo",       0),
    BinOpTest!("oofofoofooo",   "f",         2),
    BinOpTest!("oofofoofooo",   "foo",       4),
    BinOpTest!("barfoobarfoo",  "foo",       3),
    BinOpTest!("foo",           "",          0),
    BinOpTest!("foo",           "o",         1),
    BinOpTest!("abcABCabc",     "A",         3),
})}

test!{ fn TestIndex(t) {
    for tt in &indexTests() {
        let got = bytes::Index(&tt.a, &tt.b);
        if got != tt.i {
            t.Errorf(Sprintf!("Index(%q, %q) = %v; want %v",
                tt.a, tt.b, got, tt.i));
        }
    }
}}

// ── TestLastIndex ──────────────────────────────────────────────────────

fn lastIndexTests() -> slice<BinOpTest> { slice!([]BinOpTest{
    BinOpTest!("",              "",          0),
    BinOpTest!("",              "a",         -1),
    BinOpTest!("",              "foo",       -1),
    BinOpTest!("fo",            "foo",       -1),
    BinOpTest!("foo",           "foo",       0),
    BinOpTest!("foo",           "f",         0),
    BinOpTest!("oofofoofooo",   "f",         7),
    BinOpTest!("oofofoofooo",   "foo",       7),
    BinOpTest!("barfoobarfoo",  "foo",       9),
    BinOpTest!("foo",           "",          3),
    BinOpTest!("foo",           "o",         2),
    BinOpTest!("abcABCabc",     "A",         3),
})}

test!{ fn TestLastIndex(t) {
    for tt in &lastIndexTests() {
        let got = bytes::LastIndex(&tt.a, &tt.b);
        if got != tt.i {
            t.Errorf(Sprintf!("LastIndex(%q, %q) = %v; want %v",
                tt.a, tt.b, got, tt.i));
        }
    }
}}

// ── TestIndexByte ──────────────────────────────────────────────────────

test!{ fn TestIndexByte(t) {
    let cases = slice!([]BinOpTest{
        BinOpTest!("",              "a",         -1),
        BinOpTest!("a",             "a",         0),
        BinOpTest!("abcABCabc",     "A",         3),
        BinOpTest!("oofofoofooo",   "f",         2),
    });
    for tt in &cases {
        // Go's IndexByte takes a byte; we take the first byte of tt.b.
        let b = tt.b.as_bytes()[0];
        let got = bytes::IndexByte(&tt.a, b);
        if got != tt.i {
            t.Errorf(Sprintf!("IndexByte(%q, %q) = %v; want %v",
                tt.a, tt.b, got, tt.i));
        }
    }
}}
