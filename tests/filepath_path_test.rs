// Port of go1.25.5 src/path/filepath/path_test.go — Unix-side subset.
//
// Elided: Windows-specific tables (wincleantests, winjointests, winsplittests,
// winislocaltests) — goish currently only supports the host OS's SEP, tested
// here on Unix. TestWalk/TestWalkDir/TestEvalSymlinks — require Walk + symlink
// infrastructure not yet ported. testing.AllocsPerRun alloc-count checks —
// runtime-specific and orthogonal to semantic correctness.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::filepath;

struct PathTest { path: &'static str, result: &'static str }

fn clean_tests() -> slice<PathTest> { vec![
    // Already clean
    PathTest { path: "abc", result: "abc" },
    PathTest { path: "abc/def", result: "abc/def" },
    PathTest { path: "a/b/c", result: "a/b/c" },
    PathTest { path: ".", result: "." },
    PathTest { path: "..", result: ".." },
    PathTest { path: "../..", result: "../.." },
    PathTest { path: "../../abc", result: "../../abc" },
    PathTest { path: "/abc", result: "/abc" },
    PathTest { path: "/", result: "/" },
    // Empty is current dir
    PathTest { path: "", result: "." },
    // Remove trailing slash
    PathTest { path: "abc/", result: "abc" },
    PathTest { path: "abc/def/", result: "abc/def" },
    PathTest { path: "./", result: "." },
    PathTest { path: "../", result: ".." },
    // Remove doubled slash
    PathTest { path: "abc//def//ghi", result: "abc/def/ghi" },
    PathTest { path: "abc//", result: "abc" },
    // Remove . elements
    PathTest { path: "abc/./def", result: "abc/def" },
    PathTest { path: "/./abc/def", result: "/abc/def" },
    PathTest { path: "abc/.", result: "abc" },
    // Remove .. elements
    PathTest { path: "abc/def/ghi/../jkl", result: "abc/def/jkl" },
    PathTest { path: "abc/def/../ghi/../jkl", result: "abc/jkl" },
    PathTest { path: "abc/def/..", result: "abc" },
    PathTest { path: "abc/def/../..", result: "." },
    PathTest { path: "/abc/def/../..", result: "/" },
    PathTest { path: "abc/def/../../..", result: ".." },
    PathTest { path: "/abc/def/../../..", result: "/" },
    PathTest { path: "abc/def/../../../ghi/jkl/../../../mno", result: "../../mno" },
    PathTest { path: "/../abc", result: "/abc" },
    // Combinations
    PathTest { path: "abc/./../def", result: "def" },
    PathTest { path: "abc//./../def", result: "def" },
    PathTest { path: "abc/../../././../def", result: "../../def" },
].into()}

test!{ fn TestClean(t) {
    for test in clean_tests() {
        let s = filepath::Clean(test.path);
        if s != test.result {
            t.Errorf(Sprintf!("Clean(%q) = %q, want %q", test.path, s, test.result));
        }
        // Idempotence: Clean(result) == result.
        let s2 = filepath::Clean(test.result);
        if s2 != test.result {
            t.Errorf(Sprintf!("Clean(%q) = %q, want %q (idempotence)", test.result, s2, test.result));
        }
    }
}}

struct IsLocalTest { path: &'static str, is_local: bool }

test!{ fn TestIsLocal(t) {
    let tests = vec![
        IsLocalTest { path: "", is_local: false },
        IsLocalTest { path: ".", is_local: true },
        IsLocalTest { path: "..", is_local: false },
        IsLocalTest { path: "../a", is_local: false },
        IsLocalTest { path: "/", is_local: false },
        IsLocalTest { path: "/a", is_local: false },
        IsLocalTest { path: "/a/../..", is_local: false },
        IsLocalTest { path: "a", is_local: true },
        IsLocalTest { path: "a/../a", is_local: true },
        IsLocalTest { path: "a/", is_local: true },
        IsLocalTest { path: "a/.", is_local: true },
        IsLocalTest { path: "a/./b/./c", is_local: true },
    ];
    for tt in &tests {
        let got = filepath::IsLocal(tt.path);
        if got != tt.is_local {
            t.Errorf(Sprintf!("IsLocal(%q) = %v, want %v", tt.path, got, tt.is_local));
        }
    }
}}

struct SlashTest { in_: &'static str, out: &'static str }

test!{ fn TestFromAndToSlash(t) {
    // On Unix, both are identity.
    let tests = vec![
        SlashTest { in_: "", out: "" },
        SlashTest { in_: "/", out: "/" },
        SlashTest { in_: "a/b/c", out: "a/b/c" },
    ];
    for tt in tests {
        let from = filepath::FromSlash(tt.in_);
        if from != tt.out {
            t.Errorf(Sprintf!("FromSlash(%q) = %q, want %q", tt.in_, from, tt.out));
        }
        let to = filepath::ToSlash(tt.in_);
        if to != tt.out {
            t.Errorf(Sprintf!("ToSlash(%q) = %q, want %q", tt.in_, to, tt.out));
        }
    }
}}

struct SplitListT { in_: &'static str, out: Vec<&'static str> }

test!{ fn TestSplitList(t) {
    let tests = vec![
        SplitListT { in_: "", out: vec![] },
        SplitListT { in_: "a", out: vec!["a"] },
        SplitListT { in_: "a:b:c", out: vec!["a", "b", "c"] },
        SplitListT { in_: "/usr/bin:/bin:/usr/sbin", out: vec!["/usr/bin", "/bin", "/usr/sbin"] },
    ];
    for tt in tests {
        let got = filepath::SplitList(tt.in_);
        if got != tt.out {
            t.Errorf(Sprintf!("SplitList(%q) got %d, want %d", tt.in_, got.len(), tt.out.len()));
        }
    }
}}

struct SplitT { in_: &'static str, want_dir: &'static str, want_file: &'static str }

test!{ fn TestSplit(t) {
    let tests = vec![
        SplitT { in_: "a/b", want_dir: "a/", want_file: "b" },
        SplitT { in_: "a/b/", want_dir: "a/b/", want_file: "" },
        SplitT { in_: "a/", want_dir: "a/", want_file: "" },
        SplitT { in_: "a", want_dir: "", want_file: "a" },
        SplitT { in_: "/", want_dir: "/", want_file: "" },
        SplitT { in_: "", want_dir: "", want_file: "" },
    ];
    for tt in tests {
        let (d, f) = filepath::Split(tt.in_);
        if d != tt.want_dir || f != tt.want_file {
            t.Errorf(Sprintf!("Split(%q) = (%q, %q), want (%q, %q)",
                tt.in_, d, f, tt.want_dir, tt.want_file));
        }
    }
}}

struct JoinT { elem: Vec<&'static str>, path: &'static str }

test!{ fn TestJoin(t) {
    let tests = vec![
        JoinT { elem: vec![], path: "" },
        JoinT { elem: vec![""], path: "" },
        JoinT { elem: vec!["a"], path: "a" },
        JoinT { elem: vec!["a", "b"], path: "a/b" },
        JoinT { elem: vec!["a", ""], path: "a" },
        JoinT { elem: vec!["", "b"], path: "b" },
        JoinT { elem: vec!["/", "a"], path: "/a" },
        JoinT { elem: vec!["/", "a/b"], path: "/a/b" },
        JoinT { elem: vec!["/", ""], path: "/" },
        JoinT { elem: vec!["//", "a"], path: "/a" },
        JoinT { elem: vec!["/a", "b"], path: "/a/b" },
        JoinT { elem: vec!["a/", "b"], path: "a/b" },
        JoinT { elem: vec!["a/", ""], path: "a" },
        JoinT { elem: vec!["", ""], path: "" },
    ];
    for tt in tests {
        let got = filepath::Join(&tt.elem);
        if got != tt.path {
            t.Errorf(Sprintf!("Join(%v) = %q, want %q", tt.elem.len(), got, tt.path));
        }
    }
}}

struct ExtT { path: &'static str, ext: &'static str }

test!{ fn TestExt(t) {
    let tests = vec![
        ExtT { path: "", ext: "" },
        ExtT { path: ".", ext: "" },
        ExtT { path: "a.x", ext: ".x" },
        ExtT { path: "x.tar.gz", ext: ".gz" },
        ExtT { path: "/a/b/c.d.e", ext: ".e" },
    ];
    for tt in tests {
        let got = filepath::Ext(tt.path);
        if got != tt.ext {
            t.Errorf(Sprintf!("Ext(%q) = %q, want %q", tt.path, got, tt.ext));
        }
    }
}}

test!{ fn TestIsAbs(t) {
    let cases: Vec<(&str, bool)> = vec![
        ("/a", true),
        ("a", false),
        ("", false),
        ("/", true),
        ("./a", false),
    ];
    for (p, want) in cases {
        let got = filepath::IsAbs(p);
        if got != want {
            t.Errorf(Sprintf!("IsAbs(%q) = %v, want %v", p, got, want));
        }
    }
}}
