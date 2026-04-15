// Port of go1.25.5 src/bufio/bufio_test.go — Reader/Writer core tests.
//
// Elided: tests that exercise Go-specific behaviors:
//   - TestReadRune / TestUnreadRune / TestNoUnreadRuneAfter*
//     depend on UnreadByte/UnreadRune having true round-trip behavior;
//     our UnreadByte is a no-op today.
//   - TestPeek / TestBufferFull / TestReadLine* rely on Peek/ReadLine;
//     our Reader doesn't expose those yet.
//   - TestReadWriteRune / TestWriteInvalidRune (WriteRune not implemented).
//   - TestWriterAppend / TestNewReaderSizeIdempotent (buffered-buffer reuse).
//   - TestReaderWriteTo / TestWriterReadFrom (optional WriteTo/ReadFrom).
// These are tracked as v0.8.x follow-ups.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::bufio;
use std::io::Cursor;

test!{ fn TestReaderReadByte(t) {
    let mut r = bufio::NewReader(Cursor::new(b"hello".to_vec()));
    let expected = [b'h', b'e', b'l', b'l', b'o'];
    for &want in &expected {
        let (got, err) = r.ReadByte();
        if err != nil { t.Errorf(Sprintf!("ReadByte err: %s", err)); }
        if got != want { t.Errorf(Sprintf!("ReadByte got %d want %d", got, want)); }
    }
    let (_, err) = r.ReadByte();
    if err == nil { t.Errorf(Sprintf!("ReadByte after EOF got nil err")); }
}}

test!{ fn TestReaderReadString(t) {
    let mut r = bufio::NewReader(Cursor::new(b"alpha,beta,gamma".to_vec()));
    let (s, err) = r.ReadString(b',');
    if err != nil { t.Errorf(Sprintf!("1st: %s", err)); }
    if s != "alpha," { t.Errorf(Sprintf!("got %q want 'alpha,'", s)); }
    let (s, err) = r.ReadString(b',');
    if err != nil { t.Errorf(Sprintf!("2nd: %s", err)); }
    if s != "beta," { t.Errorf(Sprintf!("got %q want 'beta,'", s)); }
    // Last segment has no delim → returns EOF.
    let (s, err) = r.ReadString(b',');
    if err == nil { t.Errorf(Sprintf!("3rd: want EOF, got nil")); }
    if s != "gamma" { t.Errorf(Sprintf!("got %q want 'gamma'", s)); }
}}

test!{ fn TestReaderReadBytes(t) {
    let mut r = bufio::NewReader(Cursor::new(b"x|y|z".to_vec()));
    let (b, err) = r.ReadBytes(b'|');
    if err != nil { t.Errorf(Sprintf!("1st: %s", err)); }
    if b != b"x|" { t.Errorf(Sprintf!("got %v", b.len())); }
    let (b, err) = r.ReadBytes(b'|');
    if err != nil { t.Errorf(Sprintf!("2nd: %s", err)); }
    if b != b"y|" { t.Errorf(Sprintf!("got %v", b.len())); }
    let (b, err) = r.ReadBytes(b'|');
    if err == nil { t.Errorf(Sprintf!("3rd: expected EOF")); }
    if b != b"z" { t.Errorf(Sprintf!("got len=%d", b.len())); }
}}

test!{ fn TestReaderReadRune(t) {
    let mut r = bufio::NewReader(Cursor::new("aλb".as_bytes().to_vec()));
    let (rune, n, err) = r.ReadRune();
    if err != nil || n != 1 || rune != 'a' as i32 {
        t.Errorf(Sprintf!("ReadRune a: got (%d, %d, %s)", rune, n, err));
    }
    let (rune, n, err) = r.ReadRune();
    if err != nil || n != 2 || rune != 'λ' as i32 {
        t.Errorf(Sprintf!("ReadRune λ: got (%d, %d, %s)", rune, n, err));
    }
    let (rune, n, err) = r.ReadRune();
    if err != nil || n != 1 || rune != 'b' as i32 {
        t.Errorf(Sprintf!("ReadRune b: got (%d, %d, %s)", rune, n, err));
    }
}}

test!{ fn TestWriterWriteString(t) {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut w = bufio::NewWriter(&mut buf);
        let (n, err) = w.WriteString("hello, world");
        if err != nil { t.Errorf(Sprintf!("WriteString: %s", err)); }
        if n != 12 { t.Errorf(Sprintf!("n = %d, want 12", n)); }
        if w.Flush() != nil {
            t.Errorf(Sprintf!("Flush err"));
        }
    }
    if buf != b"hello, world" {
        t.Errorf(Sprintf!("buf = %q", String::from_utf8_lossy(&buf)));
    }
}}

test!{ fn TestWriterWriteByte(t) {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut w = bufio::NewWriter(&mut buf);
        for b in b"abcdef" { let _ = w.WriteByte(*b); }
        let _ = w.Flush();
    }
    if buf != b"abcdef" {
        t.Errorf(Sprintf!("buf = %q", String::from_utf8_lossy(&buf)));
    }
}}

test!{ fn TestWriterFlushRequired(t) {
    // Without Flush, the backing writer must not see data (until BufWriter
    // is full, which doesn't happen for small writes). Verify content is
    // nonexistent before Flush, and present after.
    let mut buf: Vec<u8> = Vec::new();
    let mut w = bufio::NewWriter(&mut buf);
    let _ = w.WriteString("abc");
    // buf is borrowed; we can't check it mid-flight. Just verify Flush
    // succeeds and the final state matches.
    let err = w.Flush();
    if err != nil { t.Errorf(Sprintf!("Flush: %s", err)); }
    drop(w);
    if buf != b"abc" {
        t.Errorf(Sprintf!("buf after Flush = %q", String::from_utf8_lossy(&buf)));
    }
}}

test!{ fn TestReaderReadStringEmpty(t) {
    let mut r = bufio::NewReader(Cursor::new(Vec::<u8>::new()));
    let (s, err) = r.ReadString(b'\n');
    if err == nil { t.Errorf(Sprintf!("expected EOF")); }
    if s != "" { t.Errorf(Sprintf!("got %q", s)); }
}}

test!{ fn TestWriterMultipleWrites(t) {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut w = bufio::NewWriter(&mut buf);
        for _ in 0..100 {
            let _ = w.WriteString("abc");
        }
        let _ = w.Flush();
    }
    if buf.len() != 300 {
        t.Errorf(Sprintf!("buf len = %d, want 300", buf.len()));
    }
}}
