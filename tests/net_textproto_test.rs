// Port of go1.25.5 src/net/textproto/{header,reader}_test.go.
// MIMEHeader canonical keys + Values() multi-value lookup; Reader's
// ReadLine / ReadContinuedLine / ReadDotLines / ReadDotBytes /
// ReadMIMEHeader.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::net::textproto;

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
    if len!(values) != 2 {
        t.Errorf(Sprintf!("count: %d; want 2", len!(values) as i64));
    }
}}

// ── Reader helper — Go-shape `textproto.NewReader(strings.NewReader(s))` ──

fn reader(s: &str) -> textproto::Reader<strings::Reader> {
    textproto::NewReader(strings::NewReader(s))
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
    if len!(s) != 0 || err == nil {
        t.Fatal(&Sprintf!("EOF: %s, %s", s, err));
    }
}}

// ── TestReadLineLongLine ────────────────────────────────────────────

test!{ fn TestReadLineLongLine(t) {
    let line = strings::Repeat("12345", 10000);
    let data = Sprintf!("%s\r\n", line);
    let mut r = textproto::NewReader(strings::NewReader(&data));
    let (s, err) = r.ReadLine();
    if err != nil { t.Fatal(&Sprintf!("Line 1: %s", err)); }
    if s != line {
        t.Fatal(&Sprintf!("%d-byte line does not match expected %d-byte line",
            len!(s) as i64, len!(line) as i64));
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
    if len!(s) != len!(want) {
        t.Fatal(&Sprintf!("ReadDotLines len = %d, want %d", len!(s), len!(want)));
    }
    for (i, v) in range!(s) {
        if *v != want[i] {
            t.Errorf(Sprintf!("ReadDotLines[%d] = %s, want %s", i as i64, v, want[i]));
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
        t.Errorf(Sprintf!("ReadDotBytes len = %d, want %d", len!(b) as i64, len!(want) as i64));
    }
}}

// ── TestReadMIMEHeader ──────────────────────────────────────────────

test!{ fn TestReadMIMEHeader(t) {
    let mut r = reader("my-key: Value 1  \r\nLong-key: Even \n Longer Value\r\nmy-Key: Value 2\r\n\n");
    let (h, err) = r.ReadMIMEHeader();
    if err != nil { t.Fatal(&Sprintf!("ReadMIMEHeader err: %s", err)); }
    let mk = h.Values("My-Key");
    if len!(mk) != 2 || mk[0] != "Value 1" || mk[1] != "Value 2" {
        // Go trims trailing whitespace from values ("Value 1  " → "Value 1").
        let first  = if len!(mk) > 0 { mk[0].clone() } else { String::new() };
        let second = if len!(mk) > 1 { mk[1].clone() } else { String::new() };
        t.Errorf(Sprintf!("My-Key values = [%s, %s]", first, second));
    }
    let lk = h.Values("Long-Key");
    if len!(lk) != 1 || lk[0] != "Even Longer Value" {
        let first = if len!(lk) > 0 { lk[0].clone() } else { String::new() };
        t.Errorf(Sprintf!("Long-Key value = %s", first));
    }
}}

// ── TestReadCodeLine ────────────────────────────────────────────────

test!{ fn TestReadCodeLine(t) {
    let mut r = reader("123 hi\n234 bye\n345 no way\n");
    let (code, msg, err) = r.ReadCodeLine(0);
    if code != 123 || msg != "hi" || err != nil {
        t.Fatal(&Sprintf!("Line 1: %d, %s, %s", code, msg, err));
    }
    let (code, msg, err) = r.ReadCodeLine(23);
    if code != 234 || msg != "bye" || err != nil {
        t.Fatal(&Sprintf!("Line 2: %d, %s, %s", code, msg, err));
    }
    let (code, msg, err) = r.ReadCodeLine(346);
    if code != 345 || msg != "no way" || err == nil {
        t.Fatal(&Sprintf!("Line 3: %d, %s, %s (expected mismatch)", code, msg, err));
    }
}}

// ── TestReadMIMEHeaderNonCompliant: spaces before colon preserved ───

test!{ fn TestReadMIMEHeaderNonCompliant(t) {
    let mut r = reader("Foo: bar\r\nContent-Language: en\r\nSID : 0\r\nAudio Mode : None\r\nPrivilege : 127\r\n\r\n");
    let (h, err) = r.ReadMIMEHeader();
    if err != nil { t.Fatal(&Sprintf!("ReadMIMEHeader: %s", err)); }
    if h.Get("Foo") != "bar" {
        t.Errorf(Sprintf!("Foo = %s, want bar", h.Get("Foo")));
    }
    if h.Get("Content-Language") != "en" {
        t.Errorf(Sprintf!("Content-Language = %s, want en", h.Get("Content-Language")));
    }
    // Non-canonical key "SID " (with trailing space) — preserved verbatim.
    if h.Get("SID ") != "0" {
        t.Errorf(Sprintf!("SID  = %s, want 0", h.Get("SID ")));
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

// ── TestReadMIMEHeaderNoKey ─────────────────────────────────────────

test!{ fn TestReadMIMEHeaderNoKey(t) {
    let mut r = reader(": bar\ntest-1: 1\n\n");
    let (_h, err) = r.ReadMIMEHeader();
    if err == nil {
        t.Fatal("ReadMIMEHeader: expected error for empty key");
    }
}}

// ── TestLargeReadMIMEHeader: 16k-byte cookie ────────────────────────

test!{ fn TestLargeReadMIMEHeader(t) {
    let big = strings::Repeat("x", 16 * 1024);
    let src = Sprintf!("Cookie: %s\r\n\n", big);
    let mut r = textproto::NewReader(strings::NewReader(&src));
    let (h, err) = r.ReadMIMEHeader();
    if err != nil { t.Fatal(&Sprintf!("ReadMIMEHeader: %s", err)); }
    let cookie = h.Get("Cookie");
    if len!(cookie) != len!(big) {
        t.Fatal(&Sprintf!("ReadMIMEHeader: %d bytes, want %d bytes",
            len!(cookie) as i64, len!(big) as i64));
    }
}}

// ── TestReadMIMEHeaderMalformed: no-colon line, tab-only line ───────

test!{ fn TestReadMIMEHeaderMalformed(t) {
    let inputs = [
        "No colon first line\r\nFoo: foo\r\n\r\n",
        "Foo: foo\r\nNo colon second line\r\n\r\n",
        ": empty key\r\n\r\n",
    ];
    for input in &inputs {
        let mut r = reader(input);
        let (_h, err) = r.ReadMIMEHeader();
        if err == nil {
            t.Errorf(Sprintf!("ReadMIMEHeader(%s): expected error, got nil", input));
        }
    }
}}

