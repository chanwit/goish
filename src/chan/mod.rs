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
    inner: Option<engine::Inner<T>>,
}

/// Go's zero-value `chan T` is nil ��� send/recv block forever, close panics.
/// In goish, a nil channel panics on all operations (avoids silent deadlock).
impl<T> Default for Chan<T> {
    fn default() -> Self { Chan { inner: None } }
}

impl<T> crate::errors::IsNil for Chan<T> {
    fn is_nil(&self) -> bool { self.inner.is_none() }
}

/// `ch != nil` / `ch == nil` — Go's polymorphic nil comparison for channels.
/// A nil channel equals nil; a live channel does not.
impl<T> PartialEq<crate::errors::error> for Chan<T> {
    fn eq(&self, other: &crate::errors::error) -> bool {
        self.is_nil() && other.is_nil()
    }
}
impl<T> PartialEq<Chan<T>> for crate::errors::error {
    fn eq(&self, other: &Chan<T>) -> bool {
        other.is_nil() && self.is_nil()
    }
}

impl<T> Chan<T> {
    /// Construct a buffered channel with capacity `cap`. Use 0 for rendezvous.
    pub fn new(cap: usize) -> Self {
        Chan { inner: Some(engine::Inner::new(cap)) }
    }

    /// Returns true if the channel is nil (zero-value, never constructed).
    pub fn is_nil(&self) -> bool { self.inner.is_none() }

    /// Access the live inner, panicking on nil channel.
    fn live(&self) -> &engine::Inner<T> {
        self.inner.as_ref().expect("use of nil channel")
    }

    /// `ch <- v` — blocks until a receiver is ready or there's room.
    ///
    /// Accepts any `impl Into<T>` so `c.Send("hello")` works when
    /// `T = string`, matching Go's implicit string-literal coercion.
    ///
    /// Panics on a closed or nil channel.
    pub fn Send(&self, v: impl Into<T>) {
        if self.live().send(v.into()).is_err() {
            panic!("send on closed channel");
        }
    }

    /// Async send — used by the `go!{}` macro inside async contexts.
    ///
    /// Panics on a closed or nil channel.
    pub async fn send(&self, v: impl Into<T>) {
        if self.live().send_async(v.into()).await.is_err() {
            panic!("send on closed channel");
        }
    }

    /// `v, ok := <-ch` — blocks until a value arrives. `ok == false` when the
    /// channel is closed and drained.
    pub fn Recv(&self) -> (T, bool)
    where T: Default {
        match self.live().recv() {
            Some(v) => (v, true),
            None => (T::default(), false),
        }
    }

    /// Async recv — used inside `go!{}` macro expansion.
    pub async fn recv(&self) -> (T, bool)
    where T: Default {
        match self.live().recv_async().await {
            Some(v) => (v, true),
            None => (T::default(), false),
        }
    }

    /// Non-blocking try-receive. Returns (value, true) on success.
    pub fn TryRecv(&self) -> (T, bool)
    where T: Default {
        match self.live().try_recv() {
            Some(v) => (v, true),
            None => (T::default(), false),
        }
    }

    /// Non-blocking try-send. Returns true on success, false on full buffer.
    /// Panics on closed or nil channel.
    pub fn TrySend(&self, v: impl Into<T>) -> bool {
        let inner = self.live();
        if inner.is_closed() {
            panic!("send on closed channel");
        }
        inner.try_send(v.into()).is_ok()
    }

    #[doc(hidden)]
    pub fn __select_try_recv(&self) -> Option<(T, bool)>
    where T: Default {
        let inner = self.live();
        if let Some(v) = inner.try_recv() {
            return Some((v, true));
        }
        if inner.is_closed() {
            return Some((T::default(), false));
        }
        None
    }

    #[doc(hidden)]
    pub fn __select_try_send(&self, v: T) -> Result<(), T> {
        let inner = self.live();
        if inner.is_closed() {
            panic!("send on closed channel");
        }
        inner.try_send(v)
    }

    /// close(ch) — panics on nil or already-closed channel.
    #[allow(non_snake_case)]
    pub fn Close(&self) {
        if !self.live().close() {
            panic!("close of closed channel");
        }
    }

    pub fn Len(&self) -> crate::types::int {
        if self.is_nil() { return 0; }
        self.live().len() as crate::types::int
    }

    pub fn Cap(&self) -> crate::types::int {
        if self.is_nil() { return 0; }
        self.live().cap() as crate::types::int
    }

    pub fn len(&self) -> usize {
        if self.is_nil() { return 0; }
        self.live().len()
    }

    #[doc(hidden)]
    pub fn __flume_rx(&self) -> &flume::Receiver<T> { self.live().__flume_rx() }

    #[doc(hidden)]
    pub fn __flume_tx(&self) -> &flume::Sender<T> { self.live().__flume_tx() }

    #[doc(hidden)]
    pub fn __flume_close_rx(&self) -> &flume::Receiver<()> { self.live().__flume_close_rx() }

    #[doc(hidden)]
    pub fn __is_closed(&self) -> bool {
        match &self.inner {
            Some(inner) => inner.is_closed(),
            None => false, // nil channel is not "closed" — it was never opened
        }
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

// `select!{ ... }` is re-exported at the crate root from goish_macros.
// See src/lib.rs. Left here as a marker for the previous macro_rules!
// definition removed in v0.10.1 (issue #119).

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
    #[should_panic(expected = "send on closed channel")]
    fn send_on_closed_panics() {
        let ch = crate::chan!(i64, 1);
        ch.Close();
        ch.Send(42);
    }

    #[test]
    #[should_panic(expected = "close of closed channel")]
    fn double_close_panics() {
        let ch = crate::chan!(i64, 1);
        ch.Close();
        ch.Close();
    }

    #[test]
    #[should_panic(expected = "send on closed channel")]
    fn try_send_on_closed_panics() {
        let ch = crate::chan!(i64, 1);
        ch.Close();
        ch.TrySend(1);
    }

    #[test]
    #[should_panic(expected = "send on closed channel")]
    fn select_send_on_closed_panics() {
        let ch = crate::chan!(i64, 1);
        ch.Close();
        crate::select!{
            send(ch, 1) => {},
            default => {},
        }
    }

    #[test]
    fn engine_name_is_flume() {
        assert_eq!(crate::chan::ENGINE, "flume");
    }

    #[test]
    fn select_default_fires_when_empty() {
        let c = crate::chan!(i64, 1);
        let mut took_default = false;
        crate::select!{
            recv(c) => {},
            default => { took_default = true; },
        }
        assert!(took_default);
    }

    #[test]
    fn select_recv_fires_when_ready() {
        let c = crate::chan!(i64, 1);
        c.Send(42);
        let mut got = -1i64;
        crate::select!{
            recv(c) |v| => { got = v; },
            default => {},
        }
        assert_eq!(got, 42);
    }

    #[test]
    fn select_recv_fires_on_closed_drained() {
        // Go's semantic: after close+drain, recv case is still "ready",
        // firing with (zero, false) — NOT default.
        let c = crate::chan!(i64, 1);
        c.Close();
        let mut fired = false;
        let mut ok_seen = true;
        crate::select!{
            recv(c) |_v, ok| => { fired = true; ok_seen = ok; },
            default => {},
        }
        assert!(fired, "recv case should fire on closed channel");
        assert!(!ok_seen, "ok should be false on closed channel");
    }

    #[test]
    fn select_send_fires_when_space() {
        let c = crate::chan!(i64, 1);
        let mut sent = false;
        crate::select!{
            send(c, 99) => { sent = true; },
            default => {},
        }
        assert!(sent);
        let (v, _) = c.Recv();
        assert_eq!(v, 99);
    }

    #[test]
    fn select_send_default_when_full() {
        let c = crate::chan!(i64, 1);
        c.Send(1); // fill
        let mut took_default = false;
        crate::select!{
            send(c, 99) => {},
            default => { took_default = true; },
        }
        assert!(took_default);
    }
}
