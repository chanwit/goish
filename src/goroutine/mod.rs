// goroutine: `go!{...}` → spawn a lightweight async task on tokio.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   go worker(jobs)                     go!{ worker(jobs).await; };
//   go func() { ... }()                 go!{ ... };
//
// Each goroutine is a tokio async task — ~100 bytes of state, scales to
// millions per process. Scheduled M:N across tokio's worker threads
// (count = GOMAXPROCS or defaults to CPU count).
//
// ## The `.await` leak
//
// Rust can't invisibly rewrite sync method calls into async, so inside a
// `go!{}` body:
//
//   outside go!{}:  c.Send(v)              (blocking, sync)
//   inside  go!{}:  c.send(v).await        (cooperative, async)
//
// Same goes for `c.recv().await`, `time::Sleep(d)` → `tokio::time::sleep(d).await`,
// `g.wait().await` to join child goroutines. A follow-up proc-macro (goish v0.5.1)
// will automate the rewrite so user code stays Go-shaped; for now, users write
// the `.await` by hand.

use std::future::Future;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

/// Global runtime shared across all `go!{}` calls in a process.
pub(crate) fn runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        let mut b = tokio::runtime::Builder::new_multi_thread();
        b.enable_all();
        if let Ok(n) = std::env::var("GOMAXPROCS") {
            if let Ok(n) = n.parse::<usize>() {
                b.worker_threads(n);
            }
        }
        b.build().expect("goish: failed to build tokio runtime")
    })
}

/// Handle to a live goroutine. `Wait()` blocks the caller until it finishes;
/// `.wait().await` joins asynchronously when called from inside another
/// `go!{}`.
pub struct Goroutine {
    handle: Option<JoinHandle<()>>,
}

impl Goroutine {
    /// Spawn a new goroutine running the given future.
    pub fn spawn<F>(f: F) -> Goroutine
    where
        F: Future<Output = ()> + Send + 'static,
    {
        crate::runtime::LIVE_GOROUTINES.fetch_add(1, Ordering::SeqCst);
        let handle = runtime().spawn(async move {
            struct Guard;
            impl Drop for Guard {
                fn drop(&mut self) {
                    crate::runtime::LIVE_GOROUTINES.fetch_sub(1, Ordering::SeqCst);
                }
            }
            let _g = Guard;
            f.await;
        });
        Goroutine { handle: Some(handle) }
    }

    /// `g.Wait()` — block the current (non-async) thread until the goroutine
    /// finishes. Returns nil on clean exit, error on panic.
    #[allow(non_snake_case)]
    pub fn Wait(mut self) -> crate::errors::error {
        match self.handle.take() {
            Some(h) => match runtime().block_on(h) {
                Ok(()) => crate::errors::nil,
                Err(_) => crate::errors::New("goroutine panicked"),
            },
            None => crate::errors::nil,
        }
    }

    /// `g.wait().await` — async join for use inside another `go!{}`. This
    /// form cooperates with the scheduler; prefer it when waiting from
    /// another goroutine.
    pub async fn wait(mut self) -> crate::errors::error {
        match self.handle.take() {
            Some(h) => match h.await {
                Ok(()) => crate::errors::nil,
                Err(_) => crate::errors::New("goroutine panicked"),
            },
            None => crate::errors::nil,
        }
    }
}

/// `go!{ stmts }` — spawn a goroutine running the block.
///
/// The block can use goish's usual sync-looking API (`c.Send(v)`,
/// `c.Recv()`, `g.Wait()`). A proc-macro rewriter (`goish_macros::
/// rewrite_go_body!`) walks the body AST and rewrites those calls into
/// their async form (`c.send(v).await`, etc.) before `tokio::spawn`ing.
/// The `.await` is invisible at the call site.
///
/// Collateral damage: any `.Send(x)`, `.Recv()`, or `.Wait()` method call
/// on a non-goish type inside `go!{}` is ALSO rewritten into async form,
/// which may not compile. Rename the conflicting method or call it
/// outside `go!{}`.
#[macro_export]
macro_rules! go {
    ($($tt:tt)*) => {
        $crate::goroutine::Goroutine::spawn(async move {
            $crate::__macros::rewrite_go_body!($($tt)*);
        })
    };
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    #[test]
    fn go_runs_and_wait_joins() {
        let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
        let log_clone = log.clone();
        let g = crate::go!{
            log_clone.lock().unwrap().push(42);
        };
        let err = g.Wait();
        assert!(err == crate::errors::nil);
        assert_eq!(*log.lock().unwrap(), vec![42]);
    }

    #[test]
    fn go_with_channel_looks_sync() {
        // Inside go!{} we write .Send (uppercase, sync form). The proc-macro
        // rewrites it to .send(…).await so the body is actually async.
        let ch = crate::chan!(i64, 4);
        let producer = ch.clone();
        let g = crate::go!{
            for i in 1i64..=3 {
                producer.Send(i);
            }
        };
        let _ = g.Wait();
        let mut got: Vec<i64> = Vec::new();
        for _ in 0..3 {
            let (v, _) = ch.Recv();
            got.push(v);
        }
        got.sort();
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn panicking_goroutine_returns_error() {
        let g = crate::go!{
            panic!("boom");
        };
        let err = g.Wait();
        assert!(err != crate::errors::nil);
    }

    /// 10k goroutines — enough to prove we're not on a 512-slot pool
    /// without blasting CI with 1M. The million-goroutine proof lives in
    /// tests/million_goroutines.rs (run with `cargo test --release`).
    #[test]
    fn ten_thousand_goroutines() {
        let ch = crate::chan!(i64, 10_000);
        let mut handles = Vec::with_capacity(10_000);
        for i in 0..10_000i64 {
            let c = ch.clone();
            handles.push(crate::go!{ c.Send(i); });
        }
        let mut sum = 0i64;
        for _ in 0..10_000 {
            let (v, _) = ch.Recv();
            sum += v;
        }
        for h in handles { let _ = h.Wait(); }
        assert_eq!(sum, (9999 * 10_000) / 2);
    }
}
