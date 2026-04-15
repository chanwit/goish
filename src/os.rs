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
}
