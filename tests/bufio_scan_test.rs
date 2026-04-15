// Port of go1.25.5 src/bufio/scan_test.go — Scanner Split variants.
//
// Elided: tests that rely on Go's function-type SplitFunc being wrapped as
// a closure capturing state (our SplitFunc is a bare fn pointer). Those
// tests exercise continuation tracking inside a custom Split closure and
// aren't representative of the common Scanner use.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::bufio;
use std::io::Cursor;

test!{ fn TestScanLines(t) {
    let cases = vec![
        ("", vec![]),
        ("a", vec!["a"]),
        ("hello", vec!["hello"]),
        ("one\ntwo\nthree", vec!["one", "two", "three"]),
        ("one\ntwo\nthree\n", vec!["one", "two", "three"]),
        ("crlf\r\nlines\r\n", vec!["crlf", "lines"]),
    ];
    for (input, want) in cases {
        let mut sc = bufio::NewScanner(Cursor::new(input.to_string()));
        sc.Split(bufio::ScanLines);
        let mut got: Vec<String> = Vec::new();
        while sc.Scan() { got.push(sc.Text().to_string()); }
        if sc.Err() != &nil { t.Errorf(Sprintf!("unexpected err: %s", sc.Err())); }
        if got != want {
            t.Errorf(Sprintf!("ScanLines(%q) got %v want %v", input, got.len(), want.len()));
        }
    }
}}

test!{ fn TestScanBytes(t) {
    let tests = vec!["", "a", "hello", "abcdefgh"];
    for input in tests {
        let mut sc = bufio::NewScanner(Cursor::new(input.to_string()));
        sc.Split(bufio::ScanBytes);
        let mut i = 0;
        while sc.Scan() {
            let b = sc.Bytes();
            if b.len() != 1 || b[0] != input.as_bytes()[i] {
                t.Errorf(Sprintf!("ScanBytes(%q)[%d] got len=%d", input, i, b.len()));
            }
            i += 1;
        }
        if i != input.len() {
            t.Errorf(Sprintf!("ScanBytes(%q) terminated at %d, want %d", input, i, input.len()));
        }
        if sc.Err() != &nil {
            t.Errorf(Sprintf!("ScanBytes(%q) err: %s", input, sc.Err()));
        }
    }
}}

test!{ fn TestScanRunes(t) {
    let tests = vec!["", "a", "abc", "¼", "☹", "abc¼☹日本語"];
    for input in tests {
        let mut sc = bufio::NewScanner(Cursor::new(input.to_string()));
        sc.Split(bufio::ScanRunes);
        let expected: Vec<char> = input.chars().collect();
        let mut rune_count = 0;
        while sc.Scan() {
            let got = sc.Text();
            let got_rune = got.chars().next().unwrap();
            if rune_count >= expected.len() {
                t.Errorf(Sprintf!("ScanRunes(%q) ran too long", input));
                break;
            }
            if got_rune != expected[rune_count] {
                t.Errorf(Sprintf!("ScanRunes(%q)[%d] got %q want %q",
                    input, rune_count, got_rune, expected[rune_count]));
            }
            rune_count += 1;
        }
        if sc.Err() != &nil {
            t.Errorf(Sprintf!("ScanRunes err: %s", sc.Err()));
        }
        if rune_count != expected.len() {
            t.Errorf(Sprintf!("ScanRunes(%q) got %d runes, want %d",
                input, rune_count, expected.len()));
        }
    }
}}

test!{ fn TestScanWords(t) {
    let tests = vec![
        ("", vec![]),
        (" ", vec![]),
        ("\n", vec![]),
        ("a", vec!["a"]),
        (" a ", vec!["a"]),
        ("abc def", vec!["abc", "def"]),
        (" abc def ", vec!["abc", "def"]),
        (" abc\tdef\nghi\rjkl mno pqr  ", vec!["abc", "def", "ghi", "jkl", "mno", "pqr"]),
    ];
    for (input, want) in tests {
        let mut sc = bufio::NewScanner(Cursor::new(input.to_string()));
        sc.Split(bufio::ScanWords);
        let mut got: Vec<String> = Vec::new();
        while sc.Scan() { got.push(sc.Text().to_string()); }
        if got.len() != want.len() || !got.iter().zip(&want).all(|(a, b)| a == b) {
            t.Errorf(Sprintf!("ScanWords(%q) got %v-count, want %v-count",
                input, got.len(), want.len()));
        }
    }
}}

test!{ fn TestScanLineTooLong(t) {
    // Build a long single line exceeding MaxTokenSize (256 for test).
    let long: String = std::iter::repeat('a').take(1000).collect();
    let input = format!("{}\n", long);
    let mut sc = bufio::NewScanner(Cursor::new(input));
    sc.Split(bufio::ScanLines);
    sc.MaxTokenSize(256);
    let mut ran = false;
    while sc.Scan() { ran = true; }
    if sc.Err() == &nil {
        t.Errorf(Sprintf!("expected ErrTooLong, got nil (ran=%v)", ran));
    } else {
        let es = format!("{}", sc.Err());
        if !strings::Contains(&es, "too long") {
            t.Errorf(Sprintf!("expected 'too long' error, got %s", es));
        }
    }
}}

test!{ fn TestScanNoNewline(t) {
    // A last line without trailing \n still emits a token.
    let input = "last-line-no-newline";
    let mut sc = bufio::NewScanner(Cursor::new(input.to_string()));
    sc.Split(bufio::ScanLines);
    let mut got: Vec<String> = Vec::new();
    while sc.Scan() { got.push(sc.Text().to_string()); }
    if got != vec!["last-line-no-newline"] {
        t.Errorf(Sprintf!("no-newline last line got %v", got.len()));
    }
}}

test!{ fn TestScanBlankLines(t) {
    // Blank lines produce empty tokens.
    let input = "one\n\ntwo\n\n\nthree";
    let mut sc = bufio::NewScanner(Cursor::new(input.to_string()));
    sc.Split(bufio::ScanLines);
    let mut got: Vec<String> = Vec::new();
    while sc.Scan() { got.push(sc.Text().to_string()); }
    let want = vec!["one", "", "two", "", "", "three"];
    if got != want {
        t.Errorf(Sprintf!("ScanLines(blank) got %d tokens, want %d", got.len(), want.len()));
    }
}}

test!{ fn TestIsSpace(t) {
    // bufio::IsSpace matches unicode::IsSpace.
    for r in ['\u{0020}', '\t', '\n', '\r', '\u{00A0}'] {
        if !bufio::IsSpace(r as i32) {
            t.Errorf(Sprintf!("IsSpace(%q) = false; want true", r));
        }
    }
    for r in ['a', '1', '_', '\0'] {
        if bufio::IsSpace(r as i32) {
            t.Errorf(Sprintf!("IsSpace(%q) = true; want false", r));
        }
    }
}}
