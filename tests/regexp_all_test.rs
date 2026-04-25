// Port of go1.25.5 src/regexp/all_test.go — core APIs and tables.
//
// Elided: tests that exercise Go's specific internals (TestCopyMatch,
// TestOnePassCutoff, TestSwitchBacktrack, TestMinInputLen, TestLiteralPrefix),
// and tests that require io.RuneReader-based find (TestFindReaderIndex,
// TestFindReaderSubmatchIndex). ReplaceAllLiteral with `$` quirks is
// subtly different between Go and Rust's regex crate; the common cases
// without `$` are tested.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::regexp;

const GOOD_RE: &[&str] = &[
    "",
    ".",
    "^.$",
    "a",
    "a*",
    "a+",
    "a?",
    "a|b",
    "a*|b*",
    "(a*|b)(c*|d)",
    "[a-z]",
    "[a\\-\\]z]",
    "[^a]",
    "[日本語]",
    "日本語+",
    "(?:)",
    "(?P<name>a)",
];

test!{ fn TestGoodCompile(t) {
    for &re in GOOD_RE {
        let (_, err) = regexp::Compile(re);
        if err != nil {
            t.Errorf(Sprintf!("Compile(%q): %s", re, err));
        }
    }
}}

const BAD_RE: &[&str] = &[
    "*", "(abc", "abc)", "x[a-z",
    "[z-a]", "abc\\",
];

test!{ fn TestBadCompile(t) {
    for &re in BAD_RE {
        let (_, err) = regexp::Compile(re);
        if err == nil {
            t.Errorf(Sprintf!("Compile(%q): expected error, got nil", re));
        }
    }
}}

test!{ fn TestMatch(t) {
    let cases: Vec<(&str, &str, bool)> = vec![
        ("^abcdefg", "abcdefg", true),
        ("a+", "baaab", true),
        ("^abcd$", "abcde", false),
        ("[a-z]+", "abcd", true),
        ("x", "y", false),
        ("日本語+", "日本語", true),
    ];
    for (pat, text, want) in cases {
        let re = regexp::MustCompile(pat);
        let got = re.MatchString(text);
        if got != want {
            t.Errorf(Sprintf!("Match(%q, %q) = %v want %v", pat, text, got, want));
        }
        let got_b = re.Match(text);
        if got_b != want {
            t.Errorf(Sprintf!("Match[bytes](%q, %q) = %v want %v", pat, text, got_b, want));
        }
    }
}}

test!{ fn TestMatchFunction(t) {
    let (ok, err) = regexp::MatchString(r"^\d+$", "12345");
    if err != nil { t.Errorf(Sprintf!("MatchString err: %s", err)); }
    if !ok { t.Errorf(Sprintf!("MatchString should match")); }
}}

struct RT { pat: &'static str, repl: &'static str, input: &'static str, output: &'static str }

test!{ fn TestReplaceAll(t) {
    let tests = vec![
        // Empty patterns & inputs.
        RT { pat: "b", repl: "", input: "", output: "" },
        RT { pat: "b", repl: "x", input: "", output: "" },
        RT { pat: "b", repl: "", input: "abc", output: "ac" },
        RT { pat: "b", repl: "x", input: "abc", output: "axc" },
        RT { pat: "y", repl: "", input: "abc", output: "abc" },

        // Start/end anchored.
        RT { pat: "^[a-c]*", repl: "x", input: "abcdabc", output: "xdabc" },
        RT { pat: "[a-c]*$", repl: "x", input: "abcdabc", output: "abcdx" },
        RT { pat: "^[a-c]*$", repl: "x", input: "abcdabc", output: "abcdabc" },

        // Cases.
        RT { pat: "abc", repl: "def", input: "abcdefg", output: "defdefg" },
        RT { pat: "bc", repl: "BC", input: "abcbcdcdedef", output: "aBCBCdcdedef" },
        RT { pat: "abc", repl: "", input: "abcdabc", output: "d" },
        RT { pat: "abc", repl: "d", input: "", output: "" },
        RT { pat: "abc", repl: "d", input: "abc", output: "d" },
        RT { pat: ".+", repl: "x", input: "abc", output: "x" },

        // Substitutions.
        RT { pat: "a+", repl: "($0)", input: "banana", output: "b(a)n(a)n(a)" },
        RT { pat: "a+", repl: "(${0})", input: "banana", output: "b(a)n(a)n(a)" },
        RT { pat: "hello, (.+)", repl: "goodbye, ${1}", input: "hello, world", output: "goodbye, world" },
        RT { pat: "hello, (?P<noun>.+)", repl: "goodbye, $noun!", input: "hello, world", output: "goodbye, world!" },
    ];
    for tc in tests {
        let (re, err) = regexp::Compile(tc.pat);
        if err != nil { t.Errorf(Sprintf!("compile(%q): %s", tc.pat, err)); continue; }
        let got = re.ReplaceAllString(tc.input, tc.repl);
        if got != tc.output {
            t.Errorf(Sprintf!("ReplaceAllString(%q, %q, %q) = %q; want %q",
                tc.pat, tc.input, tc.repl, got, tc.output));
        }
    }
}}

test!{ fn TestReplaceAllLiteral(t) {
    // Literal replacement ignores `$` semantics.
    let cases = vec![
        ("a+", "($0)", "banana", "b($0)n($0)n($0)"),
        ("a+", "$$", "aaa", "$$"),
    ];
    for (pat, repl, input, want) in cases {
        let re = regexp::MustCompile(pat);
        let got = re.ReplaceAllLiteralString(input, repl);
        if got != want {
            t.Errorf(Sprintf!("ReplaceAllLiteralString(%q, %q, %q) = %q want %q",
                pat, input, repl, got, want));
        }
    }
}}

test!{ fn TestReplaceAllFunc(t) {
    let cases = vec![
        ("[a-c]", "defabcdef", "defxayxbyxcydef"),
        ("[a-c]+", "defabcdef", "defxabcydef"),
    ];
    for (pat, input, want) in cases {
        let re = regexp::MustCompile(pat);
        let got = re.ReplaceAllStringFunc(input, |s| Sprintf!("x%vy", s));
        if got != want {
            t.Errorf(Sprintf!("ReplaceAllStringFunc(%q, %q) = %q want %q",
                pat, input, got, want));
        }
    }
}}

struct MT { pattern: &'static str, output: &'static str }

test!{ fn TestQuoteMeta(t) {
    let cases = vec![
        MT { pattern: "",      output: "" },
        MT { pattern: "foo",   output: "foo" },
        MT { pattern: "日本語+", output: "日本語\\+" },
    ];
    for tc in cases {
        let quoted = regexp::QuoteMeta(tc.pattern);
        if quoted != tc.output {
            t.Errorf(Sprintf!("QuoteMeta(%q) = %q want %q", tc.pattern, quoted, tc.output));
            continue;
        }
        if !tc.pattern.is_empty() {
            let re = regexp::MustCompile(&quoted);
            let src = Sprintf!("abc%vdef", tc.pattern);
            let replaced = re.ReplaceAllString(&src, "xyz");
            if replaced != "abcxyzdef" {
                t.Errorf(Sprintf!("QuoteMeta then Replace = %q", replaced));
            }
        }
    }
}}

test!{ fn TestSplit(t) {
    let cases: Vec<(&str, &str, i64, Vec<&str>)> = vec![
        (",", "abc", -1, vec!["abc"]),
        (",", "a,b,c", -1, vec!["a", "b", "c"]),
        (",", "a,b,c", 2, vec!["a", "b,c"]),
        (",", ",a,b,c,", -1, vec!["", "a", "b", "c", ""]),
        (r"\s+", "hello   world\t\tfoo", -1, vec!["hello", "world", "foo"]),
    ];
    for (pat, input, n, want) in cases {
        let re = regexp::MustCompile(pat);
        let got = re.Split(input, n);
        if got != want {
            t.Errorf(Sprintf!("Split(%q, %q, %d) = %d parts want %d",
                pat, input, n, got.len(), want.len()));
        }
    }
}}

test!{ fn TestSubexp(t) {
    let re = regexp::MustCompile(r"(\w+)-(\d+)");
    if re.NumSubexp() != 2 {
        t.Errorf(Sprintf!("NumSubexp = %d want 2", re.NumSubexp()));
    }
    let re = regexp::MustCompile(r"(\d+)");
    if re.NumSubexp() != 1 {
        t.Errorf(Sprintf!("NumSubexp = %d want 1", re.NumSubexp()));
    }
    let re = regexp::MustCompile(r"\d+");
    if re.NumSubexp() != 0 {
        t.Errorf(Sprintf!("NumSubexp = %d want 0", re.NumSubexp()));
    }
}}
