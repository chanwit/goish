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
// Backed by crossbeam-channel for true Go-style MPMC semantics — multiple
// senders AND multiple receivers can share the same channel via Clone.
//
// `chan!(T)` produces an unbounded channel (Go's `make(chan T)` is unbuffered;
// crossbeam's unbounded is the closest cheap analog without a runtime).

use crossbeam_channel::{Receiver, Sender};

#[derive(Clone)]
pub struct Chan<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> Chan<T> {
    /// Construct a buffered channel with capacity `cap`. Use 0 for "unbuffered"
    /// (rendezvous) semantics like Go's `make(chan T)`.
    pub fn new(cap: usize) -> Self {
        let (sender, receiver) = if cap == 0 {
            crossbeam_channel::bounded(0)
        } else {
            crossbeam_channel::bounded(cap)
        };
        Chan { sender, receiver }
    }

    /// `ch <- v` — blocks until a receiver is ready (rendezvous) or there's
    /// room in the buffer. Returns nil on success, error if the channel is
    /// closed (Go panics; we return error to keep things composable).
    pub fn Send(&self, v: T) -> crate::errors::error {
        match self.sender.send(v) {
            Ok(()) => crate::errors::nil,
            Err(_) => crate::errors::New("send on closed channel"),
        }
    }

    /// `v, ok := <-ch` — blocks until a value arrives. `ok` is false when
    /// the channel is closed and drained.
    pub fn Recv(&self) -> (T, bool)
    where
        T: Default,
    {
        match self.receiver.recv() {
            Ok(v) => (v, true),
            Err(_) => (T::default(), false),
        }
    }

    /// Non-blocking try-receive. Returns (value, true) or (default, false).
    pub fn TryRecv(&self) -> (T, bool)
    where
        T: Default,
    {
        match self.receiver.try_recv() {
            Ok(v) => (v, true),
            Err(_) => (T::default(), false),
        }
    }

    // NOTE: Go's `close(ch)` has no faithful single-method analog here.
    // Crossbeam closes a channel only when *all* senders drop. Since `Chan`
    // bundles sender+receiver and is Clone, any live clone keeps the channel
    // open. For now: let all `Chan` handles go out of scope to close.
    // A real `Close()` will arrive when we add a `done`-channel pattern or
    // an `Arc<Mutex<Option<Sender>>>` interior.

    pub fn Len(&self) -> crate::types::int {
        self.sender.len() as crate::types::int
    }

    pub fn Cap(&self) -> crate::types::int {
        self.sender.capacity().unwrap_or(0) as crate::types::int
    }

    /// Lowercase alias so the polymorphic `len!()` macro can dispatch
    /// uniformly across String/Vec/HashMap/Chan via method-call auto-ref.
    pub fn len(&self) -> usize {
        self.sender.len()
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
            for i in 0..5 {
                producer.Send(i);
            }
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

}
