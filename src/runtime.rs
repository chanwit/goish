// runtime: Go's runtime package — a minimal subset of introspection helpers.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   runtime.NumCPU()                    runtime::NumCPU()
//   runtime.GOMAXPROCS(0)               runtime::GOMAXPROCS(0)
//   runtime.Gosched()                   runtime::Gosched()
//   runtime.GOOS                        runtime::GOOS
//   runtime.GOARCH                      runtime::GOARCH
//   runtime.NumGoroutine()              runtime::NumGoroutine()   // best-effort
//   runtime.Version()                   runtime::Version()
//
// Caveat: NumGoroutine counts live goroutines started through `go!`
// (best-effort). Each goroutine is a tokio async task (~200 B); runtime
// scales to 1M per process (tests/million_goroutines.rs).

use crate::types::int;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const GOOS: &str = std::env::consts::OS;
pub const GOARCH: &str = std::env::consts::ARCH;

static GOMAXPROCS_SETTING: AtomicUsize = AtomicUsize::new(0);
pub(crate) static LIVE_GOROUTINES: AtomicUsize = AtomicUsize::new(1); // main counts as 1

#[allow(non_snake_case)]
pub fn NumCPU() -> int {
    std::thread::available_parallelism()
        .map(|n| n.get() as int)
        .unwrap_or(1)
}

/// GOMAXPROCS(n) — if n > 0, sets the stored value and returns the previous;
/// if n == 0, returns the current setting (defaulting to NumCPU() if unset).
#[allow(non_snake_case)]
pub fn GOMAXPROCS(n: int) -> int {
    let prev = GOMAXPROCS_SETTING.load(Ordering::SeqCst);
    let effective = if prev == 0 { NumCPU() as usize } else { prev };
    if n > 0 {
        GOMAXPROCS_SETTING.store(n as usize, Ordering::SeqCst);
    }
    effective as int
}

/// Gosched — yield the current thread.
#[allow(non_snake_case)]
pub fn Gosched() {
    std::thread::yield_now();
}

/// NumGoroutine — best-effort live goroutine count (including main).
#[allow(non_snake_case)]
pub fn NumGoroutine() -> int {
    LIVE_GOROUTINES.load(Ordering::SeqCst) as int
}

#[allow(non_snake_case)]
pub fn Version() -> &'static str {
    // Not semantically the Go toolchain version; report goish crate + note.
    concat!("goish-", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_cpu_positive() {
        assert!(NumCPU() >= 1);
    }

    #[test]
    fn gomaxprocs_roundtrip() {
        let prev = GOMAXPROCS(0);
        assert!(prev >= 1);
        let prev2 = GOMAXPROCS(4);
        assert_eq!(prev2, prev);
        assert_eq!(GOMAXPROCS(0), 4);
        // restore
        GOMAXPROCS(prev);
    }

    #[test]
    fn gosched_runs() {
        Gosched();
    }

    #[test]
    fn os_and_arch_nonempty() {
        assert!(!GOOS.is_empty());
        assert!(!GOARCH.is_empty());
    }

    #[test]
    fn version_format() {
        assert!(Version().starts_with("goish-"));
    }
}
