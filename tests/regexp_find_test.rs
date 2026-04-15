// Port of go1.25.5 src/regexp/find_test.go — Find/FindAll/Submatch subset.
//
// The Go findTests table contains cases that exercise Go's specific RE2 engine
// and [\xff]-style invalid-UTF-8 bytes. Our backend (the `regex` crate) operates
// on UTF-8 strings and doesn't accept invalid bytes. We port the UTF-8-safe
// subset. Cases with \a / \v / literal \xff bytes, or the distinctive quirks
// of Go's "empty-match at word boundary" handling at \b/\B are elided.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::regexp;

struct FT { pat: &'static str, text: &'static str, want: &'static str }

test!{ fn TestFindString(t) {
    let tests = vec![
        FT { pat: "",          text: "",       want: "" },
        FT { pat: "^abcdefg",  text: "abcdefg", want: "abcdefg" },
        FT { pat: "a+",        text: "baaab",   want: "aaa" },
        FT { pat: "abcd..",    text: "abcdef",  want: "abcdef" },
        FT { pat: "a",         text: "a",       want: "a" },
        FT { pat: "b",         text: "abc",     want: "b" },
        FT { pat: ".",         text: "a",       want: "a" },
        FT { pat: ".*",        text: "abcdef",  want: "abcdef" },
        FT { pat: "[a-z]+",    text: "abcd",    want: "abcd" },
        FT { pat: "[^a-z]+",   text: "ab1234cd", want: "1234" },
        FT { pat: "[日本語]+",  text: "日本語日本語", want: "日本語日本語" },
        FT { pat: "日本語+",    text: "日本語",   want: "日本語" },
        FT { pat: "ab$",       text: "cab",     want: "ab" },
        FT { pat: "axxb$",     text: "axxcb",   want: "" },  // no match
        FT { pat: "data",      text: "daXY data", want: "data" },
        FT { pat: "zx+",       text: "zzx",     want: "zx" },
        FT { pat: "ab$",       text: "abcab",   want: "ab" },
    ];
    for tt in tests {
        let re = regexp::MustCompile(tt.pat);
        let got = re.FindString(tt.text);
        if got != tt.want {
            t.Errorf(Sprintf!("FindString(%q, %q) = %q, want %q", tt.pat, tt.text, got, tt.want));
        }
    }
}}

struct FAT { pat: &'static str, text: &'static str, want: Vec<&'static str> }

test!{ fn TestFindAllString(t) {
    let tests = vec![
        FAT { pat: ".",   text: "abc",    want: vec!["a", "b", "c"] },
        FAT { pat: "ab*", text: "abbaab", want: vec!["abb", "a", "ab"] },
        FAT { pat: "\\d+", text: "a1b22c333", want: vec!["1", "22", "333"] },
        FAT { pat: "x",   text: "y",      want: vec![] },
    ];
    for tt in tests {
        let re = regexp::MustCompile(tt.pat);
        let got = re.FindAllString(tt.text, -1);
        if got.len() != tt.want.len() || !got.iter().zip(&tt.want).all(|(a, b)| a == b) {
            t.Errorf(Sprintf!("FindAllString(%q, %q) = %d matches want %d",
                tt.pat, tt.text, got.len(), tt.want.len()));
        }
    }
}}

struct FSI { pat: &'static str, text: &'static str, want: Vec<i64> }

test!{ fn TestFindStringIndex(t) {
    let tests = vec![
        FSI { pat: "a+",     text: "baaab",   want: vec![1, 4] },
        FSI { pat: "abcd",   text: "abcdef",  want: vec![0, 4] },
        FSI { pat: "x",      text: "y",       want: vec![] },
        FSI { pat: "ab$",    text: "cab",     want: vec![1, 3] },
    ];
    for tt in tests {
        let re = regexp::MustCompile(tt.pat);
        let got = re.FindStringIndex(tt.text);
        if got != tt.want {
            t.Errorf(Sprintf!("FindStringIndex(%q, %q) = %d pairs, want %d",
                tt.pat, tt.text, got.len(), tt.want.len()));
        }
    }
}}

struct FSS { pat: &'static str, text: &'static str, want: Vec<&'static str> }

test!{ fn TestFindStringSubmatch(t) {
    let tests = vec![
        FSS { pat: "(a)",     text: "a",      want: vec!["a", "a"] },
        FSS { pat: "(.)(.)", text: "日a",     want: vec!["日a", "日", "a"] },
        FSS { pat: "(.*)",    text: "",       want: vec!["", ""] },
        FSS { pat: "(.*)",    text: "abcd",   want: vec!["abcd", "abcd"] },
        FSS { pat: "(..)(..)", text: "abcd",  want: vec!["abcd", "ab", "cd"] },
        FSS { pat: "(\\w+)=(\\d+)", text: "port=8080", want: vec!["port=8080", "port", "8080"] },
    ];
    for tt in tests {
        let re = regexp::MustCompile(tt.pat);
        let got = re.FindStringSubmatch(tt.text);
        if got.len() != tt.want.len() || !got.iter().zip(&tt.want).all(|(a, b)| a == b) {
            t.Errorf(Sprintf!("FindStringSubmatch(%q, %q) got %d want %d",
                tt.pat, tt.text, got.len(), tt.want.len()));
        }
    }
}}

struct FASS { pat: &'static str, text: &'static str, want: Vec<Vec<&'static str>> }

test!{ fn TestFindAllStringSubmatch(t) {
    let tests = vec![
        FASS { pat: "(\\w+)=(\\d+)", text: "a=1 b=2",
               want: vec![vec!["a=1", "a", "1"], vec!["b=2", "b", "2"]] },
    ];
    for tt in tests {
        let re = regexp::MustCompile(tt.pat);
        let got = re.FindAllStringSubmatch(tt.text, -1);
        if got.len() != tt.want.len() {
            t.Errorf(Sprintf!("FindAllStringSubmatch(%q, %q) groups = %d want %d",
                tt.pat, tt.text, got.len(), tt.want.len()));
            continue;
        }
        for (i, g) in got.iter().enumerate() {
            if g.len() != tt.want[i].len() {
                t.Errorf(Sprintf!("match %d group count mismatch", i));
                continue;
            }
            for (j, v) in g.iter().enumerate() {
                if v != tt.want[i][j] {
                    t.Errorf(Sprintf!("match %d group %d = %q want %q", i, j, v, tt.want[i][j]));
                }
            }
        }
    }
}}
