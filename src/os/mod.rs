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

// Sub-packages.
pub mod exec;

/// os.Args — the command line, argv[0] is the program name (like Go).
#[allow(non_snake_case)]
pub fn Args() -> slice<string> {
    std::env::args().map(string::from).collect()
}

/// os.Getenv(key) — returns "" if unset (matching Go).
#[allow(non_snake_case)]
pub fn Getenv(key: impl AsRef<str>) -> string {
    std::env::var(key.as_ref()).map(string::from).unwrap_or_default()
}

/// os.LookupEnv(key) — like Getenv, but also reports whether the var was set.
#[allow(non_snake_case)]
pub fn LookupEnv(key: impl AsRef<str>) -> (string, bool) {
    match std::env::var(key.as_ref()) {
        Ok(v) => (v.into(), true),
        Err(_) => ("".into(), false),
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

/// os.Environ() — the environment as a list of "key=value" strings.
#[allow(non_snake_case)]
pub fn Environ() -> slice<string> {
    std::env::vars().map(|(k, v)| string::from(format!("{}={}", k, v))).collect()
}

/// os.Clearenv() — removes all environment variables for the current process.
#[allow(non_snake_case)]
pub fn Clearenv() {
    let keys: Vec<String> = std::env::vars().map(|(k, _)| k).collect();
    for k in keys {
        unsafe { std::env::remove_var(k); }
    }
}

/// os.Expand(s, mapping) — replaces $var or ${var} in s with mapping(var).
#[allow(non_snake_case)]
pub fn Expand(s: impl AsRef<str>, mapping: impl Fn(&str) -> string) -> string {
    let s = s.as_ref();
    let bytes = s.as_bytes();
    let mut out = std::string::String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            let (name, w) = get_shell_name(&bytes[i + 1..]);
            if w == 0 {
                // $ not followed by a valid name char (e.g. "$+", "$}").
                out.push('$');
                i += 1;
                continue;
            }
            if !name.is_empty() {
                out.push_str(&mapping(&name));
            }
            i += 1 + w;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out.into()
}

/// os.ExpandEnv(s) — shorthand for Expand(s, os.Getenv).
#[allow(non_snake_case)]
pub fn ExpandEnv(s: impl AsRef<str>) -> string {
    Expand(s, |k| Getenv(k))
}

fn get_shell_name(s: &[u8]) -> (String, usize) {
    if s.is_empty() { return (String::new(), 0); }
    if s[0] == b'{' {
        // ${...}: scan until } or end.
        if s.len() < 2 { return (String::new(), 1); }
        if s[1] == b'}' { return (String::new(), 2); } // invalid ${}; consume 2
        let mut i = 1;
        while i < s.len() && s[i] != b'}' {
            if !is_shell_special_var(s[i]) && !is_alpha_num(s[i]) {
                // invalid — Go returns "" and consumes chars up to here.
                return (String::new(), i + 1);
            }
            i += 1;
        }
        if i >= s.len() {
            // No closing brace: Go's implementation eats the characters.
            return (String::new(), s.len());
        }
        let name = std::str::from_utf8(&s[1..i]).unwrap_or("").into();
        (name, i + 1)
    } else if is_shell_special_var(s[0]) {
        let name = (s[0] as char).into();
        (name, 1)
    } else if is_alpha_num(s[0]) {
        let mut i = 0;
        while i < s.len() && is_alpha_num(s[i]) { i += 1; }
        let name = std::str::from_utf8(&s[..i]).unwrap_or("").into();
        (name, i)
    } else {
        (String::new(), 0)
    }
}

fn is_shell_special_var(c: u8) -> bool {
    matches!(c, b'*' | b'#' | b'$' | b'@' | b'!' | b'?' | b'-') || (c >= b'0' && c <= b'9')
}

fn is_alpha_num(c: u8) -> bool {
    c == b'_' || (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || (c >= b'0' && c <= b'9')
}

/// os.Hostname() → (name, error)
#[allow(non_snake_case)]
pub fn Hostname() -> (string, error) {
    // std::env doesn't expose hostname; read from env or /etc/hostname.
    if let Ok(v) = std::env::var("HOSTNAME") {
        if !v.is_empty() {
            return (v.into(), nil);
        }
    }
    match std::fs::read_to_string("/etc/hostname") {
        Ok(s) => (s.trim().into(), nil),
        Err(e) => ("".into(), New(&format!("os.Hostname: {}", e))),
    }
}

/// os.Getwd() → (dir, error)
#[allow(non_snake_case)]
pub fn Getwd() -> (string, error) {
    match std::env::current_dir() {
        Ok(p) => (p.to_string_lossy().into_owned().into(), nil),
        Err(e) => ("".into(), New(&format!("os.Getwd: {}", e))),
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
pub fn ReadFile(path: impl AsRef<str>) -> (crate::types::slice<crate::types::byte>, error) {
    match std::fs::read(path.as_ref()) {
        Ok(b) => (b.into(), nil),
        Err(e) => (crate::types::slice::new(), New(&format!("os.ReadFile: {}", e))),
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
    std::env::temp_dir().to_string_lossy().into_owned().into()
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
        Ok(f) => (File { inner: Some(f), name: p.into() }, nil),
        Err(e) => (
            File { inner: None, name: p.into() },
            New(&format!("os.Open: {}", e)),
        ),
    }
}

/// os.Create(path) — create (or truncate) for writing.
#[allow(non_snake_case)]
pub fn Create(path: impl AsRef<str>) -> (File, error) {
    let p = path.as_ref();
    match std::fs::File::create(p) {
        Ok(f) => (File { inner: Some(f), name: p.into() }, nil),
        Err(e) => (
            File { inner: None, name: p.into() },
            New(&format!("os.Create: {}", e)),
        ),
    }
}

// ── Standard streams ───────────────────────────────────────────────────
//
// Each handle is an opaque Goish newtype around the std equivalent. The
// std type names (`StdinLock<'static>`, `std::io::Stdout`, `std::io::Stderr`)
// no longer surface in return-position tooltips. `Read`/`Write` impls
// delegate to the inner so `bufio::NewReader(os::Stdin())`,
// `io::Copy(&mut os::Stdout(), &mut r)` etc. keep working unchanged.

#[allow(non_snake_case)]
pub struct StdinT {
    #[doc(hidden)]
    pub inner: std::io::StdinLock<'static>,
}

impl std::io::Read for StdinT {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> { self.inner.read(buf) }
}

#[allow(non_snake_case)]
pub struct StdoutT {
    #[doc(hidden)]
    pub inner: std::io::Stdout,
}

impl std::io::Write for StdoutT {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { self.inner.write(buf) }
    fn flush(&mut self) -> std::io::Result<()> { self.inner.flush() }
}

#[allow(non_snake_case)]
pub struct StderrT {
    #[doc(hidden)]
    pub inner: std::io::Stderr,
}

impl std::io::Write for StderrT {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { self.inner.write(buf) }
    fn flush(&mut self) -> std::io::Result<()> { self.inner.flush() }
}

/// os.Stdin — returns a lock on the process's stdin.
#[allow(non_snake_case)]
pub fn Stdin() -> StdinT {
    StdinT { inner: std::io::stdin().lock() }
}

/// os.Stdout — stdout handle.
#[allow(non_snake_case)]
pub fn Stdout() -> StdoutT {
    StdoutT { inner: std::io::stdout() }
}

/// os.Stderr — stderr handle.
#[allow(non_snake_case)]
pub fn Stderr() -> StderrT {
    StderrT { inner: std::io::stderr() }
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
