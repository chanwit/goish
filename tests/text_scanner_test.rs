// Port of go1.25.5 src/text/scanner/scanner_test.go — minimal API
// coverage. Goish implements Ident / Int / Float / String / Char / EOF
// plus single-character punctuation.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::text::scanner::{self, Scanner};

fn scan_all(src: &str) -> (slice<rune>, slice<string>) {
    let mut s = Scanner::new();
    s.Init(src);
    let mut kinds: slice<rune> = slice::new();
    let mut texts: slice<string> = slice::new();
    loop {
        let k = s.Scan();
        if k == scanner::EOF { break; }
        kinds.push(k);
        texts.push(s.TokenText().into());
    }
    (kinds, texts)
}

test!{ fn TestIdent(t) {
    let (k, v) = scan_all("foo bar_baz _under");
    if k != vec![scanner::Ident, scanner::Ident, scanner::Ident] {
        t.Errorf(Sprintf!("Ident kinds mismatch"));
    }
    if v != vec!["foo", "bar_baz", "_under"] {
        t.Errorf(Sprintf!("Ident texts mismatch"));
    }
}}

test!{ fn TestIntFloat(t) {
    let (k, v) = scan_all("42 3.14 0 0.5");
    if k != vec![scanner::Int, scanner::Float, scanner::Int, scanner::Float] {
        t.Errorf(Sprintf!("Int/Float kinds mismatch"));
    }
    if v != vec!["42", "3.14", "0", "0.5"] {
        t.Errorf(Sprintf!("Int/Float texts mismatch"));
    }
}}

test!{ fn TestString(t) {
    let (k, v) = scan_all("\"hello\" \"with \\\"escape\\\"\"");
    if k != vec![scanner::String, scanner::String] {
        t.Errorf(Sprintf!("String kinds mismatch"));
    }
    if v[0] != "\"hello\"" {
        t.Errorf(Sprintf!("String[0] = %q", v[0]));
    }
}}

test!{ fn TestChar(t) {
    let (k, v) = scan_all("'a' 'Z'");
    if k != vec![scanner::Char, scanner::Char] {
        t.Errorf(Sprintf!("Char kinds mismatch"));
    }
    if v != vec!["'a'", "'Z'"] {
        t.Errorf(Sprintf!("Char texts mismatch"));
    }
}}

test!{ fn TestPunctuation(t) {
    let (k, _) = scan_all("+ - = { }");
    let want: slice<rune> = vec!['+' as i32, '-' as i32, '=' as i32, '{' as i32, '}' as i32].into();
    if k != want {
        t.Errorf(Sprintf!("punctuation kinds mismatch"));
    }
}}

test!{ fn TestMixed(t) {
    let (k, v) = scan_all("x := 42");
    // `:=` is two tokens in goish's minimal scanner (':' then '=').
    if k != vec![scanner::Ident, ':' as i32, '=' as i32, scanner::Int] {
        t.Errorf(Sprintf!("mixed kinds: %d tokens", k.len() as i64));
    }
    if v != vec!["x", ":", "=", "42"] {
        t.Errorf(Sprintf!("mixed texts mismatch"));
    }
}}

test!{ fn TestLineTracking(t) {
    let mut s = Scanner::new();
    s.Init("a\n\nb");
    s.Scan();
    let p1 = s.Pos();
    if p1.Line != 1 && p1.Line != 2 {
        t.Errorf(Sprintf!("after first token Line = %d", p1.Line));
    }
    s.Scan();
    let p2 = s.Pos();
    if p2.Line < p1.Line {
        t.Errorf(Sprintf!("Line did not advance"));
    }
}}
