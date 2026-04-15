// os: Go's os package, ported (subset).
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   args := os.Args                     let args = os::Args();
//   host := os.Getenv("HOST")           let host = os::Getenv("HOST");
//   os.Setenv("K", "v")                 os::Setenv("K", "v");
//   os.Unsetenv("K")                    os::Unsetenv("K");
//   os.Exit(1)                          os::Exit(1);
//   name, err := os.Hostname()          let (name, err) = os::Hostname();
//   dir, err := os.Getwd()              let (dir, err) = os::Getwd();

use crate::errors::{error, nil, New};
use crate::types::{int, slice, string};

/// os.Args — the command line, argv[0] is the program name (like Go).
#[allow(non_snake_case)]
pub fn Args() -> slice<string> {
    std::env::args().collect()
}

/// os.Getenv(key) — returns "" if unset (matching Go).
#[allow(non_snake_case)]
pub fn Getenv(key: impl AsRef<str>) -> string {
    std::env::var(key.as_ref()).unwrap_or_default()
}

/// os.LookupEnv(key) — like Getenv, but also reports whether the var was set.
#[allow(non_snake_case)]
pub fn LookupEnv(key: impl AsRef<str>) -> (string, bool) {
    match std::env::var(key.as_ref()) {
        Ok(v) => (v, true),
        Err(_) => (String::new(), false),
    }
}

/// os.Setenv(key, value)
#[allow(non_snake_case)]
pub fn Setenv(key: impl AsRef<str>, val: impl AsRef<str>) -> error {
    // Safe — setenv is safe in single-threaded contexts. Tests use distinct
    // env var names to avoid races.
    unsafe {
        std::env::set_var(key.as_ref(), val.as_ref());
    }
    nil
}

/// os.Unsetenv(key)
#[allow(non_snake_case)]
pub fn Unsetenv(key: impl AsRef<str>) -> error {
    unsafe {
        std::env::remove_var(key.as_ref());
    }
    nil
}

/// os.Exit(code) — terminate the process immediately. Does not run defers.
#[allow(non_snake_case)]
pub fn Exit(code: int) -> ! {
    std::process::exit(code as i32)
}

/// os.Hostname() → (name, error)
#[allow(non_snake_case)]
pub fn Hostname() -> (string, error) {
    // std::env doesn't expose hostname; read from env or /etc/hostname.
    if let Ok(v) = std::env::var("HOSTNAME") {
        if !v.is_empty() {
            return (v, nil);
        }
    }
    match std::fs::read_to_string("/etc/hostname") {
        Ok(s) => (s.trim().to_string(), nil),
        Err(e) => (String::new(), New(&format!("os.Hostname: {}", e))),
    }
}

/// os.Getwd() → (dir, error)
#[allow(non_snake_case)]
pub fn Getwd() -> (string, error) {
    match std::env::current_dir() {
        Ok(p) => (p.to_string_lossy().into_owned(), nil),
        Err(e) => (String::new(), New(&format!("os.Getwd: {}", e))),
    }
}

/// os.Chdir(path)
#[allow(non_snake_case)]
pub fn Chdir(path: impl AsRef<str>) -> error {
    match std::env::set_current_dir(path.as_ref()) {
        Ok(()) => nil,
        Err(e) => New(&format!("os.Chdir: {}", e)),
    }
}

// ── File I/O helpers ────────────────────────────────────────────────

/// os.ReadFile(path) → (bytes, error)
#[allow(non_snake_case)]
pub fn ReadFile(path: impl AsRef<str>) -> (Vec<crate::types::byte>, error) {
    match std::fs::read(path.as_ref()) {
        Ok(b) => (b, nil),
        Err(e) => (Vec::new(), New(&format!("os.ReadFile: {}", e))),
    }
}

/// os.WriteFile(path, data, perm) — perm is ignored on Windows.
#[allow(non_snake_case)]
pub fn WriteFile(path: impl AsRef<str>, data: &[crate::types::byte], _perm: u32) -> error {
    match std::fs::write(path.as_ref(), data) {
        Ok(()) => nil,
        Err(e) => New(&format!("os.WriteFile: {}", e)),
    }
}

/// os.Remove(path) — remove a single file or empty directory.
#[allow(non_snake_case)]
pub fn Remove(path: impl AsRef<str>) -> error {
    let p = path.as_ref();
    // Try file first, fall back to empty dir.
    match std::fs::remove_file(p) {
        Ok(()) => nil,
        Err(_) => match std::fs::remove_dir(p) {
            Ok(()) => nil,
            Err(e) => New(&format!("os.Remove: {}", e)),
        },
    }
}

/// os.RemoveAll(path) — recursive remove.
#[allow(non_snake_case)]
pub fn RemoveAll(path: impl AsRef<str>) -> error {
    let p = path.as_ref();
    let md = match std::fs::metadata(p) {
        Ok(m) => m,
        Err(_) => return nil,  // not existing → Go returns nil
    };
    let r = if md.is_dir() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    };
    match r {
        Ok(()) => nil,
        Err(e) => New(&format!("os.RemoveAll: {}", e)),
    }
}

/// os.Mkdir(path, perm)
#[allow(non_snake_case)]
pub fn Mkdir(path: impl AsRef<str>, _perm: u32) -> error {
    match std::fs::create_dir(path.as_ref()) {
        Ok(()) => nil,
        Err(e) => New(&format!("os.Mkdir: {}", e)),
    }
}

/// os.MkdirAll(path, perm)
#[allow(non_snake_case)]
pub fn MkdirAll(path: impl AsRef<str>, _perm: u32) -> error {
    match std::fs::create_dir_all(path.as_ref()) {
        Ok(()) => nil,
        Err(e) => New(&format!("os.MkdirAll: {}", e)),
    }
}

/// os.TempDir() — system temp directory.
#[allow(non_snake_case)]
pub fn TempDir() -> string {
    std::env::temp_dir().to_string_lossy().into_owned()
}

// ── File handle ────────────────────────────────────────────────────────
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   f, err := os.Open("/path")          let (mut f, err) = os::Open("/path");
//   f, err := os.Create("/path")        let (mut f, err) = os::Create("/path");
//   defer f.Close()                     defer!{ f.Close(); }
//   n, err := f.Read(buf)               let (n, err) = f.Read(&mut buf);
//   n, err := f.Write(data)             let (n, err) = f.Write(data);
//   name := f.Name()                    let name = f.Name();

use std::io::{Read as _, Seek as _, Write as _};

pub struct File {
    inner: Option<std::fs::File>,
    name: string,
}

impl File {
    /// f.Name() — the path the file was opened with.
    #[allow(non_snake_case)]
    pub fn Name(&self) -> string {
        self.name.clone()
    }

    /// f.Read(buf) — (n, error). EOF is reported via n=0 + a non-nil error.
    #[allow(non_snake_case)]
    pub fn Read(&mut self, buf: &mut [crate::types::byte]) -> (int, error) {
        let f = match self.inner.as_mut() {
            Some(f) => f,
            None => return (0, New("os.File: already closed")),
        };
        match f.read(buf) {
            Ok(0) if !buf.is_empty() => (0, New("EOF")),
            Ok(n) => (n as int, nil),
            Err(e) => (0, New(&format!("os.File.Read: {}", e))),
        }
    }

    /// f.Write(data) — (n, error).
    #[allow(non_snake_case)]
    pub fn Write(&mut self, data: &[crate::types::byte]) -> (int, error) {
        let f = match self.inner.as_mut() {
            Some(f) => f,
            None => return (0, New("os.File: already closed")),
        };
        match f.write(data) {
            Ok(n) => (n as int, nil),
            Err(e) => (0, New(&format!("os.File.Write: {}", e))),
        }
    }

    /// f.Close() — releases the OS handle.
    #[allow(non_snake_case)]
    pub fn Close(&mut self) -> error {
        self.inner = None;
        nil
    }

    /// f.Sync() — fsync the file.
    #[allow(non_snake_case)]
    pub fn Sync(&self) -> error {
        match &self.inner {
            Some(f) => match f.sync_all() {
                Ok(()) => nil,
                Err(e) => New(&format!("os.File.Sync: {}", e)),
            },
            None => New("os.File: already closed"),
        }
    }

    /// f.Seek(offset, whence) — whence: 0=start, 1=current, 2=end.
    #[allow(non_snake_case)]
    pub fn Seek(&mut self, offset: int, whence: int) -> (crate::types::int64, error) {
        let f = match self.inner.as_mut() {
            Some(f) => f,
            None => return (0, New("os.File: already closed")),
        };
        let pos = match whence {
            0 => std::io::SeekFrom::Start(offset as u64),
            1 => std::io::SeekFrom::Current(offset as i64),
            2 => std::io::SeekFrom::End(offset as i64),
            _ => return (0, New(&format!("os.File.Seek: invalid whence {}", whence))),
        };
        match f.seek(pos) {
            Ok(n) => (n as crate::types::int64, nil),
            Err(e) => (0, New(&format!("os.File.Seek: {}", e))),
        }
    }
}

// Make File work with io::Writer / io::Reader blanket impls + Fprintf!.
impl std::io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.inner.as_mut() {
            Some(f) => f.read(buf),
            None => Err(std::io::Error::new(std::io::ErrorKind::Other, "closed")),
        }
    }
}

impl std::io::Write for File {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.inner.as_mut() {
            Some(f) => f.write(buf),
            None => Err(std::io::Error::new(std::io::ErrorKind::Other, "closed")),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self.inner.as_mut() {
            Some(f) => f.flush(),
            None => Ok(()),
        }
    }
}

/// os.Open(path) — open for reading.
#[allow(non_snake_case)]
pub fn Open(path: impl AsRef<str>) -> (File, error) {
    let p = path.as_ref();
    match std::fs::File::open(p) {
        Ok(f) => (File { inner: Some(f), name: p.to_string() }, nil),
        Err(e) => (
            File { inner: None, name: p.to_string() },
            New(&format!("os.Open: {}", e)),
        ),
    }
}

/// os.Create(path) — create (or truncate) for writing.
#[allow(non_snake_case)]
pub fn Create(path: impl AsRef<str>) -> (File, error) {
    let p = path.as_ref();
    match std::fs::File::create(p) {
        Ok(f) => (File { inner: Some(f), name: p.to_string() }, nil),
        Err(e) => (
            File { inner: None, name: p.to_string() },
            New(&format!("os.Create: {}", e)),
        ),
    }
}

// ── Standard streams ───────────────────────────────────────────────────

/// os.Stdin — returns a lock on the process's stdin. Implements std::io::Read
/// so goish::io::Reader and bufio::NewScanner accept it directly.
#[allow(non_snake_case)]
pub fn Stdin() -> std::io::StdinLock<'static> {
    std::io::stdin().lock()
}

/// os.Stdout — stdout handle. Implements std::io::Write.
#[allow(non_snake_case)]
pub fn Stdout() -> std::io::Stdout {
    std::io::stdout()
}

/// os.Stderr — stderr handle.
#[allow(non_snake_case)]
pub fn Stderr() -> std::io::Stderr {
    std::io::stderr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_has_at_least_program_name() {
        let a = Args();
        assert!(!a.is_empty());
    }

    #[test]
    fn getenv_missing_returns_empty() {
        let v = Getenv("GOISH_DEFINITELY_NOT_SET_12345");
        assert_eq!(v, "");
    }

    #[test]
    fn setenv_getenv_roundtrip() {
        let key = "GOISH_TEST_SETENV_1";
        let _ = Setenv(key, "hello");
        assert_eq!(Getenv(key), "hello");
        let _ = Unsetenv(key);
        assert_eq!(Getenv(key), "");
    }

    #[test]
    fn lookupenv_ok_flag() {
        let key = "GOISH_TEST_LOOKUP_1";
        let (_, ok) = LookupEnv(key);
        assert!(!ok);
        let _ = Setenv(key, "x");
        let (v, ok) = LookupEnv(key);
        assert!(ok);
        assert_eq!(v, "x");
        let _ = Unsetenv(key);
    }

    #[test]
    fn getwd_not_empty() {
        let (dir, err) = Getwd();
        assert!(err == nil);
        assert!(!dir.is_empty());
    }

    #[test]
    fn write_read_remove_file() {
        let tmp = TempDir();
        let path = format!("{}/goish_test_os_rw.txt", tmp);
        let err = WriteFile(&path, b"hello goish", 0o644);
        assert!(err == nil);
        let (data, err) = ReadFile(&path);
        assert!(err == nil);
        assert_eq!(data, b"hello goish");
        let err = Remove(&path);
        assert!(err == nil);
    }

    #[test]
    fn mkdir_removeall_roundtrip() {
        let tmp = TempDir();
        let path = format!("{}/goish_test_os_mkdir/nested/deep", tmp);
        let err = MkdirAll(&path, 0o755);
        assert!(err == nil);
        let top = format!("{}/goish_test_os_mkdir", tmp);
        let err = RemoveAll(&top);
        assert!(err == nil);
    }

    #[test]
    fn file_create_write_open_read_remove() {
        let tmp = TempDir();
        let path = format!("{}/goish_test_file.txt", tmp);

        let (mut f, err) = Create(&path);
        assert!(err == nil);
        assert_eq!(f.Name(), path);
        let (n, err) = f.Write(b"hello file");
        assert!(err == nil);
        assert_eq!(n, 10);
        let err = f.Close();
        assert!(err == nil);

        let (mut f, err) = Open(&path);
        assert!(err == nil);
        let mut buf = [0u8; 20];
        let (n, err) = f.Read(&mut buf);
        assert!(err == nil);
        assert_eq!(&buf[..n as usize], b"hello file");
        let _ = f.Close();

        let _ = Remove(&path);
    }

    #[test]
    fn file_seek() {
        let tmp = TempDir();
        let path = format!("{}/goish_test_seek.txt", tmp);
        let (mut f, _) = Create(&path);
        let _ = f.Write(b"0123456789");
        let (pos, err) = f.Seek(3, 0); // SEEK_SET
        assert!(err == nil);
        assert_eq!(pos, 3);
        let _ = f.Write(b"XY");
        let _ = f.Close();

        let (mut f, _) = Open(&path);
        let mut buf = [0u8; 10];
        let (_, _) = f.Read(&mut buf);
        assert_eq!(&buf, b"012XY56789");
        let _ = f.Close();
        let _ = Remove(&path);
    }
}
