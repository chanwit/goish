// port of go/src/path/path_test.go
//
// This is the first real Go-test-to-goish port, and the design fixture for
// the `testing::T` + `test!` + `Struct!` framework. Line-by-line correspondence
// with the Go source is the explicit design goal.

use goish::prelude::*;

// Go: type PathTest struct { path, result string }
Struct!{ type PathTest struct { path, result string } }

// Go: var cleantests = []PathTest{ {"", "."}, ... }
fn cleantests() -> slice<PathTest> { slice!([]PathTest{
    // Already clean
    PathTest!("",                               "."),
    PathTest!("abc",                            "abc"),
    PathTest!("abc/def",                        "abc/def"),
    PathTest!("a/b/c",                          "a/b/c"),
    PathTest!(".",                              "."),
    PathTest!("..",                             ".."),
    PathTest!("../..",                          "../.."),
    PathTest!("../../abc",                      "../../abc"),
    PathTest!("/abc",                           "/abc"),
    PathTest!("/",                              "/"),

    // Remove trailing slash
    PathTest!("abc/",                           "abc"),
    PathTest!("abc/def/",                       "abc/def"),
    PathTest!("a/b/c/",                         "a/b/c"),
    PathTest!("./",                             "."),
    PathTest!("../",                            ".."),
    PathTest!("/abc/",                          "/abc"),

    // Remove doubled slash
    PathTest!("abc//def//ghi",                  "abc/def/ghi"),
    PathTest!("//abc",                          "/abc"),
    PathTest!("///abc",                         "/abc"),
    PathTest!("//abc//",                        "/abc"),
    PathTest!("abc//",                          "abc"),

    // Remove . elements
    PathTest!("abc/./def",                      "abc/def"),
    PathTest!("/./abc/def",                     "/abc/def"),
    PathTest!("abc/.",                          "abc"),

    // Remove .. elements
    PathTest!("abc/def/ghi/../jkl",             "abc/def/jkl"),
    PathTest!("abc/def/../ghi/../jkl",          "abc/jkl"),
    PathTest!("abc/def/..",                     "abc"),
    PathTest!("abc/def/../..",                  "."),
    PathTest!("/abc/def/../..",                 "/"),

    // Combinations
    PathTest!("abc/./../def",                   "def"),
    PathTest!("abc//./../def",                  "def"),
    PathTest!("abc/../../././../def",           "../../def"),
})}

test!{ fn TestClean(t) {
    for test in &cleantests() {
        let s = path::Clean(&test.path);
        if s != test.result {
            t.Errorf(Sprintf!("Clean(%q) = %q, want %q", test.path, s, test.result));
        }
        let s = path::Clean(&test.result);
        if s != test.result {
            t.Errorf(Sprintf!("Clean(%q) = %q, want %q", test.result, s, test.result));
        }
    }
}}

// ── TestSplit ──────────────────────────────────────────────────────────

Struct!{ type SplitTest struct { path, dir, file string } }

test!{ fn TestSplit(t) {
    let splittests = slice!([]SplitTest{
        SplitTest!("a/b",  "a/",   "b"),
        SplitTest!("a/b/", "a/b/", ""),
        SplitTest!("a/",   "a/",   ""),
        SplitTest!("a",    "",     "a"),
        SplitTest!("/",    "/",    ""),
    });
    for test in &splittests {
        let (d, f) = path::Split(&test.path);
        if d != test.dir || f != test.file {
            t.Errorf(Sprintf!("Split(%q) = %q, %q, want %q, %q",
                test.path, d, f, test.dir, test.file));
        }
    }
}}

// ── TestExt ────────────────────────────────────────────────────────────

Struct!{ type ExtTest struct { path, ext string } }

test!{ fn TestExt(t) {
    let exttests = slice!([]ExtTest{
        ExtTest!("path.go",    ".go"),
        ExtTest!("path.pb.go", ".go"),
        ExtTest!("a.dir/b",    ""),
        ExtTest!("a.dir/b.go", ".go"),
        ExtTest!("a.dir/",     ""),
    });
    for test in &exttests {
        let x = path::Ext(&test.path);
        if x != test.ext {
            t.Errorf(Sprintf!("Ext(%q) = %q, want %q", test.path, x, test.ext));
        }
    }
}}

// ── TestBase ───────────────────────────────────────────────────────────

fn basetests() -> slice<PathTest> { slice!([]PathTest{
    PathTest!("",           "."),
    PathTest!(".",          "."),
    PathTest!("/.",         "."),
    PathTest!("/",          "/"),
    PathTest!("////",       "/"),
    PathTest!("x/",         "x"),
    PathTest!("abc",        "abc"),
    PathTest!("abc/def",    "def"),
    PathTest!("a/b/.x",     ".x"),
    PathTest!("a/b/c.",     "c."),
    PathTest!("a/b/c.x",    "c.x"),
})}

test!{ fn TestBase(t) {
    for test in &basetests() {
        let s = path::Base(&test.path);
        if s != test.result {
            t.Errorf(Sprintf!("Base(%q) = %q, want %q", test.path, s, test.result));
        }
    }
}}

// ── TestDir ────────────────────────────────────────────────────────────

fn dirtests() -> slice<PathTest> { slice!([]PathTest{
    PathTest!("",           "."),
    PathTest!(".",          "."),
    PathTest!("/.",         "/"),
    PathTest!("/",          "/"),
    PathTest!("////",       "/"),
    PathTest!("/foo",       "/"),
    PathTest!("x/",         "x"),
    PathTest!("abc",        "."),
    PathTest!("abc/def",    "abc"),
    PathTest!("a/b/.x",     "a/b"),
    PathTest!("a/b/c.",     "a/b"),
    PathTest!("a/b/c.x",    "a/b"),
})}

test!{ fn TestDir(t) {
    for test in &dirtests() {
        let s = path::Dir(&test.path);
        if s != test.result {
            t.Errorf(Sprintf!("Dir(%q) = %q, want %q", test.path, s, test.result));
        }
    }
}}

// ── TestIsAbs ──────────────────────────────────────────────────────────

Struct!{ type IsAbsTest struct { path string; isAbs bool } }

test!{ fn TestIsAbs(t) {
    let isAbsTests = slice!([]IsAbsTest{
        IsAbsTest!("",             false),
        IsAbsTest!("/",            true),
        IsAbsTest!("/usr/bin/gcc", true),
        IsAbsTest!("..",           false),
        IsAbsTest!("/a/../bb",     true),
        IsAbsTest!(".",            false),
        IsAbsTest!("./",           false),
        IsAbsTest!("lala",         false),
    });
    for test in &isAbsTests {
        let r = path::IsAbs(&test.path);
        if r != test.isAbs {
            t.Errorf(Sprintf!("IsAbs(%q) = %v, want %v", test.path, r, test.isAbs));
        }
    }
}}

// ── BenchmarkJoin — from path_test.go ─────────────────────────────────

benchmark!{ fn BenchmarkJoin(b) {
    b.ReportAllocs();
    let parts = slice!([]string{"one", "two", "three", "four"});
    while b.Loop() {
        let _ = std::hint::black_box(path::Join(&parts));
    }
}}
