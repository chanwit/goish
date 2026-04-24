// Port of go1.25.5/src/strconv/atoi_test.go — tests for
// ParseUint / ParseInt / Atoi / NumError, table-driven.
//
// Only the logic-bearing subsections are ported; Go-specific things
// like `reflect.DeepEqual`, Go's `fmt.Errorf` plumbing, and the
// per-bitsize stress tests are adapted to Rust/goish idioms while
// preserving the data tables byte-for-byte.

#![allow(non_snake_case)]
use goish::prelude::*;

// ── ParseUint64 tests (base=10 implicit) ──────────────────────────────

struct PUTest { In: &'static str, Out: u64, Err: Option<error> }

fn parseUint64Tests() -> Vec<PUTest> { vec![
    PUTest { In: "",                    Out: 0, Err: Some(strconv::ErrSyntax()) },
    PUTest { In: "0",                   Out: 0, Err: None },
    PUTest { In: "1",                   Out: 1, Err: None },
    PUTest { In: "12345",               Out: 12345, Err: None },
    PUTest { In: "012345",              Out: 12345, Err: None },
    PUTest { In: "12345x",              Out: 0, Err: Some(strconv::ErrSyntax()) },
    PUTest { In: "98765432100",         Out: 98765432100, Err: None },
    PUTest { In: "18446744073709551615", Out: u64::MAX,   Err: None },
    PUTest { In: "18446744073709551616", Out: u64::MAX,   Err: Some(strconv::ErrRange()) },
    PUTest { In: "18446744073709551620", Out: u64::MAX,   Err: Some(strconv::ErrRange()) },
    PUTest { In: "1_2_3_4_5",           Out: 0, Err: Some(strconv::ErrSyntax()) }, // base=10 no _
    PUTest { In: "_12345",              Out: 0, Err: Some(strconv::ErrSyntax()) },
    PUTest { In: "1__2345",             Out: 0, Err: Some(strconv::ErrSyntax()) },
    PUTest { In: "12345_",              Out: 0, Err: Some(strconv::ErrSyntax()) },
    PUTest { In: "-0",                  Out: 0, Err: Some(strconv::ErrSyntax()) },
    PUTest { In: "-1",                  Out: 0, Err: Some(strconv::ErrSyntax()) },
    PUTest { In: "+1",                  Out: 0, Err: Some(strconv::ErrSyntax()) },
]}

fn err_matches(got: &error, want: &error) -> bool {
    let g = Sprintf!("%v", got);
    let w = Sprintf!("%v", want);
    strings::Contains(&g, &w)
}

test!{ fn TestParseUint64(t) {
    for test in parseUint64Tests() {
        let (n, e) = strconv::ParseUint(test.In, 10, 64);
        if n != test.Out {
            t.Errorf(Sprintf!("ParseUint(%q, 10, 64) = %d; want %d", test.In, n, test.Out));
        }
        match test.Err {
            None    => if e != nil { t.Errorf(Sprintf!("ParseUint(%q, 10, 64) err = %s; want nil", test.In, e)); },
            Some(w) => if !err_matches(&e, &w) {
                t.Errorf(Sprintf!("ParseUint(%q, 10, 64) err = %s; want %s", test.In, e, w));
            },
        }
    }
}}

// ── ParseInt64 tests ──────────────────────────────────────────────────

struct PITest { In: &'static str, Out: i64, Err: Option<error> }

fn parseInt64Tests() -> Vec<PITest> { vec![
    PITest { In: "",         Out: 0, Err: Some(strconv::ErrSyntax()) },
    PITest { In: "0",        Out: 0, Err: None },
    PITest { In: "-0",       Out: 0, Err: None },
    PITest { In: "+0",       Out: 0, Err: None },
    PITest { In: "1",        Out: 1, Err: None },
    PITest { In: "-1",       Out: -1, Err: None },
    PITest { In: "+1",       Out: 1, Err: None },
    PITest { In: "12345",    Out: 12345, Err: None },
    PITest { In: "-12345",   Out: -12345, Err: None },
    PITest { In: "012345",   Out: 12345, Err: None },
    PITest { In: "-012345",  Out: -12345, Err: None },
    PITest { In: "98765432100",  Out: 98765432100,  Err: None },
    PITest { In: "-98765432100", Out: -98765432100, Err: None },
    PITest { In: "9223372036854775807",  Out: i64::MAX,   Err: None },
    PITest { In: "-9223372036854775807", Out: -i64::MAX,  Err: None },
    PITest { In: "9223372036854775808",  Out: i64::MAX,   Err: Some(strconv::ErrRange()) },
    PITest { In: "-9223372036854775808", Out: i64::MIN,   Err: None },
    PITest { In: "9223372036854775809",  Out: i64::MAX,   Err: Some(strconv::ErrRange()) },
    PITest { In: "-9223372036854775809", Out: i64::MIN,   Err: Some(strconv::ErrRange()) },
    PITest { In: "-1_2_3_4_5", Out: 0, Err: Some(strconv::ErrSyntax()) },
    PITest { In: "-_12345",    Out: 0, Err: Some(strconv::ErrSyntax()) },
    PITest { In: "_12345",     Out: 0, Err: Some(strconv::ErrSyntax()) },
    PITest { In: "1__2345",    Out: 0, Err: Some(strconv::ErrSyntax()) },
    PITest { In: "12345_",     Out: 0, Err: Some(strconv::ErrSyntax()) },
    PITest { In: "123%45",     Out: 0, Err: Some(strconv::ErrSyntax()) },
]}

test!{ fn TestParseInt64(t) {
    for test in parseInt64Tests() {
        let (n, e) = strconv::ParseInt(test.In, 10, 64);
        if n != test.Out {
            t.Errorf(Sprintf!("ParseInt(%q, 10, 64) = %d; want %d", test.In, n, test.Out));
        }
        match test.Err {
            None    => if e != nil { t.Errorf(Sprintf!("ParseInt(%q, 10, 64) err = %s; want nil", test.In, e)); },
            Some(w) => if !err_matches(&e, &w) {
                t.Errorf(Sprintf!("ParseInt(%q, 10, 64) err = %s; want %s", test.In, e, w));
            },
        }
    }
}}

// ── ParseInt base=0 (infer) tests ────────────────────────────────────

struct PIBase { In: &'static str, Base: int, Out: i64, Err: Option<error> }

fn parseInt64BaseTests() -> Vec<PIBase> { vec![
    PIBase { In: "0",                   Base: 0,  Out: 0, Err: None },
    PIBase { In: "-0",                  Base: 0,  Out: 0, Err: None },
    PIBase { In: "1",                   Base: 0,  Out: 1, Err: None },
    PIBase { In: "-1",                  Base: 0,  Out: -1, Err: None },
    PIBase { In: "12345",               Base: 0,  Out: 12345, Err: None },
    PIBase { In: "-12345",              Base: 0,  Out: -12345, Err: None },
    PIBase { In: "012345",              Base: 0,  Out: 0o12345, Err: None },
    PIBase { In: "-012345",             Base: 0,  Out: -0o12345, Err: None },
    PIBase { In: "0x12345",             Base: 0,  Out: 0x12345, Err: None },
    PIBase { In: "-0X12345",            Base: 0,  Out: -0x12345, Err: None },
    PIBase { In: "12345x",              Base: 0,  Out: 0, Err: Some(strconv::ErrSyntax()) },
    PIBase { In: "-12345x",             Base: 0,  Out: 0, Err: Some(strconv::ErrSyntax()) },
    // base-17/25/35/36 spot checks
    PIBase { In: "g",                   Base: 17, Out: 16, Err: None },
    PIBase { In: "10",                  Base: 25, Out: 25, Err: None },
    // binary extremes
    PIBase { In: "1010",                Base: 2,  Out: 10, Err: None },
    PIBase { In: "1000000000000000",    Base: 2,  Out: 1 << 15, Err: None },
    // octal
    PIBase { In: "-10",                 Base: 8,  Out: -8, Err: None },
    // hex extremes
    PIBase { In: "10",                  Base: 16, Out: 16, Err: None },
    PIBase { In: "-123456789abcdef",    Base: 16, Out: -0x123456789abcdef, Err: None },
    PIBase { In: "7fffffffffffffff",    Base: 16, Out: i64::MAX, Err: None },
    // underscores with base=0 hex
    PIBase { In: "-0x_1_2_3_4_5",       Base: 0,  Out: -0x12345, Err: None },
    PIBase { In: "0x_1_2_3_4_5",        Base: 0,  Out: 0x12345, Err: None },
    PIBase { In: "_0x12345",            Base: 0,  Out: 0, Err: Some(strconv::ErrSyntax()) },
    PIBase { In: "0x__12345",           Base: 0,  Out: 0, Err: Some(strconv::ErrSyntax()) },
    PIBase { In: "0x12345_",            Base: 0,  Out: 0, Err: Some(strconv::ErrSyntax()) },
    PIBase { In: "+0xf",                Base: 0,  Out: 0xf, Err: None },
    PIBase { In: "-0xf",                Base: 0,  Out: -0xf, Err: None },
    PIBase { In: "0x+f",                Base: 0,  Out: 0, Err: Some(strconv::ErrSyntax()) },
    PIBase { In: "0x-f",                Base: 0,  Out: 0, Err: Some(strconv::ErrSyntax()) },
]}

test!{ fn TestParseInt64Base(t) {
    for test in parseInt64BaseTests() {
        let (n, e) = strconv::ParseInt(test.In, test.Base, 64);
        if n != test.Out {
            t.Errorf(Sprintf!("ParseInt(%q, %d, 64) = %d; want %d", test.In, test.Base, n, test.Out));
        }
        match test.Err {
            None    => if e != nil { t.Errorf(Sprintf!("ParseInt(%q, %d, 64) err = %s; want nil", test.In, test.Base, e)); },
            Some(w) => if !err_matches(&e, &w) {
                t.Errorf(Sprintf!("ParseInt(%q, %d, 64) err = %s; want %s", test.In, test.Base, e, w));
            },
        }
    }
}}

// ── ParseUint32 bit-size overflow tests ───────────────────────────────

struct PU32 { In: &'static str, Out: u32, Err: Option<error> }

fn parseUint32Tests() -> Vec<PU32> { vec![
    PU32 { In: "",                  Out: 0, Err: Some(strconv::ErrSyntax()) },
    PU32 { In: "0",                 Out: 0, Err: None },
    PU32 { In: "1",                 Out: 1, Err: None },
    PU32 { In: "12345",             Out: 12345, Err: None },
    PU32 { In: "012345",            Out: 12345, Err: None },
    PU32 { In: "12345x",            Out: 0, Err: Some(strconv::ErrSyntax()) },
    PU32 { In: "987654321",         Out: 987654321, Err: None },
    PU32 { In: "4294967295",        Out: u32::MAX, Err: None },
    PU32 { In: "4294967296",        Out: u32::MAX, Err: Some(strconv::ErrRange()) },
]}

test!{ fn TestParseUint32(t) {
    for test in parseUint32Tests() {
        let (n, e) = strconv::ParseUint(test.In, 10, 32);
        if n as u32 != test.Out {
            t.Errorf(Sprintf!("ParseUint(%q, 10, 32) = %d; want %d", test.In, n, test.Out));
        }
        match test.Err {
            None    => if e != nil { t.Errorf(Sprintf!("ParseUint(%q, 10, 32) err = %s; want nil", test.In, e)); },
            Some(w) => if !err_matches(&e, &w) {
                t.Errorf(Sprintf!("ParseUint(%q, 10, 32) err = %s; want %s", test.In, e, w));
            },
        }
    }
}}

// ── ParseInt32 bit-size overflow ──────────────────────────────────────

struct PI32 { In: &'static str, Out: i32, Err: Option<error> }

fn parseInt32Tests() -> Vec<PI32> { vec![
    PI32 { In: "",             Out: 0, Err: Some(strconv::ErrSyntax()) },
    PI32 { In: "0",            Out: 0, Err: None },
    PI32 { In: "-0",           Out: 0, Err: None },
    PI32 { In: "1",            Out: 1, Err: None },
    PI32 { In: "-1",           Out: -1, Err: None },
    PI32 { In: "12345",        Out: 12345, Err: None },
    PI32 { In: "-12345",       Out: -12345, Err: None },
    PI32 { In: "2147483647",   Out: i32::MAX, Err: None },
    PI32 { In: "-2147483647",  Out: -i32::MAX, Err: None },
    PI32 { In: "2147483648",   Out: i32::MAX, Err: Some(strconv::ErrRange()) },
    PI32 { In: "-2147483648",  Out: i32::MIN, Err: None },
    PI32 { In: "2147483649",   Out: i32::MAX, Err: Some(strconv::ErrRange()) },
    PI32 { In: "-2147483649",  Out: i32::MIN, Err: Some(strconv::ErrRange()) },
]}

test!{ fn TestParseInt32(t) {
    for test in parseInt32Tests() {
        let (n, e) = strconv::ParseInt(test.In, 10, 32);
        if n as i32 != test.Out {
            t.Errorf(Sprintf!("ParseInt(%q, 10, 32) = %d; want %d", test.In, n, test.Out));
        }
        match test.Err {
            None    => if e != nil { t.Errorf(Sprintf!("ParseInt(%q, 10, 32) err = %s; want nil", test.In, e)); },
            Some(w) => if !err_matches(&e, &w) {
                t.Errorf(Sprintf!("ParseInt(%q, 10, 32) err = %s; want %s", test.In, e, w));
            },
        }
    }
}}

// ── Atoi equivalence ──────────────────────────────────────────────────

test!{ fn TestAtoi(t) {
    // Atoi == ParseInt(s, 10, 0) → int
    let cases = [("0", 0), ("-0", 0), ("1", 1), ("-1", -1), ("12345", 12345), ("98765", 98765)];
    for (s, want) in cases {
        let (n, err) = strconv::Atoi(s);
        if err != nil { t.Errorf(Sprintf!("Atoi(%q) err = %s; want nil", s, err)); }
        if n != want  { t.Errorf(Sprintf!("Atoi(%q) = %d; want %d", s, n, want)); }
    }

    let bad = ["", "abc", "1a", "-x", "1_2"];
    for s in bad {
        let (_, err) = strconv::Atoi(s);
        if err == nil { t.Errorf(Sprintf!("Atoi(%q) err = nil; want error", s)); }
        let msg = Sprintf!("%v", err);
        if !strings::Contains(&msg, "strconv.Atoi") {
            t.Errorf(Sprintf!("Atoi(%q) err does not say 'strconv.Atoi': %s", s, err));
        }
    }
}}
