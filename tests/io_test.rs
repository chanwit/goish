// Port of go1.25.5 src/io/io_test.go and multi_test.go — core functions.
//
// Elided: tests that exercise Go-specific optimization paths
//   - TestCopyReadFrom / TestCopyWriteTo: require ReadFrom/WriteTo traits
//     with specific fall-through semantics; our Copy goes through generic
//     Read/Write and does not need the fast path.
//   - TestCopyPriority / TestNopCloserWriterToForwarding / TestMultiWriter_* : likewise
//   - TestOffsetWriter_* : no OffsetWriter type yet.

#![allow(non_snake_case)]
use goish::prelude::*;
use goish::io as gio;
use std::io::Cursor;

test!{ fn TestCopy(t) {
    let mut src = Cursor::new(b"hello, world.".to_vec());
    let mut dst: Vec<u8> = Vec::new();
    let (n, err) = gio::Copy(&mut dst, &mut src);
    if err != nil { t.Errorf(Sprintf!("Copy err: %s", err)); }
    if n != 13 { t.Errorf(Sprintf!("Copy n = %d, want 13", n)); }
    if dst != b"hello, world." {
        t.Errorf(Sprintf!("dst content = %s, want hello, world.", bytes::String(&dst)));
    }
}}

test!{ fn TestCopyN(t) {
    let mut src = Cursor::new(b"0123456789".to_vec());
    let mut dst: Vec<u8> = Vec::new();
    let (n, err) = gio::CopyN(&mut dst, &mut src, 5);
    if err != nil { t.Errorf(Sprintf!("CopyN err: %s", err)); }
    if n != 5 { t.Errorf(Sprintf!("CopyN n = %d, want 5", n)); }
    if dst != b"01234" {
        t.Errorf(Sprintf!("CopyN dst = %s, want 01234", bytes::String(&dst)));
    }
}}

test!{ fn TestCopyNEOF(t) {
    // Request 20 bytes from a 10-byte source — should return EOF and n=10.
    let mut src = Cursor::new(b"0123456789".to_vec());
    let mut dst: Vec<u8> = Vec::new();
    let (n, err) = gio::CopyN(&mut dst, &mut src, 20);
    if n != 10 { t.Errorf(Sprintf!("CopyN n = %d, want 10", n)); }
    // Should be EOF (not nil).
    if err == nil { t.Errorf(Sprintf!("CopyN err = nil, want EOF")); }
}}

test!{ fn TestReadAtLeast(t) {
    let mut src = Cursor::new(b"hello".to_vec());
    let mut buf = [0u8; 5];
    let (n, err) = gio::ReadAtLeast(&mut src, &mut buf, 3);
    if err != nil { t.Errorf(Sprintf!("ReadAtLeast err: %s", err)); }
    if n < 3 { t.Errorf(Sprintf!("ReadAtLeast n = %d, want >= 3", n)); }
}}

test!{ fn TestReadAtLeastShortBuffer(t) {
    let mut src = Cursor::new(b"hello".to_vec());
    let mut buf = [0u8; 2];
    let (_, err) = gio::ReadAtLeast(&mut src, &mut buf, 4);
    if err == nil {
        t.Errorf(Sprintf!("ReadAtLeast with short buffer should err, got nil"));
    }
}}

test!{ fn TestReadAtLeastUnexpectedEOF(t) {
    // Source has 2 bytes, need at least 4 → unexpected EOF.
    let mut src = Cursor::new(b"ab".to_vec());
    let mut buf = [0u8; 4];
    let (n, err) = gio::ReadAtLeast(&mut src, &mut buf, 4);
    if err == nil { t.Errorf(Sprintf!("expected err, got nil (n=%d)", n)); }
    let es = Sprintf!("%v", err);
    if !strings::Contains(&es, "unexpected EOF") && !strings::Contains(&es, "EOF") {
        t.Errorf(Sprintf!("expected EOF-related error, got %s", es));
    }
}}

test!{ fn TestReadFull(t) {
    let mut src = Cursor::new(b"abcdefgh".to_vec());
    let mut buf = [0u8; 5];
    let (n, err) = gio::ReadFull(&mut src, &mut buf);
    if err != nil { t.Errorf(Sprintf!("ReadFull err: %s", err)); }
    if n != 5 { t.Errorf(Sprintf!("ReadFull n = %d, want 5", n)); }
    if &buf != b"abcde" {
        t.Errorf(Sprintf!("ReadFull buf = %s, want abcde", bytes::String(&buf)));
    }
}}

test!{ fn TestWriteString(t) {
    let mut dst: Vec<u8> = Vec::new();
    let (n, err) = gio::WriteString(&mut dst, "hello");
    if err != nil { t.Errorf(Sprintf!("WriteString err: %s", err)); }
    if n != 5 { t.Errorf(Sprintf!("WriteString n = %d, want 5", n)); }
    if dst != b"hello" {
        t.Errorf(Sprintf!("WriteString dst = %s", bytes::String(&dst)));
    }
}}

test!{ fn TestLimitReader(t) {
    let src = Cursor::new(b"0123456789".to_vec());
    let mut lr = gio::LimitReader(src, 4);
    let mut buf = [0u8; 10];
    let (n, _) = lr.Read(&mut buf);
    if n != 4 { t.Errorf(Sprintf!("LimitReader first read n = %d, want 4", n)); }
    let (n2, err) = lr.Read(&mut buf);
    if n2 != 0 || err == nil {
        t.Errorf(Sprintf!("LimitReader second read = (%d, %s), want (0, EOF)", n2, err));
    }
}}

test!{ fn TestTeeReader(t) {
    let src = Cursor::new(b"hello".to_vec());
    let mut mirror: Vec<u8> = Vec::new();
    let mut tr = gio::TeeReader(src, &mut mirror);
    let (buf, err) = gio::ReadAll(&mut tr);
    if err != nil { t.Errorf(Sprintf!("ReadAll err: %s", err)); }
    if buf != b"hello" {
        t.Errorf(Sprintf!("TeeReader ReadAll = %s", bytes::String(&buf)));
    }
    if mirror != b"hello" {
        t.Errorf(Sprintf!("TeeReader mirror = %s", bytes::String(&mirror)));
    }
}}

test!{ fn TestSectionReader_ReadAt(t) {
    let data: Vec<u8> = b"0123456789".to_vec();
    let sr = gio::NewSectionReader(data, 2, 5);
    let mut buf = [0u8; 5];
    let (n, err) = sr.ReadAt(&mut buf, 0);
    if err != nil { t.Errorf(Sprintf!("ReadAt err: %s", err)); }
    if n != 5 { t.Errorf(Sprintf!("ReadAt n = %d, want 5", n)); }
    if &buf != b"23456" {
        t.Errorf(Sprintf!("SectionReader content = %s, want 23456", bytes::String(&buf)));
    }
}}

test!{ fn TestSectionReader_Size(t) {
    let data: Vec<u8> = b"0123456789".to_vec();
    let sr = gio::NewSectionReader(data, 2, 7);
    if sr.Size() != 7 { t.Errorf(Sprintf!("Size = %d, want 7", sr.Size())); }
}}

test!{ fn TestSectionReader_Seek(t) {
    let data: Vec<u8> = b"0123456789".to_vec();
    let mut sr = gio::NewSectionReader(data, 2, 5);
    // Seek from start to offset 1 (absolute: base+1 = 3)
    let (p, err) = sr.Seek(1, gio::SeekStart);
    if err != nil { t.Errorf(Sprintf!("Seek err: %s", err)); }
    if p != 1 { t.Errorf(Sprintf!("Seek pos = %d, want 1", p)); }
    let mut buf = [0u8; 3];
    let (n, _) = sr.Read(&mut buf);
    if n != 3 || &buf != b"345" {
        t.Errorf(Sprintf!("after Seek, Read = %s n=%d", bytes::String(&buf), n));
    }
}}

test!{ fn TestMultiReader(t) {
    let mut mr = gio::MultiReader(vec![
        Cursor::new(b"hello ".to_vec()),
        Cursor::new(b"world".to_vec()),
    ]);
    let (out, err) = gio::ReadAll(&mut mr);
    if err != nil { t.Errorf(Sprintf!("ReadAll err: %s", err)); }
    if out != b"hello world" {
        t.Errorf(Sprintf!("MultiReader content = %s", bytes::String(&out)));
    }
}}

test!{ fn TestMultiReaderCopy(t) {
    let mut mr = gio::MultiReader(vec![
        Cursor::new(b"abc".to_vec()),
        Cursor::new(b"def".to_vec()),
    ]);
    let mut dst: Vec<u8> = Vec::new();
    let (n, err) = gio::Copy(&mut dst, &mut mr);
    if err != nil { t.Errorf(Sprintf!("Copy err: %s", err)); }
    if n != 6 { t.Errorf(Sprintf!("Copy n = %d, want 6", n)); }
    if dst != b"abcdef" {
        t.Errorf(Sprintf!("MultiReader Copy content = %s", bytes::String(&dst)));
    }
}}

test!{ fn TestMultiReaderFinalEOF(t) {
    // Empty MultiReader returns EOF immediately.
    let mut mr = gio::MultiReader(Vec::<Cursor<Vec<u8>>>::new());
    let mut buf = [0u8; 4];
    let (n, err) = mr.Read(&mut buf);
    if n != 0 || err == nil {
        t.Errorf(Sprintf!("empty MultiReader: got (%d, %s), want (0, EOF)", n, err));
    }
}}

test!{ fn TestMultiWriter(t) {
    // Heterogeneous sinks: pre-boxed trait objects still work thanks to
    // the Reader/Writer blanket impls for Box<T>.
    let dst1: Vec<u8> = Vec::new();
    let dst2: Vec<u8> = Vec::new();
    let boxed1: Box<dyn gio::Writer + Send> = Box::new(dst1);
    let boxed2: Box<dyn gio::Writer + Send> = Box::new(dst2);
    let mut mw = gio::MultiWriter(vec![boxed1, boxed2]);
    let (n, err) = mw.Write(b"fanout");
    if err != nil { t.Errorf(Sprintf!("MultiWriter err: %s", err)); }
    if n != 6 { t.Errorf(Sprintf!("MultiWriter n = %d, want 6", n)); }
}}

test!{ fn TestNopCloserReadable(t) {
    let mut nc = gio::NopCloser(Cursor::new(b"abc".to_vec()));
    let (buf, err) = gio::ReadAll(&mut nc);
    if err != nil { t.Errorf(Sprintf!("NopCloser ReadAll err: %s", err)); }
    if buf != b"abc" {
        t.Errorf(Sprintf!("NopCloser content = %s", bytes::String(&buf)));
    }
}}

test!{ fn TestDiscard(t) {
    let mut d = gio::Discard();
    let (n, err) = d.Write(b"thrown away");
    if err != nil { t.Errorf(Sprintf!("Discard err: %s", err)); }
    if n != 11 { t.Errorf(Sprintf!("Discard n = %d, want 11", n)); }
}}
