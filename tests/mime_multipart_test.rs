// Port of go1.25.5 src/mime/multipart/writer_test.go + multipart_test.go
// (subset). Full round-trip: build a multipart body with Writer, then
// read it back with Reader and verify part metadata + body content.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::mime::multipart;
use goish::net::textproto;

// ── TestWriter ──────────────────────────────────────────────────────

test!{ fn TestWriter(t) {
    let file_contents = b"my file contents";

    let mut buf = bytes::Buffer::new();
    let boundary;
    {
        let mut w = multipart::NewWriter(&mut buf);
        boundary = w.Boundary();
        {
            let (mut part, err) = w.CreateFormFile("myfile", "my-file.txt");
            if err != nil { t.Fatal(&Sprintf!("CreateFormFile: %s", err)); }
            let (_, e) = part.Write(file_contents);
            if e != nil { t.Fatal(&Sprintf!("part.Write: %s", e)); }
        }
        let err = w.WriteField("key", "val");
        if err != nil { t.Fatal(&Sprintf!("WriteField: %s", err)); }
        let err = w.Close();
        if err != nil { t.Fatal(&Sprintf!("Close: %s", err)); }

        let bytes = buf.Bytes();
        if bytes.is_empty() { t.Fatal("empty buffer"); }
        if bytes[0] == b'\r' || bytes[0] == b'\n' {
            t.Fatal("unexpected leading newline");
        }
    }

    let mut r = multipart::NewReader(&mut buf, &boundary);

    let (part1, err) = r.NextPart();
    if err != nil { t.Fatal(&Sprintf!("part 1: %s", err)); }
    if part1.FormName() != "myfile" {
        t.Errorf(Sprintf!("part 1: want form name myfile, got %s", part1.FormName()));
    }
    if part1.FileName() != "my-file.txt" {
        t.Errorf(Sprintf!("part 1: want file name my-file.txt, got %s", part1.FileName()));
    }
    if part1.Body() != file_contents {
        t.Errorf(Sprintf!("part 1: body mismatch, got %d bytes", part1.Body().len() as i64));
    }

    let (part2, err) = r.NextPart();
    if err != nil { t.Fatal(&Sprintf!("part 2: %s", err)); }
    if part2.FormName() != "key" {
        t.Errorf(Sprintf!("part 2: want form name key, got %s", part2.FormName()));
    }
    if part2.Body() != b"val" {
        t.Errorf(Sprintf!("part 2: body = %s, want val",
            std::str::from_utf8(part2.Body()).unwrap_or("?")));
    }

    let (_p, err) = r.NextPart();
    if err == nil {
        t.Fatal("expected end of parts");
    }
}}

// ── TestWriterSetBoundary ───────────────────────────────────────────

test!{ fn TestWriterSetBoundary(t) {
    struct Case { b: string, ok: bool }
    let cases: Vec<Case> = vec![
        Case { b: "abc".into(),                ok: true },
        Case { b: "".into(),                    ok: false },
        Case { b: "!".into(),                   ok: false },
        Case { b: strings::Repeat("x", 70),     ok: true },
        Case { b: strings::Repeat("x", 71),     ok: false },
        Case { b: "my-separator".into(),        ok: true },
        Case { b: "with space".into(),          ok: true },
        Case { b: "badspace ".into(),           ok: false },
        Case { b: "(boundary)".into(),          ok: true },
    ];
    range!{ i, c := cases[..];
        let mut buf = bytes::Buffer::new();
        let mut w = multipart::NewWriter(&mut buf);
        let err = w.SetBoundary(&c.b);
        let got = err == nil;
        if got != c.ok {
            t.Errorf(Sprintf!("%d. boundary %s = ok=%s; want ok=%s",
                i as i64, c.b,
                if got {"true"} else {"false"},
                if c.ok {"true"} else {"false"}));
        }
        if got {
            if w.Boundary() != c.b {
                t.Errorf(Sprintf!("%d. Boundary() = %s, want %s", i as i64, w.Boundary(), c.b));
            }
            let ct = w.FormDataContentType();
            let want_ct = Sprintf!("multipart/form-data; boundary=%s", c.b);
            if ct != want_ct {
                t.Errorf(Sprintf!("%d. ContentType = %s, want %s", i as i64, ct, want_ct));
            }
        }
    }
}}

// ── TestNameAccessors: FormName/FileName parse Content-Disposition ──

test!{ fn TestNameAccessors(t) {
    struct Case {
        disposition: &'static str,
        want_name: &'static str,
        want_filename: &'static str,
    }
    let cases = [
        Case { disposition: r#"form-data; name="foo""#,                        want_name: "foo", want_filename: "" },
        Case { disposition: " form-data ; name=foo",                            want_name: "foo", want_filename: "" },
        Case { disposition: r#"FORM-DATA;name="foo""#,                         want_name: "foo", want_filename: "" },
        Case { disposition: r#" FORM-DATA ; name="foo""#,                      want_name: "foo", want_filename: "" },
        Case { disposition: r#" FORM-DATA ; filename="foo.txt"; name=foo; baz=quux"#,
               want_name: "foo", want_filename: "foo.txt" },
        // Non-form-data disposition — FormName returns "".
        Case { disposition: r#" not-form-data ; filename="bar.txt"; name=foo; baz=quux"#,
               want_name: "", want_filename: "bar.txt" },
    ];
    range!{ i, c := cases[..];
        let mut h = textproto::MIMEHeader::new();
        h.Set("Content-Disposition", c.disposition);
        let p = multipart::ReaderPart::new_for_header(h);
        if p.FormName() != c.want_name {
            t.Errorf(Sprintf!("case %d: FormName() = %s; want %s",
                i as i64, p.FormName(), c.want_name));
        }
        if p.FileName() != c.want_filename {
            t.Errorf(Sprintf!("case %d: FileName() = %s; want %s",
                i as i64, p.FileName(), c.want_filename));
        }
    }
}}

// ── TestFormDataContentType ─────────────────────────────────────────

test!{ fn TestFormDataContentType(t) {
    let mut buf = bytes::Buffer::new();
    let mut w = multipart::NewWriter(&mut buf);
    let err = w.SetBoundary("myboundary");
    if err != nil { t.Fatal(&Sprintf!("SetBoundary: %s", err)); }
    let got = w.FormDataContentType();
    let want = "multipart/form-data; boundary=myboundary";
    if got != want {
        t.Errorf(Sprintf!("FormDataContentType = %s, want %s", got, want));
    }
}}
