// testing: Port of Go's `testing` package — goal is to let real Go tests be
// ported to goish line-by-line.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   func TestFoo(t *testing.T) { … }    test!{ fn TestFoo(t) { … } }
//   t.Errorf("got %d", got)             t.Errorf(Sprintf!("got %d", got))
//   t.Error("bad")                      t.Error("bad")
//   t.Fatalf("no way %s", why)          t.Fatalf(Sprintf!("no way %s", why))
//   t.Fatal(err)                        t.Fatal(err)
//   t.Logf("info %d", n)                t.Logf(Sprintf!("info %d", n))
//   t.Log("info")                       t.Log("info")
//   t.Skipf("slow %s", why)             t.Skipf(Sprintf!("slow %s", why))
//   t.Skip("slow")                      t.Skip("slow")
//   t.SkipNow()                         t.SkipNow()
//   t.Helper()                          t.Helper()           ← no-op today
//   t.Name()                            t.Name()
//   t.Failed()                          t.Failed()
//   t.Skipped()                         t.Skipped()
//   t.Cleanup(fn)                       t.Cleanup(|| { … })
//   t.Run("case", func(t *testing.T))   t.Run("case", |t| { … })
//
//   func TestMain(m *testing.M) { … }   test_main!{ fn TestMain(m) { … } }
//
// The format variants (Errorf/Fatalf/Logf/Skipf) are *methods* on T that
// accept a preformatted string. Users wrap the format spec with the
// existing `Sprintf!` macro. This keeps the method name Go-identical
// while avoiding a name collision with `fmt::Errorf!` (which already
// occupies the top-level macro namespace).

use crate::types::{int, string};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// ── T: test handle ─────────────────────────────────────────────────────

pub struct T {
    name: String,
    failed: AtomicBool,
    skipped: AtomicBool,
    logbuf: Mutex<String>,
    cleanups: Mutex<Vec<Box<dyn FnOnce() + Send>>>,
    // Subtest failures roll up into parent via set_failed_parent when Run
    // returns.
    sub_failures: AtomicBool,
}

impl T {
    #[doc(hidden)]
    pub fn new(name: impl Into<String>) -> T {
        T {
            name: name.into(),
            failed: AtomicBool::new(false),
            skipped: AtomicBool::new(false),
            logbuf: Mutex::new(String::new()),
            cleanups: Mutex::new(Vec::new()),
            sub_failures: AtomicBool::new(false),
        }
    }

    /// t.Name() — the test's name path.
    #[allow(non_snake_case)]
    pub fn Name(&self) -> string {
        self.name.clone().into()
    }

    /// t.Failed() — whether this test (or any of its subtests) has failed.
    #[allow(non_snake_case)]
    pub fn Failed(&self) -> bool {
        self.failed.load(Ordering::SeqCst) || self.sub_failures.load(Ordering::SeqCst)
    }

    /// t.Skipped() — whether this test was skipped.
    #[allow(non_snake_case)]
    pub fn Skipped(&self) -> bool {
        self.skipped.load(Ordering::SeqCst)
    }

    /// t.Context() — returns a Context scoped to this test.
    ///
    /// Mirrors Go 1.24+ `*testing.T.Context()`. Today we return a fresh
    /// `Background()` on every call, which is sufficient for the common
    /// port use-case (passing a context to code under test). The Go
    /// contract additionally cancels the returned context just before
    /// cleanups run; that wiring is tracked for a follow-up once T owns
    /// a lifetime-bound CancelFunc.
    #[allow(non_snake_case)]
    pub fn Context(&self) -> crate::context::Context {
        crate::context::Background()
    }

    /// t.Log(msg) — append msg to the test's log buffer. Only printed on failure.
    #[allow(non_snake_case)]
    pub fn Log(&self, msg: impl AsRef<str>) {
        self.append_log(msg.as_ref());
    }

    /// t.Error(msg) — log + mark failed; continue.
    #[allow(non_snake_case)]
    pub fn Error(&self, msg: impl AsRef<str>) {
        self.append_log(msg.as_ref());
        self.failed.store(true, Ordering::SeqCst);
    }

    /// t.Errorf(msg) — identical to Error in goish; the `f` suffix preserves
    /// Go's naming. Typical use: `t.Errorf(Sprintf!("got %d", x))`.
    #[allow(non_snake_case)]
    pub fn Errorf(&self, msg: impl AsRef<str>) {
        self.Error(msg);
    }

    /// t.Fatal(msg) — log + mark failed + stop this test immediately.
    #[allow(non_snake_case)]
    pub fn Fatal(&self, msg: impl AsRef<str>) -> ! {
        self.append_log(msg.as_ref());
        self.failed.store(true, Ordering::SeqCst);
        self.abort(Abort::FailNow);
    }

    /// t.Fatalf(msg) — alias for Fatal. Typical use: `t.Fatalf(Sprintf!(...))`.
    #[allow(non_snake_case)]
    pub fn Fatalf(&self, msg: impl AsRef<str>) -> ! { self.Fatal(msg) }

    /// t.Logf(msg) — alias for Log. Typical use: `t.Logf(Sprintf!(...))`.
    #[allow(non_snake_case)]
    pub fn Logf(&self, msg: impl AsRef<str>) { self.Log(msg) }

    /// t.Skip(msg) — log + mark skipped + stop this test immediately.
    #[allow(non_snake_case)]
    pub fn Skip(&self, msg: impl AsRef<str>) -> ! {
        self.append_log(msg.as_ref());
        self.skipped.store(true, Ordering::SeqCst);
        self.abort(Abort::SkipNow);
    }

    /// t.Skipf(msg) — alias for Skip. Typical use: `t.Skipf(Sprintf!(...))`.
    #[allow(non_snake_case)]
    pub fn Skipf(&self, msg: impl AsRef<str>) -> ! { self.Skip(msg) }

    /// t.FailNow() — mark failed + stop (equivalent to Fatal without message).
    #[allow(non_snake_case)]
    pub fn FailNow(&self) -> ! {
        self.failed.store(true, Ordering::SeqCst);
        self.abort(Abort::FailNow);
    }

    /// t.SkipNow() — mark skipped + stop.
    #[allow(non_snake_case)]
    pub fn SkipNow(&self) -> ! {
        self.skipped.store(true, Ordering::SeqCst);
        self.abort(Abort::SkipNow);
    }

    /// t.Fail() — mark failed, continue.
    #[allow(non_snake_case)]
    pub fn Fail(&self) {
        self.failed.store(true, Ordering::SeqCst);
    }

    /// t.Helper() — best-effort no-op in goish v0.4. Helpers aren't stripped
    /// from our traceback yet; kept as a stub so Go code compiles unchanged.
    #[allow(non_snake_case)]
    pub fn Helper(&self) {}

    /// t.Cleanup(f) — register a callback to run LIFO after this test returns.
    #[allow(non_snake_case)]
    pub fn Cleanup<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.cleanups.lock().unwrap().push(Box::new(f));
    }

    /// t.Parallel() — no-op under the default `#[test]` harness (tests already
    /// run in parallel threads as chosen by libtest). Present so Go code
    /// compiles unchanged.
    #[allow(non_snake_case)]
    pub fn Parallel(&self) {}

    /// t.Run(name, f) — run a subtest. Returns true iff the subtest passed.
    ///
    /// Failures in the subtest propagate the parent's Failed() without
    /// aborting the parent.
    #[allow(non_snake_case)]
    pub fn Run<F>(&self, name: impl AsRef<str>, f: F) -> bool
    where
        F: FnOnce(&T),
    {
        let full = format!("{}/{}", self.name, name.as_ref());
        let sub = T::new(full);
        let sub_ref = &sub;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f(sub_ref);
        }));
        // Run any cleanups the sub registered, even on panic.
        sub.run_cleanups();
        // Dump sub's log to parent so parent collects it.
        let sub_log = sub.logbuf.lock().unwrap().clone();
        if !sub_log.is_empty() {
            let mut g = self.logbuf.lock().unwrap();
            g.push_str(&sub_log);
        }
        let sub_failed = sub.Failed();
        let is_abort = matches!(&result, Err(e) if is_abort_panic(e));
        // Re-raise non-abort panics from the subtest.
        if let Err(e) = result {
            if !is_abort_panic(&e) { std::panic::resume_unwind(e); }
        }
        if sub_failed {
            self.sub_failures.store(true, Ordering::SeqCst);
        }
        !sub_failed && !is_abort
    }

    // ── Internals ──────────────────────────────────────────────────────

    #[doc(hidden)]
    pub fn append_log(&self, s: &str) {
        let mut g = self.logbuf.lock().unwrap();
        if !g.is_empty() && !g.ends_with('\n') { g.push('\n'); }
        g.push_str(s);
    }

    #[doc(hidden)]
    pub fn log_contents(&self) -> string {
        self.logbuf.lock().unwrap().clone().into()
    }

    #[doc(hidden)]
    pub fn log_contents_raw(&self) -> std::string::String {
        self.logbuf.lock().unwrap().clone()
    }

    #[doc(hidden)]
    pub fn run_cleanups(&self) {
        let mut g = self.cleanups.lock().unwrap();
        while let Some(f) = g.pop() { f(); }
    }

    fn abort(&self, kind: Abort) -> ! {
        std::panic::panic_any(kind);
    }

    /// Called by test! macro after the user body runs (possibly via panic).
    /// Returns `(true, "")` if the test is considered passing (or skipped),
    /// `(false, log)` otherwise.
    ///
    /// Comma-ok shape rather than `Result<(), string>` so the macro
    /// expansion stays clear of `std::result::Result` — the only place
    /// the std name would otherwise leak under this `#[doc(hidden)]` API.
    #[doc(hidden)]
    pub fn finish(&self, outcome: Outcome) -> (bool, string) {
        self.run_cleanups();
        match outcome {
            Outcome::Ok => {
                if self.Failed() {
                    (false, self.log_contents())
                } else {
                    (true, string::default())
                }
            }
            Outcome::Aborted => {
                if self.Skipped() && !self.failed.load(Ordering::SeqCst) {
                    (true, string::default())
                } else {
                    (false, self.log_contents())
                }
            }
            Outcome::Paniced(msg) => {
                let mut log = self.log_contents_raw();
                if !log.is_empty() && !log.ends_with('\n') { log.push('\n'); }
                log.push_str(&format!("panic: {}", msg));
                (false, log.into())
            }
        }
    }
}

/// Panic sentinel for Fatal/Skip that aborts the test function.
#[doc(hidden)]
#[derive(Debug)]
pub enum Abort { FailNow, SkipNow }

#[doc(hidden)]
pub fn is_abort_panic(e: &Box<dyn std::any::Any + Send>) -> bool {
    e.is::<Abort>()
}

#[doc(hidden)]
pub enum Outcome {
    Ok,
    Aborted,
    Paniced(String),
}

// ── test! macro: #[test] bridge ────────────────────────────────────────

/// `test!{ fn TestFoo(t) { … } }` — declares a `#[test]` test function whose
/// body gets a `&T` named `t`. Fatal/Skip/FailNow unwind via a sentinel panic
/// which the macro catches and converts to a PASS/FAIL/SKIP result.
#[macro_export]
macro_rules! test {
    (fn $name:ident ( $t:ident ) $body:block) => {
        #[test]
        #[allow(non_snake_case)]
        fn $name() {
            let __t = $crate::testing::T::new(stringify!($name));
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let $t: &$crate::testing::T = &__t;
                $body
            }));
            let finished = match outcome {
                Ok(()) => __t.finish($crate::testing::__priv::Outcome::Ok),
                Err(e) if $crate::runtime::is_goexit_panic(&e) => {
                    // Goexit: clean termination per Go semantics.
                    __t.finish($crate::testing::__priv::Outcome::Ok)
                }
                Err(e) if $crate::testing::__priv::is_abort_panic(&e) => {
                    __t.finish($crate::testing::__priv::Outcome::Aborted)
                }
                Err(e) => {
                    let msg: std::string::String = if let Some(s) = e.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    __t.finish($crate::testing::__priv::Outcome::Paniced(msg))
                }
            };
            let (passed, log) = finished;
            if !passed {
                panic!("{}", log);
            } else if __t.Skipped() {
                // libtest has no way to report "skipped"; just exit OK with a log line.
                eprintln!("--- SKIP: {} ({})", __t.Name(), __t.log_contents());
            }
        }

    };
}

/// `test_h!{ fn TestX(t) { … } }` — variant for **custom-harness** test
/// files (`harness = false`). Emits a plain function + an inventory
/// registration so `test_main!`'s generated `main()` can discover and
/// run it via `m.Run()`.
///
/// Use `test!` for files running under the default libtest harness
/// (the common case); switch to `test_h!` only in files where you've
/// set `harness = false` and are using `test_main!`.
///
/// Rationale: rustc's `#[test]` attribute (which `test!` emits) hides
/// the fn from ordinary module scope under a custom harness, which
/// breaks inventory's link-time fn-pointer capture.
#[macro_export]
macro_rules! test_h {
    (fn $name:ident ( $t:ident ) $body:block) => {
        #[allow(non_snake_case, dead_code)]
        fn $name() {
            let __t = $crate::testing::T::new(stringify!($name));
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let $t: &$crate::testing::T = &__t;
                $body
            }));
            let finished = match outcome {
                Ok(()) => __t.finish($crate::testing::__priv::Outcome::Ok),
                Err(e) if $crate::runtime::is_goexit_panic(&e) => {
                    // Goexit: clean termination per Go semantics.
                    __t.finish($crate::testing::__priv::Outcome::Ok)
                }
                Err(e) if $crate::testing::__priv::is_abort_panic(&e) => {
                    __t.finish($crate::testing::__priv::Outcome::Aborted)
                }
                Err(e) => {
                    let msg: std::string::String = if let Some(s) = e.downcast_ref::<&str>() {
                        (*s).to_string()
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    __t.finish($crate::testing::__priv::Outcome::Paniced(msg))
                }
            };
            let (passed, log) = finished;
            if !passed { panic!("{}", log); }
        }

        $crate::__goish_inventory::submit! {
            $crate::testing::RegisteredTest {
                name: stringify!($name),
                run: $name,
            }
        }
    };
}

// Expose Outcome / is_abort_panic to the test! macro without committing
// to a public API.
#[doc(hidden)]
pub mod __priv {
    pub use super::{is_abort_panic, Outcome};
}

// ── Short / Verbose flag accessors ─────────────────────────────────────

use std::sync::OnceLock;

fn flags() -> &'static Flags {
    static F: OnceLock<Flags> = OnceLock::new();
    F.get_or_init(Flags::parse)
}

struct Flags { short: bool, verbose: bool }

impl Flags {
    fn parse() -> Self {
        let mut short = false;
        let mut verbose = false;
        for a in std::env::args() {
            match a.as_str() {
                "-short" | "--short" | "-test.short" => short = true,
                "-v" | "--verbose" | "-test.v" => verbose = true,
                _ => {}
            }
        }
        Flags { short, verbose }
    }
}

#[allow(non_snake_case)]
pub fn Short() -> bool { flags().short }

#[allow(non_snake_case)]
pub fn Verbose() -> bool { flags().verbose }

/// testing.AllocsPerRun(runs, f) — stub returning 0 in v0.4.
/// Rust has no stable allocator introspection; tests that depend on this
/// value should use `if testing::AllocsPerRun(...) == 0.0` guards or skip.
#[allow(non_snake_case)]
pub fn AllocsPerRun<F: FnMut()>(_runs: int, mut f: F) -> f64 {
    f();
    0.0
}

// ── B (benchmark handle) ──────────────────────────────────────────────

use std::time::{Duration, Instant};

pub struct B {
    pub N: int,
    report_allocs: AtomicBool,
    bytes: std::sync::atomic::AtomicI64,
    timer_running: bool,
    elapsed: Duration,
    last_start: Option<Instant>,
    // Condition-style b.Loop() iteration state.
    loop_counter: int,
}

impl B {
    #[doc(hidden)]
    pub fn new(n: int) -> B {
        B {
            N: n,
            report_allocs: AtomicBool::new(false),
            bytes: std::sync::atomic::AtomicI64::new(0),
            timer_running: true,
            elapsed: Duration::ZERO,
            last_start: Some(Instant::now()),
            loop_counter: n,
        }
    }

    /// b.Loop() — condition-style iteration (Go 1.24+). Returns true while
    /// more iterations are needed.
    #[allow(non_snake_case)]
    pub fn Loop(&mut self) -> bool {
        if self.loop_counter > 0 {
            self.loop_counter -= 1;
            true
        } else {
            self.StopTimer();
            false
        }
    }

    /// b.ResetTimer() — discards measured time so far. Useful after expensive
    /// setup that shouldn't count toward the benchmark.
    #[allow(non_snake_case)]
    pub fn ResetTimer(&mut self) {
        self.elapsed = Duration::ZERO;
        if self.timer_running {
            self.last_start = Some(Instant::now());
        }
    }

    #[allow(non_snake_case)]
    pub fn StartTimer(&mut self) {
        if !self.timer_running {
            self.timer_running = true;
            self.last_start = Some(Instant::now());
        }
    }

    #[allow(non_snake_case)]
    pub fn StopTimer(&mut self) {
        if self.timer_running {
            if let Some(t) = self.last_start.take() {
                self.elapsed += t.elapsed();
            }
            self.timer_running = false;
        }
    }

    #[allow(non_snake_case)]
    pub fn ReportAllocs(&self) {
        self.report_allocs.store(true, Ordering::SeqCst);
    }

    /// b.SetBytes(n) — record per-iteration byte throughput for MB/s output.
    #[allow(non_snake_case)]
    pub fn SetBytes(&self, n: int) {
        self.bytes.store(n, Ordering::SeqCst);
    }

    /// Internal: finalize the benchmark and return a one-line report.
    #[doc(hidden)]
    pub fn report(&mut self, name: &str) -> String {
        self.StopTimer();
        let ns = self.elapsed.as_nanos() as f64;
        let ran = self.N as f64 - self.loop_counter as f64;
        let ran = if ran < 1.0 { self.N as f64 } else { ran };
        let ns_per_op = if ran > 0.0 { ns / ran } else { 0.0 };
        let mut s = format!("{:<40} {:>10} {:>14.2} ns/op",
            name, self.N - self.loop_counter, ns_per_op);
        let bytes = self.bytes.load(Ordering::SeqCst);
        if bytes > 0 && ns > 0.0 {
            let mb_per_s = (bytes as f64 * ran) / (ns / 1e9) / (1024.0 * 1024.0);
            s.push_str(&format!(" {:>8.2} MB/s", mb_per_s));
        }
        s
    }
}

/// `benchmark!{ fn BenchmarkX(b) { … } }` — registers a benchmark as a
/// regular `#[test]`.
///
/// - Runs with a default N of 1000. Override via the `GOISH_BENCH_N` env
///   var at runtime. The body can also use `while b.Loop() { … }` which
///   decrements the internal counter and honours StopTimer.
/// - On completion the ns/op line prints to stderr (libtest captures it
///   unless `--nocapture` is passed).
/// - `cargo test` still runs them; to skip benchmarks, filter by test name
///   prefix: `cargo test -- --skip Benchmark`.
#[macro_export]
macro_rules! benchmark {
    (fn $name:ident ( $b:ident ) $body:block) => {
        #[test]
        #[allow(non_snake_case)]
        fn $name() {
            let n: $crate::types::int = ::std::env::var("GOISH_BENCH_N")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000);
            let mut __b = $crate::testing::B::new(n);
            {
                let $b: &mut $crate::testing::B = &mut __b;
                $body
            }
            let line = __b.report(stringify!($name));
            ::std::eprintln!("{}", line);
        }
    };
}

// ── test_main! / M — real TestMain harness backed by `inventory` ──────
//
// Under the default `#[test]` harness, `test_main!` remains inert
// (the user body type-checks but never runs) — libtest owns `main()`.
//
// Under a custom harness (`harness = false` in `[[test]]` in Cargo.toml),
// `test_main!` expands to a real `fn main()` that:
//   1. Parses `-run` / `-v` / `-short` command-line flags
//   2. Constructs `M`
//   3. Executes the user's TestMain body (so setup/teardown runs)
//   4. User calls `m.Run()` which iterates every `test!` registered in
//      the `inventory` crate's linker table, running each one
//   5. `m.Run()` returns Go's style exit code (0 pass / 1 fail); the
//      user's TestMain typically ends with `os::Exit(m.Run())`.

/// A test registered by the `test!` macro. `inventory::submit!` stores
/// one of these per test at link time; `M::Run()` walks the whole list.
pub struct RegisteredTest {
    pub name: &'static str,
    pub run: fn(),
}

inventory::collect!(RegisteredTest);

/// Go's `*testing.M` — the value TestMain receives and calls `.Run()` on.
pub struct M {
    filter: Option<String>,
    verbose: bool,
}

impl M {
    #[doc(hidden)]
    pub fn new() -> Self {
        M {
            filter: std::env::args().find_map(|a| {
                a.strip_prefix("-run=")
                    .or_else(|| a.strip_prefix("-test.run="))
                    .map(|s| s.into())
            }),
            verbose: std::env::args().any(|a| {
                matches!(a.as_str(), "-v" | "--verbose" | "-test.v")
            }),
        }
    }

    /// `m.Run()` — run every registered test (optionally filtered by
    /// `-run=<regex>`). Returns 0 if all passed, 1 otherwise.
    #[allow(non_snake_case)]
    pub fn Run(&self) -> int {
        let pat: Option<crate::regexp::Regexp> =
            self.filter.as_deref().map(|p| {
                let (re, _err) = crate::regexp::Compile(p);
                re
            });

        let mut ran = 0usize;
        let mut failed = 0usize;
        for t in inventory::iter::<RegisteredTest>() {
            if let Some(re) = &pat {
                if !re.MatchString(t.name) { continue; }
            }
            if self.verbose { eprintln!("=== RUN   {}", t.name); }
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (t.run)()));
            ran += 1;
            match outcome {
                Ok(()) => {
                    if self.verbose { eprintln!("--- PASS: {}", t.name); }
                }
                Err(_) => {
                    failed += 1;
                    eprintln!("--- FAIL: {}", t.name);
                }
            }
        }
        eprintln!("TestMain: ran {} tests, {} failed", ran, failed);
        if failed == 0 { 0 } else { 1 }
    }
}

impl Default for M {
    fn default() -> Self { M::new() }
}

/// `test_main!{ fn TestMain(m) { … } }` — generate a Go-shape TestMain.
///
/// Behaviour depends on whether the test target is using the default
/// libtest harness or a custom one (`harness = false`):
///
///   - default harness:   user body is type-checked but never runs
///   - custom harness:    macro emits `fn main()` that runs the user
///                        body, which typically ends with `m.Run()`
///
/// To switch a test file into custom-harness mode, add to Cargo.toml:
///
///   [[test]]
///   name = "mytest"
///   path = "tests/mytest.rs"
///   harness = false
#[macro_export]
macro_rules! test_main {
    (fn $name:ident ( $m:ident ) $body:block) => {
        // User's TestMain body — usable both as an ordinary fn (under
        // default harness) and callable from the generated main() below
        // (under custom harness).
        #[allow(dead_code, non_snake_case)]
        fn $name(__m: &$crate::testing::M) {
            let $m: &$crate::testing::M = __m;
            $body
        }

        // Generate a main(). Under default harness this is harmless
        // (libtest provides its own main and the two don't collide
        // because `cargo test` uses `--test` which renames user main).
        // Under `harness = false`, this IS the entrypoint.
        #[allow(dead_code)]
        fn main() {
            let __m = $crate::testing::M::new();
            $name(&__m);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_name_and_flags() {
        let t = T::new("TestSelf");
        assert_eq!(t.Name(), "TestSelf");
        assert!(!t.Failed());
        assert!(!t.Skipped());
    }

    #[test]
    fn t_error_marks_failed() {
        let t = T::new("X");
        t.Error("oops");
        assert!(t.Failed());
        assert!(t.log_contents().contains("oops"));
    }

    #[test]
    fn t_fatal_aborts_via_panic() {
        let result = std::panic::catch_unwind(|| {
            let t = T::new("X");
            t.Fatal("boom");
        });
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(is_abort_panic(&e));
        }
    }

    #[test]
    fn t_skip_aborts_and_marks_skipped() {
        let t = std::sync::Arc::new(T::new("X"));
        let tt = t.clone();
        let result = std::panic::catch_unwind(move || {
            tt.Skip("slow");
        });
        assert!(result.is_err());
        assert!(t.Skipped());
    }

    #[test]
    fn cleanup_runs_lifo() {
        let log = std::sync::Arc::new(Mutex::new(Vec::<i32>::new()));
        let t = T::new("X");
        let l1 = log.clone(); t.Cleanup(move || l1.lock().unwrap().push(1));
        let l2 = log.clone(); t.Cleanup(move || l2.lock().unwrap().push(2));
        let l3 = log.clone(); t.Cleanup(move || l3.lock().unwrap().push(3));
        t.run_cleanups();
        assert_eq!(*log.lock().unwrap(), vec![3, 2, 1]);
    }

    #[test]
    fn subtest_failure_propagates_to_parent() {
        let t = T::new("Parent");
        let ok = t.Run("sub", |sub| {
            sub.Error("inner fail");
        });
        assert!(!ok);
        assert!(t.Failed());
    }

    #[test]
    fn subtest_pass_does_not_fail_parent() {
        let t = T::new("Parent");
        let ok = t.Run("sub", |_sub| { /* no assertion */ });
        assert!(ok);
        assert!(!t.Failed());
    }

    #[test]
    fn errorf_method_accepts_sprintf() {
        let t = T::new("X");
        t.Errorf(crate::fmt::Sprintf!("got %d want %d", 1, 2));
        assert!(t.Failed());
        let log = t.log_contents();
        assert!(log.contains("got 1 want 2"), "log = {:?}", log);
    }

    #[test]
    fn short_and_verbose_do_not_panic() {
        let _ = Short();
        let _ = Verbose();
    }

    #[test]
    fn b_loop_counts_down() {
        let mut b = B::new(3);
        let mut n = 0;
        while b.Loop() { n += 1; }
        assert_eq!(n, 3);
        // N is preserved for reporting; the internal counter tracks loop state.
        assert_eq!(b.N, 3);
    }

    #[test]
    fn b_report_format() {
        let mut b = B::new(100);
        while b.Loop() { std::hint::black_box(1 + 1); }
        let line = b.report("BenchmarkX");
        assert!(line.contains("BenchmarkX"));
        assert!(line.contains("ns/op"));
    }
}
