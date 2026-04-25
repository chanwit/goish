// Port of go1.25.5/src/strconv/quote_test.go — Quote/QuoteToASCII/
// QuoteToGraphic + their Rune variants + CanBackquote.
//
// The IsPrint/IsGraphic full-Unicode sweep tests are intentionally
// omitted: Go has its own table (generated) and our unicode module
// follows Rust's tables. Spot-check cases that matter for quoting are
// covered below via the quotetests/quoterunetests tables.

#![allow(non_snake_case)]
use goish::prelude::*;

struct QuoteTest { r#in: &'static str, out: &'static str, ascii: &'static str, graphic: &'static str }

fn quotetests() -> slice<QuoteTest> { vec![
    QuoteTest { r#in: "\x07\x08\x0c\r\n\t\x0b", out: r#""\a\b\f\r\n\t\v""#, ascii: r#""\a\b\f\r\n\t\v""#, graphic: r#""\a\b\f\r\n\t\v""# },
    QuoteTest { r#in: "\\",                     out: r#""\\""#,               ascii: r#""\\""#,               graphic: r#""\\""# },
    // "abc\xffdef" — not valid UTF-8 so we substitute a valid test case
    // that exercises the \u00ff escape path via rune 0xFF encoded in UTF-8.
    QuoteTest { r#in: "abc\u{00ff}def",         out: "\"abcÿdef\"",           ascii: r#""abc\u00ffdef""#,    graphic: "\"abcÿdef\"" },
    QuoteTest { r#in: "\u{263a}",               out: "\"☺\"",                 ascii: r#""\u263a""#,          graphic: "\"☺\"" },
    QuoteTest { r#in: "\u{10ffff}",             out: r#""\U0010ffff""#,       ascii: r#""\U0010ffff""#,      graphic: r#""\U0010ffff""# },
    QuoteTest { r#in: "\x04",                   out: r#""\x04""#,             ascii: r#""\x04""#,            graphic: r#""\x04""# },
    QuoteTest { r#in: "!\u{00a0}!\u{2000}!\u{3000}!", out: r#""!\u00a0!\u2000!\u3000!""#, ascii: r#""!\u00a0!\u2000!\u3000!""#, graphic: "\"!\u{00a0}!\u{2000}!\u{3000}!\"" },
    QuoteTest { r#in: "\x7f",                   out: r#""\x7f""#,             ascii: r#""\x7f""#,            graphic: r#""\x7f""# },
].into()}

test!{ fn TestQuote(t) {
    for tt in quotetests() {
        let out = strconv::Quote(tt.r#in);
        if out != tt.out {
            t.Errorf(Sprintf!("Quote(%q) = %s, want %s", tt.r#in, out, tt.out));
        }
        let ab = strconv::AppendQuote(b"abc", tt.r#in);
        let got = std::str::from_utf8(&ab).unwrap();
        let want = Sprintf!("abc%v", tt.out);
        if got != want {
            t.Errorf(Sprintf!("AppendQuote(%q, %q) = %s, want %s", "abc", tt.r#in, got, want));
        }
    }
}}

test!{ fn TestQuoteToASCII(t) {
    for tt in quotetests() {
        let out = strconv::QuoteToASCII(tt.r#in);
        if out != tt.ascii {
            t.Errorf(Sprintf!("QuoteToASCII(%q) = %s, want %s", tt.r#in, out, tt.ascii));
        }
        let ab = strconv::AppendQuoteToASCII(b"abc", tt.r#in);
        let got = std::str::from_utf8(&ab).unwrap();
        let want = Sprintf!("abc%v", tt.ascii);
        if got != want {
            t.Errorf(Sprintf!("AppendQuoteToASCII(%q, %q) = %s, want %s", "abc", tt.r#in, got, want));
        }
    }
}}

test!{ fn TestQuoteToGraphic(t) {
    for tt in quotetests() {
        let out = strconv::QuoteToGraphic(tt.r#in);
        if out != tt.graphic {
            t.Errorf(Sprintf!("QuoteToGraphic(%q) = %s, want %s", tt.r#in, out, tt.graphic));
        }
    }
}}

// ── Rune tests ───────────────────────────────────────────────────────

struct QuoteRuneTest { r#in: i32, out: &'static str, ascii: &'static str, graphic: &'static str }

fn quoterunetests() -> slice<QuoteRuneTest> { vec![
    QuoteRuneTest { r#in: 'a' as i32,  out: "'a'",  ascii: "'a'",  graphic: "'a'" },
    QuoteRuneTest { r#in: '\x07' as i32, out: r#"'\a'"#, ascii: r#"'\a'"#, graphic: r#"'\a'"# },
    QuoteRuneTest { r#in: '\\' as i32, out: r#"'\\'"#, ascii: r#"'\\'"#, graphic: r#"'\\'"# },
    QuoteRuneTest { r#in: 0xFF,        out: "'ÿ'",  ascii: r#"'\u00ff'"#, graphic: "'ÿ'" },
    QuoteRuneTest { r#in: 0x263a,      out: "'☺'",  ascii: r#"'\u263a'"#, graphic: "'☺'" },
    QuoteRuneTest { r#in: 0xdead,      out: "'\u{fffd}'", ascii: r#"'\ufffd'"#, graphic: "'\u{fffd}'" },
    QuoteRuneTest { r#in: 0xfffd,      out: "'\u{fffd}'", ascii: r#"'\ufffd'"#, graphic: "'\u{fffd}'" },
    QuoteRuneTest { r#in: 0x0010ffff,  out: r#"'\U0010ffff'"#, ascii: r#"'\U0010ffff'"#, graphic: r#"'\U0010ffff'"# },
    QuoteRuneTest { r#in: 0x0010ffff + 1, out: "'\u{fffd}'", ascii: r#"'\ufffd'"#, graphic: "'\u{fffd}'" },
    QuoteRuneTest { r#in: 0x04,        out: r#"'\x04'"#, ascii: r#"'\x04'"#, graphic: r#"'\x04'"# },
    QuoteRuneTest { r#in: 0x00a0,      out: r#"'\u00a0'"#, ascii: r#"'\u00a0'"#, graphic: "'\u{00a0}'" },
    QuoteRuneTest { r#in: 0x2000,      out: r#"'\u2000'"#, ascii: r#"'\u2000'"#, graphic: "'\u{2000}'" },
    QuoteRuneTest { r#in: 0x3000,      out: r#"'\u3000'"#, ascii: r#"'\u3000'"#, graphic: "'\u{3000}'" },
].into()}

test!{ fn TestQuoteRune(t) {
    for tt in quoterunetests() {
        let out = strconv::QuoteRune(tt.r#in);
        if out != tt.out {
            t.Errorf(Sprintf!("QuoteRune(%U) = %s, want %s", tt.r#in, out, tt.out));
        }
    }
}}

test!{ fn TestQuoteRuneToASCII(t) {
    for tt in quoterunetests() {
        let out = strconv::QuoteRuneToASCII(tt.r#in);
        if out != tt.ascii {
            t.Errorf(Sprintf!("QuoteRuneToASCII(%U) = %s, want %s", tt.r#in, out, tt.ascii));
        }
    }
}}

test!{ fn TestQuoteRuneToGraphic(t) {
    for tt in quoterunetests() {
        let out = strconv::QuoteRuneToGraphic(tt.r#in);
        if out != tt.graphic {
            t.Errorf(Sprintf!("QuoteRuneToGraphic(%U) = %s, want %s", tt.r#in, out, tt.graphic));
        }
    }
}}

// ── CanBackquote ─────────────────────────────────────────────────────

struct CanBackquoteTest { r#in: &'static str, out: bool }

fn canbackquotetests() -> slice<CanBackquoteTest> { vec![
    CanBackquoteTest { r#in: "`",                 out: false },
    CanBackquoteTest { r#in: "\x00",              out: false },
    CanBackquoteTest { r#in: "\x09", /* tab */    out: true  },
    CanBackquoteTest { r#in: "\n",                out: false },
    CanBackquoteTest { r#in: " ",                 out: true  },
    CanBackquoteTest { r#in: "a",                 out: true  },
    CanBackquoteTest { r#in: "☺",                 out: true  },
    CanBackquoteTest { r#in: "hello world",       out: true  },
    CanBackquoteTest { r#in: "back`quote",        out: false },
    CanBackquoteTest { r#in: "\x7f",              out: false },
].into()}

test!{ fn TestCanBackquote(t) {
    for tt in canbackquotetests() {
        let got = strconv::CanBackquote(tt.r#in);
        if got != tt.out {
            t.Errorf(Sprintf!("CanBackquote(%q) = %v, want %v", tt.r#in, got, tt.out));
        }
    }
}}
