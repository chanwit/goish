// Port of go1.25.5 src/unicode/letter_test.go — predicate and case-map
// checks on ASCII + representative Unicode ranges.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestIsLetter(t) {
    for r in ['a', 'z', 'A', 'Z', 'é', 'α', 'ア'] {
        if !unicode::IsLetter(r as u32 as i32) {
            t.Errorf(Sprintf!("IsLetter(%q) = false", r));
        }
    }
    for r in ['0', '9', ' ', '.', '\t'] {
        if unicode::IsLetter(r as u32 as i32) {
            t.Errorf(Sprintf!("IsLetter(%q) = true", r));
        }
    }
}}

test!{ fn TestIsDigit(t) {
    for r in ['0', '9', '5'] {
        if !unicode::IsDigit(r as u32 as i32) {
            t.Errorf(Sprintf!("IsDigit(%q) = false", r));
        }
    }
    for r in ['a', 'A', '.', ' '] {
        if unicode::IsDigit(r as u32 as i32) {
            t.Errorf(Sprintf!("IsDigit(%q) = true", r));
        }
    }
}}

test!{ fn TestIsSpace(t) {
    for r in [' ', '\t', '\n', '\r'] {
        if !unicode::IsSpace(r as u32 as i32) {
            t.Errorf(Sprintf!("IsSpace(%q) = false", r));
        }
    }
    for r in ['a', '0'] {
        if unicode::IsSpace(r as u32 as i32) {
            t.Errorf(Sprintf!("IsSpace(%q) = true", r));
        }
    }
}}

test!{ fn TestIsUpperLower(t) {
    if !unicode::IsUpper('A' as u32 as i32) {
        t.Errorf(Sprintf!("IsUpper('A') = false"));
    }
    if unicode::IsUpper('a' as u32 as i32) {
        t.Errorf(Sprintf!("IsUpper('a') = true"));
    }
    if !unicode::IsLower('a' as u32 as i32) {
        t.Errorf(Sprintf!("IsLower('a') = false"));
    }
    if unicode::IsLower('A' as u32 as i32) {
        t.Errorf(Sprintf!("IsLower('A') = true"));
    }
}}

test!{ fn TestToUpper(t) {
    let cases: slice<(char, char)> = vec![('a', 'A'), ('z', 'Z'), ('A', 'A'), ('0', '0'), ('é', 'É')].into();
    for (inp, want) in cases {
        let got = unicode::ToUpper(inp as u32 as i32) as u32;
        if got != want as u32 {
            t.Errorf(Sprintf!("ToUpper(%q) = %q, want %q", inp, char::from_u32(got).unwrap_or('?'), want));
        }
    }
}}

test!{ fn TestToLower(t) {
    let cases: slice<(char, char)> = vec![('A', 'a'), ('Z', 'z'), ('a', 'a'), ('0', '0'), ('É', 'é')].into();
    for (inp, want) in cases {
        let got = unicode::ToLower(inp as u32 as i32) as u32;
        if got != want as u32 {
            t.Errorf(Sprintf!("ToLower(%q) = %q, want %q", inp, char::from_u32(got).unwrap_or('?'), want));
        }
    }
}}

test!{ fn TestIsPunct(t) {
    for r in ['.', ',', '!', '?', ';'] {
        if !unicode::IsPunct(r as u32 as i32) {
            t.Errorf(Sprintf!("IsPunct(%q) = false", r));
        }
    }
    for r in ['a', '0', ' '] {
        if unicode::IsPunct(r as u32 as i32) {
            t.Errorf(Sprintf!("IsPunct(%q) = true", r));
        }
    }
}}

test!{ fn TestIsControl(t) {
    for r in ['\0', '\t', '\n', '\r'] {
        if !unicode::IsControl(r as u32 as i32) {
            t.Errorf(Sprintf!("IsControl(%q) = false", r as u32 as i64));
        }
    }
    for r in ['a', '0', ' '] {
        if unicode::IsControl(r as u32 as i32) {
            t.Errorf(Sprintf!("IsControl(%q) = true", r));
        }
    }
}}
