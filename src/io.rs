// io: Go's io package — Reader/Writer interfaces + Copy/ReadAll/MultiReader/
// MultiWriter/TeeReader/SectionReader/LimitReader/CopyN/ReadFull/ReadAtLeast.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   n, err := r.Read(buf)               let (n, err) = r.Read(&mut buf);
//   n, err := w.Write(buf)              let (n, err) = w.Write(buf);
//   n, err := io.Copy(dst, src)         let (n, err) = io::Copy(&mut dst, &mut src);
//   if err == io.EOF { ... }            if err == io::EOF { ... }
//
// Reader/Writer are our own traits so we can layer streams (MultiReader,
// LimitReader, TeeReader) in pure Rust. Blanket impls make any
// std::io::Read / std::io::Write compatible.

use crate::errors::{error, nil, New};
use crate::types::{byte, int, int64};
use std::sync::{Arc, Mutex};

/// io.EOF — returned by Reader.Read when no more input is available.
#[allow(non_snake_case)]
pub fn EOF() -> error { New("EOF") }

#[allow(non_snake_case)]
pub fn ErrUnexpectedEOF() -> error { New("unexpected EOF") }

#[allow(non_snake_case)]
pub fn ErrShortWrite() -> error { New("short write") }

#[allow(non_snake_case)]
pub fn ErrShortBuffer() -> error { New("short buffer") }

/// True if err is an EOF sentinel (message == "EOF").
pub fn is_eof(e: &error) -> bool {
    if *e == nil { return false; }
    format!("{}", e) == "EOF"
}

// ─── Reader / Writer traits ───────────────────────────────────────────

pub trait Reader {
    fn Read(&mut self, p: &mut [byte]) -> (int, error);
}

pub trait Writer {
    fn Write(&mut self, p: &[byte]) -> (int, error);
}

pub trait Closer {
    fn Close(&mut self) -> error;
}

pub trait Seeker {
    /// Seek sets the offset for the next read/write to offset, relative
    /// to whence (0=start, 1=current, 2=end).
    fn Seek(&mut self, offset: int64, whence: int) -> (int64, error);
}

pub trait ReaderAt {
    fn ReadAt(&self, p: &mut [byte], off: int64) -> (int, error);
}

pub trait WriterAt {
    fn WriteAt(&mut self, p: &[byte], off: int64) -> (int, error);
}

// Forwarding impls for `Box<dyn Reader/Writer + Send>` so MultiReader /
// MultiWriter accept pre-boxed trait objects as well as bare concrete
// readers/writers. Bare `Box<T>` (with `T` concrete) already works via
// the `impl<R: std::io::Read> Reader for R` blanket.

impl Reader for Box<dyn Reader + Send> {
    fn Read(&mut self, p: &mut [byte]) -> (int, error) { (**self).Read(p) }
}
impl Writer for Box<dyn Writer + Send> {
    fn Write(&mut self, p: &[byte]) -> (int, error) { (**self).Write(p) }
}

// ─── Combined interfaces (Go embedding) ───────────────────────────────
//
// Go composes io interfaces via struct embedding, e.g.:
//
//   type ReadCloser interface { Reader; Closer }
//
// In Rust we express each combination as a supertrait with a blanket
// impl, so any type implementing the constituent traits automatically
// implements the combined one. No Box-gymnastics at the impl site.

pub trait ReadCloser: Reader + Closer {}
impl<T: Reader + Closer + ?Sized> ReadCloser for T {}

pub trait WriteCloser: Writer + Closer {}
impl<T: Writer + Closer + ?Sized> WriteCloser for T {}

pub trait ReadWriter: Reader + Writer {}
impl<T: Reader + Writer + ?Sized> ReadWriter for T {}

pub trait ReadWriteCloser: Reader + Writer + Closer {}
impl<T: Reader + Writer + Closer + ?Sized> ReadWriteCloser for T {}

pub trait ReadSeeker: Reader + Seeker {}
impl<T: Reader + Seeker + ?Sized> ReadSeeker for T {}

pub trait WriteSeeker: Writer + Seeker {}
impl<T: Writer + Seeker + ?Sized> WriteSeeker for T {}

pub trait ReadWriteSeeker: Reader + Writer + Seeker {}
impl<T: Reader + Writer + Seeker + ?Sized> ReadWriteSeeker for T {}

pub trait ReadSeekCloser: Reader + Seeker + Closer {}
impl<T: Reader + Seeker + Closer + ?Sized> ReadSeekCloser for T {}

// ─── Seek whence constants ────────────────────────────────────────────

#[allow(non_upper_case_globals)] pub const SeekStart: int = 0;
#[allow(non_upper_case_globals)] pub const SeekCurrent: int = 1;
#[allow(non_upper_case_globals)] pub const SeekEnd: int = 2;

// ─── Blanket impls for std traits ─────────────────────────────────────

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

// Seeker blanket over std::io::Seek — lets Cursor, File, etc. flow into
// the combined interfaces (ReadSeeker, ReadWriteSeeker, ...).
impl<S: std::io::Seek + ?Sized> Seeker for S {
    fn Seek(&mut self, offset: int64, whence: int) -> (int64, error) {
        let pos = match whence {
            0 => std::io::SeekFrom::Start(offset as u64),
            1 => std::io::SeekFrom::Current(offset),
            2 => std::io::SeekFrom::End(offset),
            _ => return (0, New(&format!("seek: invalid whence {}", whence))),
        };
        match std::io::Seek::seek(self, pos) {
            Ok(n) => (n as int64, nil),
            Err(e) => (0, New(&format!("{}", e))),
        }
    }
}

// ─── Copy / ReadAll / WriteString ─────────────────────────────────────

#[allow(non_snake_case)]
pub fn Copy<W: Writer + ?Sized, R: Reader + ?Sized>(
    dst: &mut W, src: &mut R,
) -> (int64, error) {
    copy_buffer_impl(dst, src, None, None)
}

#[allow(non_snake_case)]
pub fn CopyN<W: Writer + ?Sized, R: Reader + ?Sized>(
    dst: &mut W, src: &mut R, n: int64,
) -> (int64, error) {
    let (written, err) = copy_buffer_impl(dst, src, None, Some(n));
    if written == n { return (n, nil); }
    if written < n && err == nil {
        return (written, EOF());
    }
    (written, err)
}

#[allow(non_snake_case)]
pub fn CopyBuffer<W: Writer + ?Sized, R: Reader + ?Sized>(
    dst: &mut W, src: &mut R, buf: Option<&mut [byte]>,
) -> (int64, error) {
    match buf {
        Some(b) if b.is_empty() => panic!("empty buffer in CopyBuffer"),
        _ => {}
    }
    copy_buffer_impl(dst, src, buf, None)
}

fn copy_buffer_impl<W: Writer + ?Sized, R: Reader + ?Sized>(
    dst: &mut W, src: &mut R, buf: Option<&mut [byte]>, limit: Option<int64>,
) -> (int64, error) {
    const DEFAULT: usize = 32 * 1024;
    let mut owned_buf: Vec<byte>;
    let buffer: &mut [byte] = match buf {
        Some(b) => b,
        None => {
            owned_buf = vec![0; DEFAULT];
            &mut owned_buf[..]
        }
    };
    let mut written: int64 = 0;
    loop {
        let n_req = match limit {
            Some(l) if (l - written) < buffer.len() as int64 => (l - written) as usize,
            _ => buffer.len(),
        };
        if limit == Some(written) { break; }
        let (nr, er) = src.Read(&mut buffer[..n_req]);
        if nr > 0 {
            let (nw, ew) = dst.Write(&buffer[..nr as usize]);
            if nw < 0 || nw > nr {
                return (written, New("invalid Write result"));
            }
            written += nw as int64;
            if ew != nil { return (written, ew); }
            if nr != nw { return (written, ErrShortWrite()); }
        }
        if er != nil {
            if er == EOF() { return (written, nil); }
            return (written, er);
        }
    }
    (written, nil)
}

#[allow(non_snake_case)]
pub fn ReadAll<R: Reader + ?Sized>(r: &mut R) -> (crate::types::slice<byte>, error) {
    let mut buf: Vec<byte> = Vec::with_capacity(512);
    let mut scratch = [0u8; 4096];
    loop {
        let (n, e) = r.Read(&mut scratch);
        if n > 0 { buf.extend_from_slice(&scratch[..n as usize]); }
        if e != nil {
            if e == EOF() { return (buf.into(), nil); }
            return (buf.into(), e);
        }
    }
}

/// Like `ReadAll`, but returns a `string` instead of `Vec<byte>` — saves
/// the caller a `bytes::String(...)` round-trip on the common case of
/// reading a text body. Non-UTF-8 bytes go through lossy decode.
#[allow(non_snake_case)]
pub fn ReadAllString<R: Reader + ?Sized>(r: &mut R) -> (crate::types::string, error) {
    let (b, e) = ReadAll(r);
    (crate::types::string::from(b), e)
}

#[allow(non_snake_case)]
pub fn ReadFull<R: Reader + ?Sized>(r: &mut R, buf: &mut [byte]) -> (int, error) {
    ReadAtLeast(r, buf, buf.len() as int)
}

#[allow(non_snake_case)]
pub fn ReadAtLeast<R: Reader + ?Sized>(r: &mut R, buf: &mut [byte], min: int) -> (int, error) {
    if (buf.len() as int) < min {
        return (0, ErrShortBuffer());
    }
    let mut n: int = 0;
    while n < min {
        let (nn, err) = r.Read(&mut buf[n as usize..]);
        n += nn;
        if err != nil {
            if err == EOF() && n >= min { return (n, nil); }
            if err == EOF() && n > 0 { return (n, ErrUnexpectedEOF()); }
            return (n, err);
        }
    }
    (n, nil)
}

#[allow(non_snake_case)]
pub fn WriteString<W: Writer + ?Sized>(w: &mut W, s: impl AsRef<str>) -> (int, error) {
    w.Write(s.as_ref().as_bytes())
}

// ─── LimitReader ──────────────────────────────────────────────────────

pub struct LimitedReader<R> { pub R: R, pub N: int64 }

impl<R: Reader> Reader for LimitedReader<R> {
    fn Read(&mut self, p: &mut [byte]) -> (int, error) {
        if self.N <= 0 { return (0, EOF()); }
        let want = if (p.len() as int64) > self.N { self.N as usize } else { p.len() };
        let (n, err) = self.R.Read(&mut p[..want]);
        self.N -= n as int64;
        (n, err)
    }
}

#[allow(non_snake_case)]
pub fn LimitReader<R: Reader>(r: R, n: int64) -> LimitedReader<R> {
    LimitedReader { R: r, N: n }
}

// ─── MultiReader ──────────────────────────────────────────────────────

pub struct MultiReaderT {
    readers: Vec<Box<dyn Reader + Send>>,
}

impl Reader for MultiReaderT {
    fn Read(&mut self, p: &mut [byte]) -> (int, error) {
        while !self.readers.is_empty() {
            // Treat nested MultiReader specially to match Go's flatten.
            let (n, err) = self.readers[0].Read(p);
            if err == EOF() {
                self.readers.remove(0);
                if n > 0 { return (n, nil); }
                continue;
            }
            return (n, err);
        }
        (0, EOF())
    }
}

#[allow(non_snake_case)]
pub fn MultiReader<I, R>(readers: I) -> MultiReaderT
where
    I: IntoIterator<Item = R>,
    R: Reader + Send + 'static,
{
    MultiReaderT {
        readers: readers
            .into_iter()
            .map(|r| Box::new(r) as Box<dyn Reader + Send>)
            .collect(),
    }
}

// ─── MultiWriter ──────────────────────────────────────────────────────

pub struct MultiWriterT {
    writers: Vec<Box<dyn Writer + Send>>,
}

impl Writer for MultiWriterT {
    fn Write(&mut self, p: &[byte]) -> (int, error) {
        for w in &mut self.writers {
            let (n, err) = w.Write(p);
            if err != nil { return (n, err); }
            if (n as usize) != p.len() { return (n, ErrShortWrite()); }
        }
        (p.len() as int, nil)
    }
}

#[allow(non_snake_case)]
pub fn MultiWriter<I, W>(writers: I) -> MultiWriterT
where
    I: IntoIterator<Item = W>,
    W: Writer + Send + 'static,
{
    MultiWriterT {
        writers: writers
            .into_iter()
            .map(|w| Box::new(w) as Box<dyn Writer + Send>)
            .collect(),
    }
}

// ─── TeeReader ────────────────────────────────────────────────────────

pub struct TeeReaderT<R, W> { pub r: R, pub w: W }

impl<R: Reader, W: Writer> Reader for TeeReaderT<R, W> {
    fn Read(&mut self, p: &mut [byte]) -> (int, error) {
        let (n, err) = self.r.Read(p);
        if n > 0 {
            let (_, werr) = self.w.Write(&p[..n as usize]);
            if werr != nil { return (n, werr); }
        }
        (n, err)
    }
}

#[allow(non_snake_case)]
pub fn TeeReader<R: Reader, W: Writer>(r: R, w: W) -> TeeReaderT<R, W> {
    TeeReaderT { r, w }
}

// ─── SectionReader ────────────────────────────────────────────────────

pub struct SectionReaderT<R: ReaderAt> {
    r: R,
    base: int64,
    off: int64,
    limit: int64,
}

impl<R: ReaderAt> SectionReaderT<R> {
    pub fn Size(&self) -> int64 { self.limit - self.base }
}

impl<R: ReaderAt> Reader for SectionReaderT<R> {
    fn Read(&mut self, p: &mut [byte]) -> (int, error) {
        if self.off >= self.limit { return (0, EOF()); }
        let max = (self.limit - self.off) as usize;
        let want = if p.len() > max { max } else { p.len() };
        let (n, err) = self.r.ReadAt(&mut p[..want], self.off);
        self.off += n as int64;
        (n, err)
    }
}

impl<R: ReaderAt> ReaderAt for SectionReaderT<R> {
    fn ReadAt(&self, p: &mut [byte], off: int64) -> (int, error) {
        if off < 0 || off >= self.Size() { return (0, EOF()); }
        let real_off = off + self.base;
        let max = (self.limit - real_off) as usize;
        if max <= 0 { return (0, EOF()); }
        let want = if p.len() > max { max } else { p.len() };
        let (n, err) = self.r.ReadAt(&mut p[..want], real_off);
        if (n as usize) < p.len() && err == nil {
            return (n, EOF());
        }
        (n, err)
    }
}

impl<R: ReaderAt> Seeker for SectionReaderT<R> {
    fn Seek(&mut self, offset: int64, whence: int) -> (int64, error) {
        let new_off = match whence {
            SeekStart => self.base + offset,
            SeekCurrent => self.off + offset,
            SeekEnd => self.limit + offset,
            _ => return (0, New("io.SectionReader.Seek: invalid whence")),
        };
        if new_off < self.base { return (0, New("io.SectionReader.Seek: negative position")); }
        self.off = new_off;
        (new_off - self.base, nil)
    }
}

#[allow(non_snake_case)]
pub fn NewSectionReader<R: ReaderAt>(r: R, off: int64, n: int64) -> SectionReaderT<R> {
    // Saturate limit to base+n (matches Go's saturation when off+n overflows).
    let rem_max = i64::MAX - off;
    let limit = off + if n > rem_max { rem_max } else { n };
    SectionReaderT { r, base: off, off, limit }
}

// ─── NopCloser / Discard ──────────────────────────────────────────────

pub struct NopCloserT<R> { pub r: R }

impl<R: Reader> Reader for NopCloserT<R> {
    fn Read(&mut self, p: &mut [byte]) -> (int, error) { self.r.Read(p) }
}
impl<R> Closer for NopCloserT<R> {
    fn Close(&mut self) -> error { nil }
}

#[allow(non_snake_case)]
pub fn NopCloser<R: Reader>(r: R) -> NopCloserT<R> { NopCloserT { r } }

/// io.Discard — a Writer that successfully consumes anything.
pub struct DiscardT;

impl Writer for DiscardT {
    fn Write(&mut self, p: &[byte]) -> (int, error) { (p.len() as int, nil) }
}

#[allow(non_upper_case_globals)]
pub const Discard: fn() -> DiscardT = || DiscardT;

// ─── Pipe ─────────────────────────────────────────────────────────────

struct PipeShared {
    buf: Vec<byte>,
    read_err: error,
    write_err: error,
    closed: bool,
}

pub struct PipeReaderT { inner: Arc<Mutex<PipeShared>> }
pub struct PipeWriterT { inner: Arc<Mutex<PipeShared>> }

impl Reader for PipeReaderT {
    fn Read(&mut self, p: &mut [byte]) -> (int, error) {
        // Blocking loop on internal buffer. Simple implementation.
        loop {
            let mut g = self.inner.lock().unwrap();
            if !g.buf.is_empty() {
                let n = std::cmp::min(p.len(), g.buf.len());
                p[..n].copy_from_slice(&g.buf[..n]);
                g.buf.drain(..n);
                return (n as int, nil);
            }
            if g.closed { return (0, EOF()); }
            if g.write_err != nil { return (0, g.write_err.clone()); }
            drop(g);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
impl Closer for PipeReaderT {
    fn Close(&mut self) -> error {
        let mut g = self.inner.lock().unwrap();
        g.read_err = New("io: read/write on closed pipe");
        g.closed = true;
        nil
    }
}

impl Writer for PipeWriterT {
    fn Write(&mut self, p: &[byte]) -> (int, error) {
        let mut g = self.inner.lock().unwrap();
        if g.closed || g.read_err != nil {
            return (0, if g.read_err != nil { g.read_err.clone() } else { New("io: pipe closed") });
        }
        g.buf.extend_from_slice(p);
        (p.len() as int, nil)
    }
}
impl Closer for PipeWriterT {
    fn Close(&mut self) -> error {
        let mut g = self.inner.lock().unwrap();
        g.closed = true;
        nil
    }
}

#[allow(non_snake_case)]
pub fn Pipe() -> (PipeReaderT, PipeWriterT) {
    let inner = Arc::new(Mutex::new(PipeShared {
        buf: Vec::new(), read_err: nil, write_err: nil, closed: false,
    }));
    (PipeReaderT { inner: inner.clone() }, PipeWriterT { inner })
}

// ─── ByteReader wrapper for &[byte] (ReaderAt support) ────────────────

impl ReaderAt for Vec<byte> {
    fn ReadAt(&self, p: &mut [byte], off: int64) -> (int, error) {
        if off < 0 { return (0, New("io: negative offset")); }
        if (off as usize) >= self.len() { return (0, EOF()); }
        let start = off as usize;
        let avail = self.len() - start;
        let n = std::cmp::min(avail, p.len());
        p[..n].copy_from_slice(&self[start..start + n]);
        if n < p.len() { (n as int, EOF()) } else { (n as int, nil) }
    }
}

impl ReaderAt for &[byte] {
    fn ReadAt(&self, p: &mut [byte], off: int64) -> (int, error) {
        if off < 0 { return (0, New("io: negative offset")); }
        if (off as usize) >= self.len() { return (0, EOF()); }
        let start = off as usize;
        let avail = self.len() - start;
        let n = std::cmp::min(avail, p.len());
        p[..n].copy_from_slice(&self[start..start + n]);
        if n < p.len() { (n as int, EOF()) } else { (n as int, nil) }
    }
}

impl<const N: usize> ReaderAt for &[byte; N] {
    fn ReadAt(&self, p: &mut [byte], off: int64) -> (int, error) {
        (self.as_slice()).ReadAt(p, off)
    }
}

impl ReaderAt for crate::types::slice<byte> {
    fn ReadAt(&self, p: &mut [byte], off: int64) -> (int, error) {
        self.as_slice().ReadAt(p, off)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn combined_interfaces_blanket_impl() {
        // Cursor<Vec<u8>> gets Read+Seek+Write via std, and from our blanket
        // impls Reader+Writer+Seeker — so ReadWriteSeeker (combined trait)
        // also applies automatically.
        fn takes_rws<T: ReadWriteSeeker>(_: &mut T) {}
        let mut cur = Cursor::new(vec![0u8; 8]);
        takes_rws(&mut cur);
    }

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

    #[test]
    fn limit_reader_stops_at_n() {
        let src = Cursor::new(b"abcdefgh".to_vec());
        let mut lr = LimitReader(src, 3);
        let mut buf = [0u8; 8];
        let (n, _) = lr.Read(&mut buf);
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], b"abc");
    }

    #[test]
    fn tee_reader_mirrors_bytes() {
        let src = Cursor::new(b"tee me".to_vec());
        let mut copied: Vec<u8> = Vec::new();
        let mut tr = TeeReader(src, &mut copied);
        let mut out = Vec::new();
        std::io::copy(&mut ReaderToStd(&mut tr), &mut out).unwrap();
        // (can't use goish::Copy into goish writer easily across std io here)
        assert_eq!(&copied[..], b"tee me");
        assert_eq!(&out[..], b"tee me");
    }

    // Adapter so our goish Reader can be handed to std::io::copy.
    struct ReaderToStd<'a, R: Reader>(&'a mut R);
    impl<'a, R: Reader> std::io::Read for ReaderToStd<'a, R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let (n, err) = self.0.Read(buf);
            if err != nil && err != EOF() {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err)));
            }
            Ok(n as usize)
        }
    }

    #[test]
    fn read_full_errors_unexpected_eof() {
        let mut src = Cursor::new(b"ab".to_vec());
        let mut buf = [0u8; 4];
        let (n, err) = ReadFull(&mut src, &mut buf);
        assert_eq!(n, 2);
        assert!(err != nil);
    }

    #[test]
    fn copy_n_stops() {
        let mut src = Cursor::new(b"abcdefgh".to_vec());
        let mut dst: Vec<u8> = Vec::new();
        let (n, err) = CopyN(&mut dst, &mut src, 4);
        assert!(err == nil);
        assert_eq!(n, 4);
        assert_eq!(dst, b"abcd");
    }

    #[test]
    fn section_reader_basic() {
        let data: Vec<u8> = b"0123456789".to_vec();
        let sr = NewSectionReader(data, 3, 4);
        assert_eq!(sr.Size(), 4);
        let mut p = [0u8; 10];
        let (n, _) = sr.ReadAt(&mut p, 0);
        assert_eq!(n, 4);
        assert_eq!(&p[..4], b"3456");
    }
}
