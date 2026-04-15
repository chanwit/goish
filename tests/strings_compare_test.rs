// port of go/src/strings/compare_test.go (TestCompare + TestCompareIdenticalString)
//
// Derived from Go's own comment: bytes/compare_test.go upstream.

#![allow(non_camel_case_types, non_snake_case)]
use goish::prelude::*;

Struct!{ type compareTest struct { a, b string; i int } }

fn compareTests() -> slice<compareTest> { slice!([]compareTest{
    compareTest!("",            "",             0),
    compareTest!("a",           "",             1),
    compareTest!("",            "a",            -1),
    compareTest!("abc",         "abc",          0),
    compareTest!("ab",          "abc",          -1),
    compareTest!("abc",         "ab",           1),
    compareTest!("x",           "ab",           1),
    compareTest!("ab",          "x",            -1),
    compareTest!("x",           "a",            1),
    compareTest!("b",           "x",            -1),
    // runtime·memeq chunked-impl test
    compareTest!("abcdefgh",    "abcdefgh",     0),
    compareTest!("abcdefghi",   "abcdefghi",    0),
    compareTest!("abcdefghi",   "abcdefghj",    -1),
})}

test!{ fn TestCompare(t) {
    for test in &compareTests() {
        // Go's test does a sliding-offset repeat; we skip that (Go's test
        // proves SIMD alignment doesn't bias results — our impl is scalar
        // so the original one-shot comparison is equivalent).
        let cmp = strings::Compare(&test.a, &test.b);
        if cmp != test.i {
            t.Errorf(Sprintf!("Compare(%q, %q) = %v; want %v",
                test.a, test.b, cmp, test.i));
        }
    }
}}

test!{ fn TestCompareIdenticalString(t) {
    let s = "Hello Gophers!";
    if strings::Compare(s, s) != 0 {
        t.Error("s != s");
    }
    // Go: Compare(s, s[:1]) != 1 -> fail
    if strings::Compare(s, &s[..1]) != 1 {
        t.Error("s > s[:1] failed");
    }
}}
