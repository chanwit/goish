// goroutine: `go!{...}` → spawn a goroutine on the tokio blocking pool.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   go worker(jobs)                     go!{ worker(jobs); };
//   go func() { ... }()                 go!{ ... };
//
// Implementation: `tokio::task::spawn_blocking`. Tokio maintains a pool of
// OS threads (default 512) that get reused across goroutines, so spawn cost
// is amortized vs plain `std::thread::spawn`. The body stays synchronous —
// flume's `Send/Recv` block cleanly on the blocking pool.
//
// This isn't true M:N green-thread scheduling (that would require async
// bodies + a proc-macro to rewrite channel ops into `.await`). It IS
// materially lighter than `std::thread::spawn` per-goroutine.

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
        if let Ok(n) = std::env::var("GOISH_BLOCKING_POOL_SIZE") {
            if let Ok(n) = n.parse::<usize>() {
                b.max_blocking_threads(n);
            }
        }
        b.build().expect("goish: failed to build tokio runtime")
    })
}

/// Handle to a live goroutine. `Wait()` blocks until it finishes.
pub struct Goroutine {
    handle: Option<JoinHandle<()>>,
}

impl Goroutine {
    /// Spawn a new goroutine running `f`. Normally users invoke this via
    /// `go!{ ... }` rather than calling directly.
    pub fn spawn<F>(f: F) -> Goroutine
    where
        F: FnOnce() + Send + 'static,
    {
        crate::runtime::LIVE_GOROUTINES.fetch_add(1, Ordering::SeqCst);
        let handle = runtime().spawn_blocking(move || {
            struct Guard;
            impl Drop for Guard {
                fn drop(&mut self) {
                    crate::runtime::LIVE_GOROUTINES.fetch_sub(1, Ordering::SeqCst);
                }
            }
            let _g = Guard;
            f();
        });
        Goroutine { handle: Some(handle) }
    }

    /// Wait for the goroutine to finish. Returns nil on clean exit, error on
    /// panic.
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
}

/// `go!{ stmts }` — spawn a goroutine running the block.
///
/// Returns a `Goroutine` handle — ignore it if you don't need to wait.
#[macro_export]
macro_rules! go {
    ($($tt:tt)*) => {
        $crate::goroutine::Goroutine::spawn(move || {
            $($tt)*
        })
    };
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    #[test]
    fn go_runs_in_another_thread_and_wait_joins() {
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
    fn go_with_channel() {
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

    #[test]
    fn many_goroutines_via_pool() {
        // Spawn 1000 goroutines — tokio's blocking pool reuses threads.
        // Each sends to a shared channel; main recvs the sum.
        let ch = crate::chan!(i64, 1000);
        let mut handles = Vec::new();
        for i in 0..1000i64 {
            let c = ch.clone();
            handles.push(crate::go!{ c.Send(i); });
        }
        let mut sum = 0i64;
        for _ in 0..1000 {
            let (v, _) = ch.Recv();
            sum += v;
        }
        for h in handles { let _ = h.Wait(); }
        assert_eq!(sum, (999 * 1000) / 2);
    }
}
