// Port of go1.25.5/src/strings/replace_test.go — Replacer smoke tests.
// Go's test has a huge 256-entry inc-replacer case + tree-based shard
// tests that hit the optimised paths; those are tracked as v0.6.x
// follow-ups. This file covers the canonical HTML-escape and
// capital-letters cases plus the no-op / empty-input cases.

#![allow(non_snake_case)]
use goish::prelude::*;

fn html_escaper() -> strings::Replacer {
    strings::NewReplacer(&[
        "&",  "&amp;",
        "<",  "&lt;",
        ">",  "&gt;",
        "\"", "&quot;",
        "'",  "&apos;",
    ])
}

fn html_unescaper() -> strings::Replacer {
    strings::NewReplacer(&[
        "&amp;",  "&",
        "&lt;",   "<",
        "&gt;",   ">",
        "&quot;", "\"",
        "&apos;", "'",
    ])
}

fn capital_letters() -> strings::Replacer {
    strings::NewReplacer(&["a", "A", "b", "B"])
}

struct Case { r: strings::Replacer, r#in: string, out: string }

test!{ fn TestReplacer(t) {
    let cases: Vec<Case> = vec![
        Case { r: capital_letters(), r#in: "brad".into(),                out: "BrAd".into() },
        Case { r: capital_letters(),
               r#in: strings::Repeat("a", (32 << 10) + 123),
               out:  strings::Repeat("A", (32 << 10) + 123) },
        Case { r: capital_letters(), r#in: "".into(),                    out: "".into() },

        // HTML-escape round-trips.
        Case { r: html_escaper(),
               r#in:  "<script>alert(\"xss\")</script>".into(),
               out:   "&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;".into() },
        Case { r: html_escaper(),
               r#in:  "AT&T".into(),
               out:   "AT&amp;T".into() },
        Case { r: html_unescaper(),
               r#in:  "&amp;lt;".into(),
               out:   "&lt;".into() }, // only one pass
        Case { r: html_unescaper(),
               r#in:  "&lt;b&gt;hi&lt;/b&gt;".into(),
               out:   "<b>hi</b>".into() },
    ];

    for c in cases {
        let got = c.r.Replace(&c.r#in);
        if got != c.out {
            t.Errorf(Sprintf!("Replace(%q) got %q; want %q", c.r#in, got, c.out));
        }
    }
}}

test!{ fn TestReplacerNoOp(t) {
    // Empty Replacer = identity.
    let r = strings::NewReplacer(&[] as &[&str]);
    let got = r.Replace("anything");
    if got != "anything" {
        t.Errorf(Sprintf!("empty Replacer got %q; want %q", got, "anything"));
    }
}}

test!{ fn TestReplacerOrderMatters(t) {
    // Go's rule: when multiple patterns could match at one position,
    // the longest/earlier pattern wins (implementation-defined but must
    // be consistent across runs).
    let r = strings::NewReplacer(&["ab", "X", "a", "Y"]);
    let got = r.Replace("ab ac");
    // "X Yc" — 'ab' should win over 'a' at position 0.
    if got != "X Yc" {
        t.Errorf(Sprintf!("got %q; want %q", got, "X Yc"));
    }
}}
