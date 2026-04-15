// chan: Go's channels, ported.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   ch := make(chan int)                let ch = chan!(int);
//   ch := make(chan int, 10)            let ch = chan!(int, 10);
//   ch <- 42                            ch.Send(42);
//   v, ok := <-ch                       let (v, ok) = ch.Recv();
//   close(ch)                           ch.Close();
//
// MPMC: multiple senders AND receivers can share the same channel via Clone.
//
// Backed by the `flume` crate (v0.5 bake-off winner vs async-channel —
// ~2× faster on unbuffered rendezvous, 3.5× less RSS for 100k channels).

mod engine;

/// Name of the channel engine (kept for telemetry / benchmarks).
pub const ENGINE: &str = engine::ENGINE_NAME;

#[derive(Clone)]
pub struct Chan<T> {
    inner: engine::Inner<T>,
}

impl<T> Chan<T> {
    /// Construct a buffered channel with capacity `cap`. Use 0 for rendezvous.
    pub fn new(cap: usize) -> Self {
        Chan { inner: engine::Inner::new(cap) }
    }

    /// `ch <- v` — blocks until a receiver is ready or there's room.
    /// Returns nil on success, error if the channel is closed.
    pub fn Send(&self, v: T) -> crate::errors::error {
        match self.inner.send(v) {
            Ok(()) => crate::errors::nil,
            Err(_) => crate::errors::New("send on closed channel"),
        }
    }

    /// Async send — used by the `go!{}` macro inside async contexts.
    pub async fn send(&self, v: T) -> crate::errors::error {
        match self.inner.send_async(v).await {
            Ok(()) => crate::errors::nil,
            Err(_) => crate::errors::New("send on closed channel"),
        }
    }

    /// `v, ok := <-ch` — blocks until a value arrives. `ok == false` when the
    /// channel is closed and drained.
    pub fn Recv(&self) -> (T, bool)
    where T: Default {
        match self.inner.recv() {
            Some(v) => (v, true),
            None => (T::default(), false),
        }
    }

    /// Async recv — used inside `go!{}` macro expansion.
    pub async fn recv(&self) -> (T, bool)
    where T: Default {
        match self.inner.recv_async().await {
            Some(v) => (v, true),
            None => (T::default(), false),
        }
    }

    /// Non-blocking try-receive. Returns (value, true) on success.
    pub fn TryRecv(&self) -> (T, bool)
    where T: Default {
        match self.inner.try_recv() {
            Some(v) => (v, true),
            None => (T::default(), false),
        }
    }

    /// Non-blocking try-send. Returns true on success.
    pub fn TrySend(&self, v: T) -> bool {
        self.inner.try_send(v).is_ok()
    }

    /// close(ch) — mark the channel closed. Remaining buffered items still
    /// recv; once drained, receivers return (zero, false).
    #[allow(non_snake_case)]
    pub fn Close(&self) {
        self.inner.close();
    }

    pub fn Len(&self) -> crate::types::int {
        self.inner.len() as crate::types::int
    }

    pub fn Cap(&self) -> crate::types::int {
        self.inner.cap() as crate::types::int
    }

    /// Lowercase alias for the polymorphic `len!()` macro.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

/// `chan!(T)`         → unbuffered channel (rendezvous)
/// `chan!(T, n)`      → buffered channel with capacity n
#[macro_export]
macro_rules! chan {
    ($t:ty, $cap:expr) => {
        $crate::chan::Chan::<$t>::new($cap)
    };
    ($t:ty) => {
        $crate::chan::Chan::<$t>::new(0)
    };
}

/// `close!(&ch)` — Go's `close(ch)`.
#[macro_export]
macro_rules! close {
    ($ch:expr) => {
        ($ch).Close()
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn buffered_send_then_recv() {
        let ch = crate::chan!(i64, 4);
        ch.Send(10);
        ch.Send(20);
        let (v, ok) = ch.Recv();
        assert!(ok);
        assert_eq!(v, 10);
        let (v, ok) = ch.Recv();
        assert!(ok);
        assert_eq!(v, 20);
    }

    #[test]
    fn try_recv_on_empty() {
        let ch = crate::chan!(i64, 1);
        let (_, ok) = ch.TryRecv();
        assert!(!ok);
        ch.Send(99);
        let (v, ok) = ch.TryRecv();
        assert!(ok);
        assert_eq!(v, 99);
    }

    #[test]
    fn cross_thread_buffered() {
        let ch = crate::chan!(i64, 8);
        let producer = ch.clone();
        let handle = std::thread::spawn(move || {
            for i in 0..5 { producer.Send(i); }
        });
        let mut sum = 0i64;
        for _ in 0..5 {
            let (v, _) = ch.Recv();
            sum += v;
        }
        handle.join().unwrap();
        assert_eq!(sum, 0 + 1 + 2 + 3 + 4);
    }

    #[test]
    fn unbuffered_rendezvous() {
        let ch = crate::chan!(i64);
        let producer = ch.clone();
        let handle = std::thread::spawn(move || {
            producer.Send(42);
        });
        let (v, ok) = ch.Recv();
        handle.join().unwrap();
        assert!(ok);
        assert_eq!(v, 42);
    }

    #[test]
    fn close_drains_then_returns_zero_ok_false() {
        let ch = crate::chan!(i64, 4);
        ch.Send(1);
        ch.Send(2);
        ch.Close();
        let (v, ok) = ch.Recv();
        assert!(ok); assert_eq!(v, 1);
        let (v, ok) = ch.Recv();
        assert!(ok); assert_eq!(v, 2);
        let (v, ok) = ch.Recv();
        assert!(!ok); assert_eq!(v, 0);
    }

    #[test]
    fn send_on_closed_returns_error() {
        let ch = crate::chan!(i64, 1);
        ch.Close();
        let err = ch.Send(42);
        assert!(err != crate::errors::nil);
    }

    #[test]
    fn engine_name_is_flume() {
        assert_eq!(crate::chan::ENGINE, "flume");
    }
}
