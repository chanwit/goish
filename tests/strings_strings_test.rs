// Partial port of go1.25.5/src/strings/strings_test.go.
//
// Go's strings_test.go runs ~2200 lines against ~40 test functions.
// v0.6.0 ports the highest-leverage table-driven tests (Index, LastIndex,
// IndexAny, Contains/ContainsAny, Count, Trim family, Repeat, Replace,
// Cut/CutPrefix/CutSuffix, EqualFold). Remaining tests — FieldsFunc,
// Map on multi-rune strings, ToValidUTF8, Reader/Builder — live in
// v0.6.x follow-ups (already tracked in issues).
//
// Some upstream cases use invalid UTF-8 byte sequences that Rust's `&str`
// can't hold verbatim; those are elided with a note.

#![allow(non_snake_case)]
use goish::prelude::*;

struct IxT { s: &'static str, sep: &'static str, out: i64 }

fn indexTests() -> Vec<IxT> { vec![
    IxT { s: "",        sep: "",    out: 0 },
    IxT { s: "",        sep: "a",   out: -1 },
    IxT { s: "",        sep: "foo", out: -1 },
    IxT { s: "fo",      sep: "foo", out: -1 },
    IxT { s: "foo",     sep: "foo", out: 0 },
    IxT { s: "oofofoofooo", sep: "f",   out: 2 },
    IxT { s: "oofofoofooo", sep: "foo", out: 4 },
    IxT { s: "barfoobarfoo", sep: "foo", out: 3 },
    IxT { s: "foo",     sep: "",    out: 0 },
    IxT { s: "foo",     sep: "o",   out: 1 },
    IxT { s: "abcABCabc", sep: "A", out: 3 },
    IxT { s: "x",       sep: "x",   out: 0 },
    IxT { s: "abc",     sep: "a",   out: 0 },
    IxT { s: "abc",     sep: "b",   out: 1 },
    IxT { s: "abc",     sep: "c",   out: 2 },
    IxT { s: "abc",     sep: "x",   out: -1 },
    IxT { s: "ab",      sep: "ab",  out: 0 },
    IxT { s: "xab",     sep: "ab",  out: 1 },
    IxT { s: "abcd",    sep: "abcd", out: 0 },
    IxT { s: "xabcd",   sep: "abcd", out: 1 },
    IxT { s: "xabcqq",  sep: "abcqq", out: 1 },
    IxT { s: "01234567", sep: "01234567", out: 0 },
    IxT { s: "x01234567", sep: "01234567", out: 1 },
    IxT { s: "oxoxoxoxoxoxoxoxoxoxoxoy", sep: "oy", out: 22 },
    IxT { s: "oxoxoxoxoxoxoxoxoxoxoxox", sep: "oy", out: -1 },
    IxT { s: "oxoxoxoxoxoxoxoxoxoxox☺", sep: "☺", out: 22 },
]}

test!{ fn TestIndex(t) {
    for tc in indexTests() {
        let got = strings::Index(tc.s, tc.sep);
        if got != tc.out {
            t.Errorf(Sprintf!("Index(%q, %q) = %d; want %d", tc.s, tc.sep, got, tc.out));
        }
    }
}}

fn lastIndexTests() -> Vec<IxT> { vec![
    IxT { s: "",    sep: "",    out: 0 },
    IxT { s: "",    sep: "a",   out: -1 },
    IxT { s: "",    sep: "foo", out: -1 },
    IxT { s: "fo",  sep: "foo", out: -1 },
    IxT { s: "foo", sep: "foo", out: 0 },
    IxT { s: "foo", sep: "f",   out: 0 },
    IxT { s: "oofofoofooo", sep: "f",   out: 7 },
    IxT { s: "oofofoofooo", sep: "foo", out: 7 },
    IxT { s: "barfoobarfoo", sep: "foo", out: 9 },
    IxT { s: "foo", sep: "",    out: 3 },
    IxT { s: "foo", sep: "o",   out: 2 },
    IxT { s: "abcABCabc", sep: "A", out: 3 },
    IxT { s: "abcABCabc", sep: "a", out: 6 },
]}

test!{ fn TestLastIndex(t) {
    for tc in lastIndexTests() {
        let got = strings::LastIndex(tc.s, tc.sep);
        if got != tc.out {
            t.Errorf(Sprintf!("LastIndex(%q, %q) = %d; want %d", tc.s, tc.sep, got, tc.out));
        }
    }
}}

fn indexAnyTests() -> Vec<IxT> { vec![
    IxT { s: "",        sep: "",     out: -1 },
    IxT { s: "",        sep: "a",    out: -1 },
    IxT { s: "",        sep: "abc",  out: -1 },
    IxT { s: "a",       sep: "",     out: -1 },
    IxT { s: "a",       sep: "a",    out: 0 },
    // Cases with invalid UTF-8 bytes elided — Rust's &str can't hold them.
    IxT { s: "aaa",     sep: "a",    out: 0 },
    IxT { s: "abc",     sep: "xyz",  out: -1 },
    IxT { s: "abc",     sep: "xcz",  out: 2 },
    IxT { s: "ab☺c",    sep: "x☺yz", out: 2 },
    IxT { s: "a☺b☻c☹d", sep: "cx",   out: 8 },
    IxT { s: "aRegExp*", sep: ".(|)*+?^$[]", out: 7 },
]}

test!{ fn TestIndexAny(t) {
    for tc in indexAnyTests() {
        let got = strings::IndexAny(tc.s, tc.sep);
        if got != tc.out {
            t.Errorf(Sprintf!("IndexAny(%q, %q) = %d; want %d", tc.s, tc.sep, got, tc.out));
        }
    }
}}

struct SC { s: &'static str, sub: &'static str, out: bool }

fn containsTests() -> Vec<SC> { vec![
    SC { s: "abc",     sub: "bc",  out: true  },
    SC { s: "abc",     sub: "bcd", out: false },
    SC { s: "abc",     sub: "",    out: true  },
    SC { s: "",        sub: "a",   out: false },
    SC { s: "\u{263A}",     sub: "\u{263A}", out: true  }, // ☺ / ☺
]}

test!{ fn TestContains(t) {
    for tc in containsTests() {
        let got = strings::Contains(tc.s, tc.sub);
        if got != tc.out {
            t.Errorf(Sprintf!("Contains(%q, %q) = %v; want %v", tc.s, tc.sub, got, tc.out));
        }
    }
}}

struct CA { s: &'static str, chars: &'static str, out: bool }

fn containsAnyTests() -> Vec<CA> { vec![
    CA { s: "",     chars: "",     out: false },
    CA { s: "",     chars: "a",    out: false },
    CA { s: "",     chars: "abc",  out: false },
    CA { s: "a",    chars: "",     out: false },
    CA { s: "a",    chars: "a",    out: true  },
    CA { s: "aaa",  chars: "a",    out: true  },
    CA { s: "abc",  chars: "xyz",  out: false },
    CA { s: "abc",  chars: "xcz",  out: true  },
    CA { s: "a☺b☻c☹d", chars: "uvw☻xyz", out: true },
    CA { s: "aRegExp*", chars: ".(|)*+?^$[]", out: true },
    CA { s: "1....2....3....4", chars: "xyz", out: false },
]}

test!{ fn TestContainsAny(t) {
    for tc in containsAnyTests() {
        let got = strings::ContainsAny(tc.s, tc.chars);
        if got != tc.out {
            t.Errorf(Sprintf!("ContainsAny(%q, %q) = %v; want %v", tc.s, tc.chars, got, tc.out));
        }
    }
}}

struct CT { s: &'static str, sub: &'static str, out: i64 }

fn countTests() -> Vec<CT> { vec![
    CT { s: "",     sub: "",    out: 1 },
    CT { s: "",     sub: "notempty", out: 0 },
    CT { s: "notempty", sub: "", out: 9 }, // len+1
    CT { s: "smaller", sub: "not smaller", out: 0 },
    CT { s: "12345678987654321", sub: "6", out: 2 },
    CT { s: "611161116",          sub: "6", out: 3 },
    CT { s: "notequal", sub: "NotEqual", out: 0 },
    CT { s: "equal",    sub: "equal",    out: 1 },
    CT { s: "abc1231231123q", sub: "123", out: 3 },
    CT { s: "11111", sub: "11", out: 2 },
]}

test!{ fn TestCount(t) {
    for tc in countTests() {
        let got = strings::Count(tc.s, tc.sub);
        if got != tc.out {
            t.Errorf(Sprintf!("Count(%q, %q) = %d; want %d", tc.s, tc.sub, got, tc.out));
        }
    }
}}

struct TRM { f: &'static str, r#in: &'static str, arg: &'static str, out: &'static str }

fn trimTests() -> Vec<TRM> { vec![
    TRM { f: "Trim",       r#in: "abba",   arg: "a",  out: "bb"   },
    TRM { f: "Trim",       r#in: "abba",   arg: "ab", out: ""     },
    TRM { f: "TrimLeft",   r#in: "abba",   arg: "ab", out: ""     },
    TRM { f: "TrimRight",  r#in: "abba",   arg: "ab", out: ""     },
    TRM { f: "TrimLeft",   r#in: "abba",   arg: "a",  out: "bba"  },
    TRM { f: "TrimLeft",   r#in: "abba",   arg: "b",  out: "abba" },
    TRM { f: "TrimRight",  r#in: "abba",   arg: "a",  out: "abb"  },
    TRM { f: "TrimRight",  r#in: "abba",   arg: "b",  out: "abba" },
    TRM { f: "Trim",       r#in: "<tag>",  arg: "<>", out: "tag"  },
    TRM { f: "Trim",       r#in: "* listitem", arg: " *", out: "listitem" },
    TRM { f: "Trim",       r#in: "\"quote\"", arg: "\"", out: "quote" },
    TRM { f: "Trim",       r#in: "\u{2C6F}\u{2C6F}\u{0250}\u{0250}\u{2C6F}\u{2C6F}", arg: "\u{2C6F}", out: "\u{0250}\u{0250}" },
    // empty string cases
    TRM { f: "Trim",       r#in: "abba",   arg: "",   out: "abba" },
    TRM { f: "Trim",       r#in: "",       arg: "123", out: ""    },
    TRM { f: "TrimLeft",   r#in: "abba",   arg: "",   out: "abba" },
    TRM { f: "TrimRight",  r#in: "abba",   arg: "",   out: "abba" },
    TRM { f: "TrimPrefix", r#in: "aabb",   arg: "a",  out: "abb"  },
    TRM { f: "TrimPrefix", r#in: "aabb",   arg: "b",  out: "aabb" },
    TRM { f: "TrimSuffix", r#in: "aabb",   arg: "a",  out: "aabb" },
    TRM { f: "TrimSuffix", r#in: "aabb",   arg: "b",  out: "aab"  },
]}

test!{ fn TestTrim(t) {
    for tc in trimTests() {
        let actual = match tc.f {
            "Trim"       => strings::Trim(tc.r#in, tc.arg),
            "TrimLeft"   => strings::TrimLeft(tc.r#in, tc.arg),
            "TrimRight"  => strings::TrimRight(tc.r#in, tc.arg),
            "TrimPrefix" => strings::TrimPrefix(tc.r#in, tc.arg),
            "TrimSuffix" => strings::TrimSuffix(tc.r#in, tc.arg),
            _ => { t.Errorf(Sprintf!("Undefined trim function %s", tc.f)); continue; }
        };
        if actual != tc.out {
            t.Errorf(Sprintf!("%s(%q, %q) = %q; want %q", tc.f, tc.r#in, tc.arg, actual, tc.out));
        }
    }
}}

struct RP { r#in: &'static str, count: i64, out: &'static str }

fn repeatTests() -> Vec<RP> { vec![
    RP { r#in: "",   count: 0, out: "" },
    RP { r#in: "",   count: 1, out: "" },
    RP { r#in: "",   count: 2, out: "" },
    RP { r#in: "-",  count: 0, out: "" },
    RP { r#in: "-",  count: 1, out: "-" },
    RP { r#in: "-",  count: 10, out: "----------" },
    RP { r#in: "abc", count: 3, out: "abcabcabc" },
]}

test!{ fn TestRepeat(t) {
    for tc in repeatTests() {
        let got = strings::Repeat(tc.r#in, tc.count);
        if got != tc.out {
            t.Errorf(Sprintf!("Repeat(%q, %d) = %q; want %q", tc.r#in, tc.count, got, tc.out));
        }
    }
}}

struct REP { r#in: &'static str, old: &'static str, new: &'static str, n: i64, out: &'static str }

fn replaceTests() -> Vec<REP> { vec![
    REP { r#in: "hello",   old: "l", new: "L", n: 0, out: "hello" },
    REP { r#in: "hello",   old: "l", new: "L", n: -1, out: "heLLo" },
    REP { r#in: "hello",   old: "x", new: "X", n: -1, out: "hello" },
    REP { r#in: "",        old: "x", new: "X", n: -1, out: "" },
    REP { r#in: "radar",   old: "r", new: "<r>", n: -1, out: "<r>ada<r>" },
    REP { r#in: "",        old: "",  new: "<>", n: -1, out: "<>" },
    REP { r#in: "banana",  old: "a", new: "<>", n: -1, out: "b<>n<>n<>" },
    REP { r#in: "banana",  old: "a", new: "<>", n: 1,  out: "b<>nana" },
    REP { r#in: "banana",  old: "a", new: "<>", n: 1000, out: "b<>n<>n<>" },
    REP { r#in: "banana",  old: "an", new: "<>", n: -1, out: "b<><>a" },
    REP { r#in: "banana",  old: "ana", new: "<>", n: -1, out: "b<>na" },
    REP { r#in: "banana",  old: "",   new: "<>", n: -1, out: "<>b<>a<>n<>a<>n<>a<>" },
    REP { r#in: "banana",  old: "",   new: "<>", n: 10, out: "<>b<>a<>n<>a<>n<>a<>" },
    REP { r#in: "banana",  old: "",   new: "<>", n: 6,  out: "<>b<>a<>n<>a<>n<>a" },
    REP { r#in: "banana",  old: "",   new: "<>", n: 5,  out: "<>b<>a<>n<>a<>na" },
    REP { r#in: "banana",  old: "",   new: "<>", n: 1,  out: "<>banana" },
    REP { r#in: "banana",  old: "a",  new: "a",  n: -1, out: "banana" },
    REP { r#in: "banana",  old: "a",  new: "a",  n: 1,  out: "banana" },
    REP { r#in: "☺☻☹",   old: "", new: "<>", n: -1, out: "<>☺<>☻<>☹<>" },
]}

test!{ fn TestReplace(t) {
    for tc in replaceTests() {
        let got = strings::Replace(tc.r#in, tc.old, tc.new, tc.n);
        if got != tc.out {
            t.Errorf(Sprintf!("Replace(%q, %q, %q, %d) = %q; want %q",
                tc.r#in, tc.old, tc.new, tc.n, got, tc.out));
        }
    }
}}

struct CUT { s: &'static str, sep: &'static str, before: &'static str, after: &'static str, found: bool }

fn cutTests() -> Vec<CUT> { vec![
    CUT { s: "abc",  sep: "b",  before: "a",   after: "c",    found: true  },
    CUT { s: "abc",  sep: "a",  before: "",    after: "bc",   found: true  },
    CUT { s: "abc",  sep: "c",  before: "ab",  after: "",     found: true  },
    CUT { s: "abc",  sep: "abc", before: "",   after: "",     found: true  },
    CUT { s: "abc",  sep: "",   before: "",    after: "abc",  found: true  },
    CUT { s: "abc",  sep: "d",  before: "abc", after: "",     found: false },
    CUT { s: "",     sep: "d",  before: "",    after: "",     found: false },
    CUT { s: "",     sep: "",   before: "",    after: "",     found: true  },
]}

test!{ fn TestCut(t) {
    for tc in cutTests() {
        let (b, a, f) = strings::Cut(tc.s, tc.sep);
        if b != tc.before || a != tc.after || f != tc.found {
            t.Errorf(Sprintf!("Cut(%q, %q) = %q, %q, %v; want %q, %q, %v",
                tc.s, tc.sep, b, a, f, tc.before, tc.after, tc.found));
        }
    }
}}

struct CP { s: &'static str, sep: &'static str, after: &'static str, found: bool }

fn cutPrefixTests() -> Vec<CP> { vec![
    CP { s: "abc",  sep: "a",  after: "bc",   found: true  },
    CP { s: "abc",  sep: "abc", after: "",    found: true  },
    CP { s: "abc",  sep: "",   after: "abc",  found: true  },
    CP { s: "abc",  sep: "d",  after: "abc",  found: false },
    CP { s: "",     sep: "d",  after: "",     found: false },
    CP { s: "",     sep: "",   after: "",     found: true  },
]}

test!{ fn TestCutPrefix(t) {
    for tc in cutPrefixTests() {
        let (a, f) = strings::CutPrefix(tc.s, tc.sep);
        if a != tc.after || f != tc.found {
            t.Errorf(Sprintf!("CutPrefix(%q, %q) = %q, %v; want %q, %v",
                tc.s, tc.sep, a, f, tc.after, tc.found));
        }
    }
}}

fn cutSuffixTests() -> Vec<CP> { vec![
    CP { s: "abc",  sep: "bc", after: "a",    found: true  },
    CP { s: "abc",  sep: "abc", after: "",    found: true  },
    CP { s: "abc",  sep: "",   after: "abc",  found: true  },
    CP { s: "abc",  sep: "d",  after: "abc",  found: false },
    CP { s: "",     sep: "d",  after: "",     found: false },
    CP { s: "",     sep: "",   after: "",     found: true  },
]}

test!{ fn TestCutSuffix(t) {
    for tc in cutSuffixTests() {
        let (a, f) = strings::CutSuffix(tc.s, tc.sep);
        if a != tc.after || f != tc.found {
            t.Errorf(Sprintf!("CutSuffix(%q, %q) = %q, %v; want %q, %v",
                tc.s, tc.sep, a, f, tc.after, tc.found));
        }
    }
}}

struct EF { s: &'static str, t: &'static str, out: bool }

fn equalFoldTests() -> Vec<EF> { vec![
    EF { s: "abc",         t: "abc",       out: true  },
    EF { s: "ABcd",        t: "ABcd",      out: true  },
    EF { s: "123abc",      t: "123ABC",    out: true  },
    EF { s: "αβδ",         t: "ΑΒΔ",       out: true  },
    EF { s: "abc",         t: "xyz",       out: false },
    EF { s: "abc",         t: "XYZ",       out: false },
    EF { s: "1",           t: "01",        out: false },
    EF { s: "",            t: "",          out: true  },
    EF { s: "abcd",        t: "abcde",     out: false },
    EF { s: "K",           t: "K",         out: true  },
]}

test!{ fn TestEqualFold(t) {
    for tc in equalFoldTests() {
        let got = strings::EqualFold(tc.s, tc.t);
        if got != tc.out {
            t.Errorf(Sprintf!("EqualFold(%q, %q) = %v; want %v", tc.s, tc.t, got, tc.out));
        }
    }
}}
