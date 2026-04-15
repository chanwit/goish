// io: Go's io package — Reader/Writer interfaces + Copy/EOF.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   n, err := r.Read(buf)               let (n, err) = r.Read(&mut buf);
//   n, err := w.Write(buf)              let (n, err) = w.Write(buf);
//   n, err := io.Copy(dst, src)         let (n, err) = io::Copy(&mut dst, &mut src);
//   if err == io.EOF { ... }            if errors::Is(&err, &io::EOF) { ... }
//
// Thin Go-named layer over `std::io::Read` / `std::io::Write`. Any type
// implementing those std traits automatically satisfies goish's Reader/Writer.

use crate::errors::{error, nil, New};
use crate::types::{byte, int, int64};

/// io.EOF — returned by Reader.Read when no more input is available.
/// Compare with errors::Is(&err, &io::EOF).
pub fn EOF() -> error {
    New("EOF")
}

pub trait Reader {
    fn Read(&mut self, p: &mut [byte]) -> (int, error);
}

pub trait Writer {
    fn Write(&mut self, p: &[byte]) -> (int, error);
}

// Blanket impls: anything that impls std::io::Read is a goish Reader,
// anything that impls std::io::Write is a goish Writer.

impl<R: std::io::Read + ?Sized> Reader for R {
    fn Read(&mut self, p: &mut [byte]) -> (int, error) {
        match std::io::Read::read(self, p) {
            Ok(0) if !p.is_empty() => (0, EOF()),
            Ok(n) => (n as int, nil),
            Err(e) => (0, New(&format!("{}", e))),
        }
    }
}

impl<W: std::io::Write + ?Sized> Writer for W {
    fn Write(&mut self, p: &[byte]) -> (int, error) {
        match std::io::Write::write(self, p) {
            Ok(n) => (n as int, nil),
            Err(e) => (0, New(&format!("{}", e))),
        }
    }
}

/// io.Copy(dst, src) — stream every byte from src to dst.
#[allow(non_snake_case)]
pub fn Copy<W: std::io::Write + ?Sized, R: std::io::Read + ?Sized>(
    dst: &mut W,
    src: &mut R,
) -> (int64, error) {
    match std::io::copy(src, dst) {
        Ok(n) => (n as int64, nil),
        Err(e) => (0, New(&format!("io.Copy: {}", e))),
    }
}

/// io.ReadAll(r) — read until EOF, return the full contents.
#[allow(non_snake_case)]
pub fn ReadAll<R: std::io::Read + ?Sized>(r: &mut R) -> (Vec<byte>, error) {
    let mut buf = Vec::new();
    match r.read_to_end(&mut buf) {
        Ok(_) => (buf, nil),
        Err(e) => (buf, New(&format!("io.ReadAll: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reader_read_from_cursor() {
        let mut cur = Cursor::new(b"hello".to_vec());
        let mut buf = [0u8; 5];
        let (n, err) = cur.Read(&mut buf);
        assert!(err == nil);
        assert_eq!(n, 5);
        assert_eq!(&buf, b"hello");
    }

    #[test]
    fn writer_write_to_vec() {
        let mut v: Vec<u8> = Vec::new();
        let (n, err) = v.Write(b"abc");
        assert!(err == nil);
        assert_eq!(n, 3);
        assert_eq!(v, b"abc");
    }

    #[test]
    fn copy_streams_bytes() {
        let mut src = Cursor::new(b"payload".to_vec());
        let mut dst: Vec<u8> = Vec::new();
        let (n, err) = Copy(&mut dst, &mut src);
        assert!(err == nil);
        assert_eq!(n, 7);
        assert_eq!(dst, b"payload");
    }

    #[test]
    fn read_all_returns_full_contents() {
        let mut src = Cursor::new(b"goish".to_vec());
        let (buf, err) = ReadAll(&mut src);
        assert!(err == nil);
        assert_eq!(buf, b"goish");
    }
}
