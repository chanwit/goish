// port of go/src/strconv/itoa_test.go
//
// Second acid-test of the test!/Struct! framework. Exercises large table-
// driven data with i64::MIN / i64::MAX edge cases.

#![allow(non_camel_case_types)]
use goish::prelude::*;

// Go: type itob64Test struct { in int64; base int; out string }
Struct!{ type itob64Test struct { in_ int64; base int; out string } }
//                                 ^^^ `in` is a Rust keyword — one-letter rename
// is the only concession vs. Go. All other tests below use `test.in_`.

fn itob64tests() -> slice<itob64Test> { slice!([]itob64Test{
    itob64Test!(0, 10, "0"),
    itob64Test!(1, 10, "1"),
    itob64Test!(-1, 10, "-1"),
    itob64Test!(12345678, 10, "12345678"),
    itob64Test!(-987654321, 10, "-987654321"),
    itob64Test!(i32::MAX as int64, 10, "2147483647"),
    itob64Test!((-(i32::MAX as int64)), 10, "-2147483647"),
    itob64Test!(1i64 << 31, 10, "2147483648"),
    itob64Test!(-(1i64 << 31), 10, "-2147483648"),
    itob64Test!((1i64 << 31) + 1, 10, "2147483649"),
    itob64Test!(-(1i64 << 31) - 1, 10, "-2147483649"),
    itob64Test!((1i64 << 32) - 1, 10, "4294967295"),
    itob64Test!(-((1i64 << 32) - 1), 10, "-4294967295"),
    itob64Test!(1i64 << 32, 10, "4294967296"),
    itob64Test!(-(1i64 << 32), 10, "-4294967296"),
    itob64Test!((1i64 << 32) + 1, 10, "4294967297"),
    itob64Test!(-((1i64 << 32) + 1), 10, "-4294967297"),
    itob64Test!(1i64 << 50, 10, "1125899906842624"),
    itob64Test!(int64::MAX, 10, "9223372036854775807"),
    itob64Test!(-int64::MAX, 10, "-9223372036854775807"),
    itob64Test!(int64::MIN, 10, "-9223372036854775808"),

    itob64Test!(0, 2, "0"),
    itob64Test!(10, 2, "1010"),
    itob64Test!(-1, 2, "-1"),
    itob64Test!(1i64 << 15, 2, "1000000000000000"),

    itob64Test!(-8, 8, "-10"),
    itob64Test!(0o57635436545, 8, "57635436545"),
    itob64Test!(1i64 << 24, 8, "100000000"),

    itob64Test!(16, 16, "10"),
    itob64Test!(-0x123456789abcdef, 16, "-123456789abcdef"),
    itob64Test!(int64::MAX, 16, "7fffffffffffffff"),
    itob64Test!(int64::MAX, 2,
        "111111111111111111111111111111111111111111111111111111111111111"),
    itob64Test!(int64::MIN, 2,
        "-1000000000000000000000000000000000000000000000000000000000000000"),

    itob64Test!(16, 17, "g"),
    itob64Test!(25, 25, "10"),
    itob64Test!((((((17*35+24)*35+21)*35+34)*35+12)*35+24)*35 + 32, 35, "holycow"),
    itob64Test!((((((17*36+24)*36+21)*36+34)*36+12)*36+24)*36 + 32, 36, "holycow"),
})}

test!{ fn TestItoa(t) {
    for test in &itob64tests() {
        let s = strconv::FormatInt(test.in_, test.base);
        if s != test.out {
            t.Errorf(Sprintf!("FormatInt(%v, %v) = %v want %v",
                test.in_, test.base, s, test.out));
        }
        let x = strconv::AppendInt(b"abc".to_vec(), test.in_, test.base);
        let xs = String::from_utf8(x).unwrap();
        let want = format!("abc{}", test.out);
        if xs != want {
            t.Errorf(Sprintf!("AppendInt(%q, %v, %v) = %q want %v",
                "abc", test.in_, test.base, xs, test.out));
        }

        if test.in_ >= 0 {
            let s = strconv::FormatUint(test.in_ as u64, test.base);
            if s != test.out {
                t.Errorf(Sprintf!("FormatUint(%v, %v) = %v want %v",
                    test.in_, test.base, s, test.out));
            }
            let x = strconv::AppendUint(Vec::new(), test.in_ as u64, test.base);
            let xs = String::from_utf8(x).unwrap();
            if xs != test.out {
                t.Errorf(Sprintf!("AppendUint(%q, %v, %v) = %q want %v",
                    "abc", test.in_ as u64, test.base, xs, test.out));
            }
        }

        if test.base == 10 {
            // goish: int is already int64, so `int(test.in_) == test.in_` is
            // trivially true — no conditional needed. Go's itoa_test gates on
            // platform-sized int; in goish this is always taken.
            let s = strconv::Itoa(test.in_);
            if s != test.out {
                t.Errorf(Sprintf!("Itoa(%v) = %v want %v",
                    test.in_, s, test.out));
            }
        }
    }

    // Override when base is illegal — expect panic.
    //
    // Go uses defer+recover; goish uses recover!{} which maps the same
    // pattern onto a single macro call.
    let r = recover!{ strconv::FormatUint(12345678, 1) };
    if r.is_none() {
        t.Fatal("expected panic due to illegal base");
    }
}}

// ── TestUitoa ──────────────────────────────────────────────────────────

Struct!{ type uitob64Test struct { in_ uint64; base int; out string } }

fn uitob64tests() -> slice<uitob64Test> { slice!([]uitob64Test{
    uitob64Test!(int64::MAX as u64, 10, "9223372036854775807"),
    uitob64Test!(1u64 << 63, 10, "9223372036854775808"),
    uitob64Test!((1u64 << 63) + 1, 10, "9223372036854775809"),
    uitob64Test!(u64::MAX - 1, 10, "18446744073709551614"),
    uitob64Test!(u64::MAX, 10, "18446744073709551615"),
    uitob64Test!(u64::MAX, 2,
        "1111111111111111111111111111111111111111111111111111111111111111"),
})}

test!{ fn TestUitoa(t) {
    for test in &uitob64tests() {
        let s = strconv::FormatUint(test.in_, test.base);
        if s != test.out {
            t.Errorf(Sprintf!("FormatUint(%v, %v) = %v want %v",
                test.in_, test.base, s, test.out));
        }
        let x = strconv::AppendUint(b"abc".to_vec(), test.in_, test.base);
        let xs = String::from_utf8(x).unwrap();
        let want = format!("abc{}", test.out);
        if xs != want {
            t.Errorf(Sprintf!("AppendUint(%q, %v, %v) = %q want %v",
                "abc", test.in_, test.base, xs, test.out));
        }
    }
}}
