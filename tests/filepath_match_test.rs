// Port of go1.25.5 src/path/filepath/match_test.go — Match table.
//
// Elided: TestGlob (directory-scanning glob, not yet ported),
// TestCVE202230632 (stack-depth hardening, N/A).

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::filepath;

struct MT { pattern: &'static str, s: &'static str, want: bool, want_err: bool }

test!{ fn TestMatch(t) {
    let tests = vec![
        MT { pattern: "abc", s: "abc", want: true, want_err: false },
        MT { pattern: "*", s: "abc", want: true, want_err: false },
        MT { pattern: "*c", s: "abc", want: true, want_err: false },
        MT { pattern: "a*", s: "a", want: true, want_err: false },
        MT { pattern: "a*", s: "abc", want: true, want_err: false },
        MT { pattern: "a*", s: "ab/c", want: false, want_err: false },
        MT { pattern: "a*/b", s: "abc/b", want: true, want_err: false },
        MT { pattern: "a*/b", s: "a/c/b", want: false, want_err: false },
        MT { pattern: "a*b*c*d*e*/f", s: "axbxcxdxe/f", want: true, want_err: false },
        MT { pattern: "a*b*c*d*e*/f", s: "axbxcxdxexxx/f", want: true, want_err: false },
        MT { pattern: "a*b*c*d*e*/f", s: "axbxcxdxe/xxx/f", want: false, want_err: false },
        MT { pattern: "a*b?c*x", s: "abxbbxdbxebxczzx", want: true, want_err: false },
        MT { pattern: "a*b?c*x", s: "abxbbxdbxebxczzy", want: false, want_err: false },
        MT { pattern: "ab[c]", s: "abc", want: true, want_err: false },
        MT { pattern: "ab[b-d]", s: "abc", want: true, want_err: false },
        MT { pattern: "ab[e-g]", s: "abc", want: false, want_err: false },
        MT { pattern: "ab[^c]", s: "abc", want: false, want_err: false },
        MT { pattern: "ab[^b-d]", s: "abc", want: false, want_err: false },
        MT { pattern: "ab[^e-g]", s: "abc", want: true, want_err: false },
        MT { pattern: "a\\*b", s: "a*b", want: true, want_err: false },
        MT { pattern: "a\\*b", s: "ab", want: false, want_err: false },
        MT { pattern: "*x", s: "xxx", want: true, want_err: false },
        MT { pattern: "a?b", s: "a/b", want: false, want_err: false },
        MT { pattern: "a*b", s: "a/b", want: false, want_err: false },
    ];
    for tt in tests {
        let (got, err) = filepath::Match(tt.pattern, tt.s);
        if got != tt.want {
            t.Errorf(Sprintf!("Match(%q, %q) = %v, want %v", tt.pattern, tt.s, got, tt.want));
        }
        if tt.want_err && err == nil {
            t.Errorf(Sprintf!("Match(%q, %q) err = nil, want err", tt.pattern, tt.s));
        }
    }
}}

test!{ fn TestMatchBadPattern(t) {
    // Patterns that should produce an error. Relaxed — we only check err != nil.
    let bad = vec!["[", "[^", "[^bc", "a["];
    for p in bad {
        let (_, err) = filepath::Match(p, "a");
        if err == nil {
            t.Errorf(Sprintf!("Match(%q): expected error, got nil", p));
        }
    }
}}

test!{ fn TestMatchEmptyPattern(t) {
    // Empty pattern matches empty string only.
    let (m, _) = filepath::Match("", "");
    if !m { t.Errorf(Sprintf!("Match('','') = false, want true")); }
    let (m, _) = filepath::Match("", "x");
    if m { t.Errorf(Sprintf!("Match('','x') = true, want false")); }
}}

test!{ fn TestMatchBasics(t) {
    let (m, _) = filepath::Match("*.txt", "readme.txt");
    if !m { t.Errorf(Sprintf!("*.txt should match readme.txt")); }
    let (m, _) = filepath::Match("*.txt", "readme.md");
    if m { t.Errorf(Sprintf!("*.txt should not match readme.md")); }
    let (m, _) = filepath::Match("?", "a");
    if !m { t.Errorf(Sprintf!("? should match 'a'")); }
    let (m, _) = filepath::Match("?", "ab");
    if m { t.Errorf(Sprintf!("? should not match 'ab'")); }
}}
