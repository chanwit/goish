// goroutine: `go!{...}` → spawn a goroutine.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   go worker(jobs)                     go!{ worker(jobs); };
//   go func() { ... }()                 go!{ ... };
//
// Implementation note: v0.1 spawns an OS thread (std::thread::spawn), not a
// green thread. Tens of thousands of goroutines won't scale here like they
// do in Go — that's deferred until we have a real scheduler. The `go!{}`
// macro returns a `JoinHandle` so you can .Wait() on it if desired.

use std::thread::JoinHandle;

/// Wrapper around `std::thread::JoinHandle` with Go-style `Wait()`.
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
        Goroutine {
            handle: Some(std::thread::spawn(f)),
        }
    }

    /// Wait for the goroutine to finish. Returns nil error if it completed
    /// normally, or an error if it panicked.
    pub fn Wait(mut self) -> crate::errors::error {
        match self.handle.take() {
            Some(h) => match h.join() {
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
}
