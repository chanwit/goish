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

    /// `select!`-internal: "is a recv case ready right now?"
    ///
    /// Go distinguishes three states visible to `select`:
    ///   - value present         → case fires with (v, true)
    ///   - channel closed+drained → case fires with (zero, false)
    ///   - empty + still open    → case BLOCKS; default fires instead
    ///
    /// Plain `TryRecv` collapses the last two (both give `(zero, false)`).
    /// This method returns `Some((v, ok))` when the case should fire, and
    /// `None` when the case is blocked.
    #[doc(hidden)]
    pub fn __select_try_recv(&self) -> Option<(T, bool)>
    where T: Default {
        if let Some(v) = self.inner.try_recv() {
            return Some((v, true));
        }
        if self.inner.is_closed() {
            return Some((T::default(), false));
        }
        None
    }

    /// `select!`-internal: "is a send case ready right now?" Returns Err(v)
    /// if the buffer is full (or rendezvous has no partner). Matches the
    /// semantic Go's `select { case c <- v: }` uses.
    #[doc(hidden)]
    pub fn __select_try_send(&self, v: T) -> Result<(), T> {
        self.inner.try_send(v)
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

/// `select!{ ... }` — Go's select statement.
///
/// Supported arm forms:
///   - `recv(c)              => { body }`   — case fires, value discarded
///   - `recv(c) |v|          => { body }`   — bind received value
///   - `recv(c) |v, ok|      => { body }`   — bind value + close-flag
///   - `send(c, expr)        => { body }`   — send expr; fires on success
///   - `default              => { body }`   — fallback if no case ready
///
/// Semantics:
///   - All case channels are tested non-blocking.
///   - If multiple are ready, the first in source order wins (Go's spec
///     says uniform-random; this is a known simplification to revisit).
///   - If none ready and `default` is present, default fires.
///   - If none ready and no `default`, this select *currently* spins
///     with a short sleep between polls. (Proper cross-channel parking
///     via `flume::Selector` lands in the next iteration.)
#[macro_export]
macro_rules! select {
    ($($tt:tt)*) => {
        $crate::__select_parse!(@arms [] $($tt)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __select_parse {
    // recv(c) |v, ok| => { body }
    (@arms [$($acc:tt)*] recv($ch:expr) |$v:ident, $ok:ident| => $body:block $(, $($rest:tt)*)?) => {
        $crate::__select_parse!(@arms [$($acc)* (RecvBind2 ($ch) ($v) ($ok) ($body))] $($($rest)*)?)
    };
    // recv(c) |v| => { body }
    (@arms [$($acc:tt)*] recv($ch:expr) |$v:ident| => $body:block $(, $($rest:tt)*)?) => {
        $crate::__select_parse!(@arms [$($acc)* (RecvBind1 ($ch) ($v) ($body))] $($($rest)*)?)
    };
    // recv(c) => { body }
    (@arms [$($acc:tt)*] recv($ch:expr) => $body:block $(, $($rest:tt)*)?) => {
        $crate::__select_parse!(@arms [$($acc)* (RecvDrop ($ch) ($body))] $($($rest)*)?)
    };
    // send(c, v) => { body }
    (@arms [$($acc:tt)*] send($ch:expr, $v:expr) => $body:block $(, $($rest:tt)*)?) => {
        $crate::__select_parse!(@arms [$($acc)* (Send ($ch) ($v) ($body))] $($($rest)*)?)
    };
    // default => { body }
    (@arms [$($acc:tt)*] default => $body:block $(,)?) => {
        $crate::__select_parse!(@emit [$($acc)*] [$body])
    };
    // End of input, no default
    (@arms [$($acc:tt)*]) => {
        $crate::__select_parse!(@emit [$($acc)*] [])
    };

    // Emit: try each arm; if none fire and no default, spin-wait briefly.
    (@emit [$($arms:tt)*] [$($def:tt)*]) => {{
        #[allow(unused_mut, unused_assignments)]
        let mut __goish_fired = false;
        loop {
            $crate::__select_parse!(@try __goish_fired $($arms)*);
            if __goish_fired {
                break;
            }
            $crate::__select_parse!(@default_or_spin __goish_fired [$($def)*]);
            if __goish_fired {
                break;
            }
        }
    }};

    // If default exists: fire it.
    (@default_or_spin $fired:ident [$($def:tt)+]) => {{
        { $($def)+ }
        $fired = true;
    }};
    // No default: short sleep before retry.
    (@default_or_spin $fired:ident []) => {{
        std::thread::sleep(std::time::Duration::from_millis(1));
    }};

    // Try each arm:
    (@try $fired:ident (RecvBind2 ($ch:expr) ($v:ident) ($ok:ident) ($body:block)) $($rest:tt)*) => {
        if !$fired {
            if let Some(($v, $ok)) = ($ch).__select_try_recv() {
                $body
                $fired = true;
            }
        }
        $crate::__select_parse!(@try $fired $($rest)*);
    };
    (@try $fired:ident (RecvBind1 ($ch:expr) ($v:ident) ($body:block)) $($rest:tt)*) => {
        if !$fired {
            if let Some(($v, _)) = ($ch).__select_try_recv() {
                $body
                $fired = true;
            }
        }
        $crate::__select_parse!(@try $fired $($rest)*);
    };
    (@try $fired:ident (RecvDrop ($ch:expr) ($body:block)) $($rest:tt)*) => {
        if !$fired {
            if ($ch).__select_try_recv().is_some() {
                $body
                $fired = true;
            }
        }
        $crate::__select_parse!(@try $fired $($rest)*);
    };
    (@try $fired:ident (Send ($ch:expr) ($v:expr) ($body:block)) $($rest:tt)*) => {
        if !$fired {
            if ($ch).__select_try_send($v).is_ok() {
                $body
                $fired = true;
            }
        }
        $crate::__select_parse!(@try $fired $($rest)*);
    };
    (@try $fired:ident) => {};
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
