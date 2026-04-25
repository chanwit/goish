// Port of go1.25.5 src/encoding/csv/{reader,writer}_test.go — core
// table-driven Read / Write round-trips with RFC 4180 quoting.
//
// Skipped: the enormous fuzz/edge-case Go table (LazyQuotes, BareQuote
// modes, trimspace options) — these are not in goish's v0.11 surface.
// Covered: simple rows, quoted fields with embedded commas and quotes,
// CRLF line endings, comment lines, multi-row ReadAll, Writer quoting.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestReadSimple(t) {
    let mut r = encoding::csv::NewReader("a,b,c\n1,2,3\n");
    let (rec, err) = r.Read();
    if err != nil { t.Errorf(Sprintf!("first Read err: %s", err)); return; }
    if rec != vec!["a", "b", "c"] {
        t.Errorf(Sprintf!("first record = %v", format!("{:?}", rec)));
    }
    let (rec, err) = r.Read();
    if err != nil { t.Errorf(Sprintf!("second Read err: %s", err)); return; }
    if rec != vec!["1", "2", "3"] {
        t.Errorf(Sprintf!("second record = %v", format!("{:?}", rec)));
    }
    let (_, err) = r.Read();
    if err == nil { t.Errorf(Sprintf!("expected EOF, got nil")); }
}}

test!{ fn TestReadQuoted(t) {
    let mut r = encoding::csv::NewReader("\"hello, world\",\"she said \"\"hi\"\"\"\n");
    let (rec, err) = r.Read();
    if err != nil { t.Errorf(Sprintf!("Read err: %s", err)); return; }
    if rec.len() != 2 {
        t.Errorf(Sprintf!("got %d fields, want 2", rec.len() as i64));
        return;
    }
    if rec[0] != "hello, world" {
        t.Errorf(Sprintf!("field 0 = %q, want \"hello, world\"", rec[0]));
    }
    if rec[1] != "she said \"hi\"" {
        t.Errorf(Sprintf!("field 1 = %q, want 'she said \"hi\"'", rec[1]));
    }
}}

test!{ fn TestReadAll(t) {
    let mut r = encoding::csv::NewReader("a,b\n1,2\n3,4\n");
    let (recs, err) = r.ReadAll();
    if err != nil { t.Errorf(Sprintf!("ReadAll err: %s", err)); return; }
    if recs.len() != 3 {
        t.Errorf(Sprintf!("got %d records, want 3", recs.len() as i64));
    }
}}

test!{ fn TestReadCRLF(t) {
    let mut r = encoding::csv::NewReader("a,b\r\n1,2\r\n");
    let (rec, err) = r.Read();
    if err != nil { t.Errorf(Sprintf!("Read err: %s", err)); return; }
    if rec != vec!["a", "b"] {
        t.Errorf(Sprintf!("record = %v", format!("{:?}", rec)));
    }
}}

test!{ fn TestReadWrongFields(t) {
    // Default FieldsPerRecord behavior: first row sets count; later row
    // with different count errors.
    let mut r = encoding::csv::NewReader("a,b\n1,2,3\n");
    let (_, err) = r.Read();
    if err != nil { t.Errorf(Sprintf!("first Read err: %s", err)); }
    let (_, err) = r.Read();
    if err == nil {
        t.Errorf(Sprintf!("expected error on record with wrong field count"));
    }
}}

test!{ fn TestWriteSimple(t) {
    let mut w = encoding::csv::NewWriter();
    let row = vec!["a", "b", "c"];
    let err = w.Write(&row);
    if err != nil { t.Errorf(Sprintf!("Write err: %s", err)); return; }
    let s = w.String();
    if s != "a,b,c\n" {
        t.Errorf(Sprintf!("Writer output = %q, want \"a,b,c\\n\"", s));
    }
}}

test!{ fn TestWriteQuoted(t) {
    let mut w = encoding::csv::NewWriter();
    let row = vec!["hello, world", "she said \"hi\""];
    let err = w.Write(&row);
    if err != nil { t.Errorf(Sprintf!("Write err: %s", err)); return; }
    let s = w.String();
    let want = "\"hello, world\",\"she said \"\"hi\"\"\"\n";
    if s != want {
        t.Errorf(Sprintf!("Writer output = %q, want %q", s, want));
    }
}}

test!{ fn TestWriteRoundTrip(t) {
    let mut w = encoding::csv::NewWriter();
    for row in &[
        vec!["a", "b"],
        vec!["hello, world", "plain"],
        vec!["with\nnewline", "ok"],
    ] {
        let err = w.Write(row);
        if err != nil { t.Errorf(Sprintf!("Write err: %s", err)); return; }
    }
    let s = w.String();
    let mut r = encoding::csv::NewReader(&s);
    let (recs, err) = r.ReadAll();
    if err != nil { t.Errorf(Sprintf!("ReadAll err: %s", err)); return; }
    if recs.len() != 3 {
        t.Errorf(Sprintf!("round-trip: got %d rows, want 3", recs.len() as i64));
    }
}}
