// Port of go1.25.5 src/net/textproto/{header,reader}_test.go.
// MIMEHeader canonical keys + Values() multi-value lookup; Reader's
// ReadLine / ReadContinuedLine / ReadDotLines / ReadDotBytes /
// ReadMIMEHeader.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::net::textproto;
use std::io::Cursor;

// ── TestCanonicalMIMEHeaderKey ──────────────────────────────────────

test!{ fn TestCanonicalMIMEHeaderKey(t) {
    struct Case { input: &'static str, want: &'static str }
    let cases = [
        Case { input: "a-b-c",       want: "A-B-C" },
        Case { input: "a-1-c",       want: "A-1-C" },
        Case { input: "User-Agent",  want: "User-Agent" },
        Case { input: "uSER-aGENT",  want: "User-Agent" },
        Case { input: "user-agent",  want: "User-Agent" },
        Case { input: "USER-AGENT",  want: "User-Agent" },
        Case { input: "foo-bar_baz", want: "Foo-Bar_baz" },
        Case { input: "foo-bar$baz", want: "Foo-Bar$baz" },
        Case { input: "foo-bar~baz", want: "Foo-Bar~baz" },
        Case { input: "foo-bar*baz", want: "Foo-Bar*baz" },
        Case { input: "üser-agenT",   want: "üser-agenT" },
        Case { input: "a B",          want: "a B" },
        Case { input: "C Ontent-Transfer-Encoding", want: "C Ontent-Transfer-Encoding" },
        Case { input: "foo bar",     want: "foo bar" },
    ];
    for c in &cases {
        let got = textproto::CanonicalMIMEHeaderKey(c.input);
        if got != c.want {
            t.Errorf(Sprintf!("CanonicalMIMEHeaderKey(%s) = %s, want %s", c.input, got, c.want));
        }
    }
}}

// ── TestMIMEHeaderMultipleValues ────────────────────────────────────

test!{ fn TestMIMEHeaderMultipleValues(t) {
    let mut h = textproto::MIMEHeader::new();
    h.Add("Set-Cookie", "cookie 1");
    h.Add("Set-Cookie", "cookie 2");
    let values = h.Values("set-cookie");
    if values.len() != 2 {
        t.Errorf(Sprintf!("count: %d; want 2", values.len() as i64));
    }
}}

// ── Reader helper ───────────────────────────────────────────────────

fn reader(s: &'static str) -> textproto::Reader<Cursor<&'static [u8]>> {
    textproto::Reader::NewReader(Cursor::new(s.as_bytes()))
}

// ── TestReadLine ────────────────────────────────────────────────────

test!{ fn TestReadLine(t) {
    let mut r = reader("line1\nline2\n");
    let (s, err) = r.ReadLine();
    if s != "line1" || err != nil {
        t.Fatal(&Sprintf!("Line 1: %s, %s", s, err));
    }
    let (s, err) = r.ReadLine();
    if s != "line2" || err != nil {
        t.Fatal(&Sprintf!("Line 2: %s, %s", s, err));
    }
    let (s, err) = r.ReadLine();
    if !s.is_empty() || err == nil {
        t.Fatal(&Sprintf!("EOF: %s, %s", s, err));
    }
}}

// ── TestReadLineLongLine ────────────────────────────────────────────

test!{ fn TestReadLineLongLine(t) {
    let line = "12345".repeat(10000);
    let data = format!("{}\r\n", line);
    // Need a 'static-like data source: box+leak the data for the Cursor.
    let leaked: &'static [u8] = Box::leak(data.into_boxed_str().into_boxed_bytes());
    let mut r = textproto::Reader::NewReader(Cursor::new(leaked));
    let (s, err) = r.ReadLine();
    if err != nil { t.Fatal(&Sprintf!("Line 1: %s", err)); }
    if s != line {
        t.Fatal(&Sprintf!("%d-byte line does not match expected %d-byte line", s.len() as i64, line.len() as i64));
    }
}}

// ── TestReadContinuedLine ───────────────────────────────────────────

test!{ fn TestReadContinuedLine(t) {
    let mut r = reader("line1\nline\n 2\nline3\n");
    let (s, err) = r.ReadContinuedLine();
    if s != "line1" || err != nil { t.Fatal(&Sprintf!("Line 1: %s, %s", s, err)); }
    let (s, err) = r.ReadContinuedLine();
    if s != "line 2" || err != nil { t.Fatal(&Sprintf!("Line 2: %s, %s", s, err)); }
    let (s, err) = r.ReadContinuedLine();
    if s != "line3" || err != nil { t.Fatal(&Sprintf!("Line 3: %s, %s", s, err)); }
}}

// ── TestReadDotLines ────────────────────────────────────────────────

test!{ fn TestReadDotLines(t) {
    let mut r = reader("dotlines\r\n.foo\r\n..bar\n...baz\nquux\r\n\r\n.\r\n");
    let (s, err) = r.ReadDotLines();
    let want = ["dotlines", "foo", ".bar", "..baz", "quux", ""];
    if err != nil { t.Fatal(&Sprintf!("ReadDotLines err: %s", err)); }
    if s.len() != want.len() {
        t.Fatal(&Sprintf!("ReadDotLines len = %d, want %d", s.len() as i64, want.len() as i64));
    }
    for i in 0..s.len() {
        if s[i] != want[i] {
            t.Errorf(Sprintf!("ReadDotLines[%d] = %s, want %s", i as i64, s[i], want[i]));
        }
    }
}}

// ── TestReadDotBytes ────────────────────────────────────────────────

test!{ fn TestReadDotBytes(t) {
    let mut r = reader("dotlines\r\n.foo\r\n..bar\n...baz\nquux\r\n\r\n.\r\n");
    let (b, err) = r.ReadDotBytes();
    let want = b"dotlines\nfoo\n.bar\n..baz\nquux\n\n".to_vec();
    if err != nil { t.Fatal(&Sprintf!("ReadDotBytes err: %s", err)); }
    if b != want {
        t.Errorf(Sprintf!("ReadDotBytes len = %d, want %d", b.len() as i64, want.len() as i64));
    }
}}

// ── TestReadMIMEHeader ──────────────────────────────────────────────

test!{ fn TestReadMIMEHeader(t) {
    let mut r = reader("my-key: Value 1  \r\nLong-key: Even \n Longer Value\r\nmy-Key: Value 2\r\n\n");
    let (h, err) = r.ReadMIMEHeader();
    if err != nil { t.Fatal(&Sprintf!("ReadMIMEHeader err: %s", err)); }
    let mk = h.Values("My-Key");
    if mk.len() != 2 || mk[0] != "Value 1" || mk[1] != "Value 2" {
        // Note: Go trims trailing whitespace from values ("Value 1  " → "Value 1").
        t.Errorf(Sprintf!("My-Key values = [%s, %s]",
            mk.get(0).cloned().unwrap_or_default(),
            mk.get(1).cloned().unwrap_or_default()));
    }
    let lk = h.Values("Long-Key");
    if lk.len() != 1 || lk[0] != "Even Longer Value" {
        t.Errorf(Sprintf!("Long-Key value = %s",
            lk.get(0).cloned().unwrap_or_default()));
    }
}}

// ── TestReadMIMEHeaderSingle ────────────────────────────────────────

test!{ fn TestReadMIMEHeaderSingle(t) {
    let mut r = reader("Foo: bar\n\n");
    let (h, err) = r.ReadMIMEHeader();
    if err != nil { t.Fatal(&Sprintf!("ReadMIMEHeader: %s", err)); }
    if h.Get("Foo") != "bar" {
        t.Errorf(Sprintf!("Foo = %s, want bar", h.Get("Foo")));
    }
}}
