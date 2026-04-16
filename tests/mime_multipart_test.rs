// Port of go1.25.5 src/mime/multipart/writer_test.go + multipart_test.go
// (subset). Full round-trip: build a multipart body with Writer, then
// read it back with Reader and verify part metadata + body content.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::mime::multipart;
use std::io::Cursor;

// ── TestWriter ──────────────────────────────────────────────────────

test!{ fn TestWriter(t) {
    let file_contents = b"my file contents";

    let mut buf: Vec<u8> = Vec::new();
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

        if buf.is_empty() { t.Fatal("empty buffer"); }
        if buf[0] == b'\r' || buf[0] == b'\n' {
            t.Fatal("unexpected leading newline");
        }
    }

    let mut r = multipart::NewReader(Cursor::new(&buf), &boundary);

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
    struct Case { b: &'static str, ok: bool }
    let cases = [
        Case { b: "abc",          ok: true },
        Case { b: "",              ok: false },
        Case { b: "!",             ok: false },
        Case { b: "my-separator",   ok: true },
        Case { b: "with space",    ok: true },
        Case { b: "badspace ",     ok: false },
    ];
    for c in &cases {
        let mut buf: Vec<u8> = Vec::new();
        let mut w = multipart::NewWriter(&mut buf);
        let err = w.SetBoundary(c.b);
        let got = err == nil;
        if got != c.ok {
            t.Errorf(Sprintf!("SetBoundary(%s): got ok=%s want ok=%s",
                c.b, if got {"true"} else {"false"}, if c.ok {"true"} else {"false"}));
        }
        if got && w.Boundary() != c.b {
            t.Errorf(Sprintf!("Boundary() = %s, want %s", w.Boundary(), c.b));
        }
    }
}}

// ── TestFormDataContentType ─────────────────────────────────────────

test!{ fn TestFormDataContentType(t) {
    let mut buf: Vec<u8> = Vec::new();
    let mut w = multipart::Writer::NewWriter(&mut buf);
    let err = w.SetBoundary("myboundary");
    if err != nil { t.Fatal(&Sprintf!("SetBoundary: %s", err)); }
    let got = w.FormDataContentType();
    let want = "multipart/form-data; boundary=myboundary";
    if got != want {
        t.Errorf(Sprintf!("FormDataContentType = %s, want %s", got, want));
    }
}}
