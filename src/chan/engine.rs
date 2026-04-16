//! chan/engine: flume-backed channel engine.
//!
//! Flume is MPMC, bounded/unbounded, has both sync and async receivers.
//! Gap vs Go: flume has no sender-side `close()` that wakes parked
//! receivers. We emulate it with a *shadow* close channel whose sender is
//! dropped when `Close()` runs — that flips every clone of the shadow
//! receiver into `Disconnected`, which `flume::Selector` and
//! `tokio::select!` both pick up instantly. Net effect: sync `Recv()`,
//! async `recv_async()`, and `select!` arms all park *truly* (no polling,
//! no spin) and wake within one scheduler cycle of `Close()`.
//!
//! Three shared signals (behind `Arc`) model close:
//!   - `closed: AtomicBool` — fast sender-side check (Go's closed flag).
//!   - `close_signal: tokio::sync::Semaphore` — async waiter wakeup
//!     (`.close()` makes `.acquire()` return `Err` on every pending waiter).
//!   - `close_rx: flume::Receiver<()>` + `close_tx_guard` — sync waiter
//!     wakeup. The guard holds the only `Sender<()>`; dropping it on
//!     `Close()` disconnects the receiver, which `flume::Selector` uses
//!     to bail out of a parked recv.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

pub struct Inner<T> {
    tx: flume::Sender<T>,
    rx: flume::Receiver<T>,
    closed: Arc<AtomicBool>,
    /// Never-acquired-as-permit semaphore; `.close()` on it makes every
    /// pending `acquire()` return `Err(AcquireError)`. Async receivers
    /// select-await on this so `Chan::Close()` wakes them.
    close_signal: Arc<Semaphore>,
    /// Sync-side close signal. The shadow `Sender<()>` lives inside the
    /// mutex-guarded `Option`; `Close()` takes it and drops it. All clones
    /// of `close_rx` then transition to `Disconnected`, waking any parked
    /// `flume::Selector` call in `recv()` or in the `select!` macro.
    close_rx: flume::Receiver<()>,
    close_tx_guard: Arc<Mutex<Option<flume::Sender<()>>>>,
}

impl<T> Inner<T> {
    pub fn new(cap: usize) -> Self {
        let (tx, rx) = if cap == 0 {
            flume::bounded(0)
        } else {
            flume::bounded(cap)
        };
        // Shadow channel: never carries a value. Its sole job is to host a
        // sender whose `Drop` wakes parked sync receivers.
        let (close_tx, close_rx) = flume::bounded::<()>(1);
        Inner {
            tx,
            rx,
            closed: Arc::new(AtomicBool::new(false)),
            close_signal: Arc::new(Semaphore::new(0)),
            close_rx,
            close_tx_guard: Arc::new(Mutex::new(Some(close_tx))),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Mark the channel closed. Returns `true` if this call performed the
    /// transition (open → closed); `false` if the channel was already
    /// closed — callers panic on the double-close case to match Go.
    pub fn close(&self) -> bool {
        let was_open = !self.closed.swap(true, Ordering::SeqCst);
        if was_open {
            // Wake async waiters.
            self.close_signal.close();
            // Wake sync waiters: drop the shadow sender so every clone of
            // `close_rx` disconnects.
            let _ = self.close_tx_guard.lock().unwrap().take();
        }
        was_open
    }

    pub fn send(&self, v: T) -> Result<(), T> {
        if self.is_closed() {
            return Err(v);
        }
        match self.tx.send(v) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into_inner()),
        }
    }

    pub async fn send_async(&self, v: T) -> Result<(), T> {
        if self.is_closed() {
            return Err(v);
        }
        match self.tx.send_async(v).await {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into_inner()),
        }
    }

    pub fn try_send(&self, v: T) -> Result<(), T> {
        if self.is_closed() {
            return Err(v);
        }
        match self.tx.try_send(v) {
            Ok(()) => Ok(()),
            Err(flume::TrySendError::Full(v)) => Err(v),
            Err(flume::TrySendError::Disconnected(v)) => Err(v),
        }
    }

    /// Blocking receive. Parks on a `flume::Selector` that races the main
    /// receiver against the shadow close receiver, so `Close()` wakes the
    /// caller within one scheduler cycle (no polling).
    pub fn recv(&self) -> Option<T> {
        loop {
            // Fast path: pending value or already-disconnected main chan.
            match self.rx.try_recv() {
                Ok(v) => return Some(v),
                Err(flume::TryRecvError::Disconnected) => return None,
                Err(flume::TryRecvError::Empty) => {
                    if self.is_closed() { return None; }
                }
            }
            // Park: fire on EITHER a value arriving OR the shadow close
            // channel disconnecting.
            let mut got: Option<T> = None;
            flume::Selector::new()
                .recv(&self.rx, |res| {
                    if let Ok(v) = res { got = Some(v); }
                })
                .recv(&self.close_rx, |_| {
                    // Shadow disconnected → loop re-checks is_closed and
                    // drains any still-buffered values.
                })
                .wait();
            if got.is_some() { return got; }
            // else: loop back to drain/close-check
        }
    }

    pub async fn recv_async(&self) -> Option<T> {
        loop {
            match self.rx.try_recv() {
                Ok(v) => return Some(v),
                Err(flume::TryRecvError::Disconnected) => return None,
                Err(flume::TryRecvError::Empty) => {
                    if self.is_closed() { return None; }
                    // Park until either a value arrives OR Close() fires.
                    tokio::select! {
                        biased;
                        res = self.rx.recv_async() => match res {
                            Ok(v) => return Some(v),
                            Err(_) => return None,
                        },
                        _ = self.close_signal.acquire() => {
                            // close_signal is closed → we've been told to
                            // shut down. Retry the loop: if there's still a
                            // value buffered, we should drain it; otherwise
                            // the is_closed check returns None.
                            continue;
                        }
                    }
                }
            }
        }
    }

    pub fn try_recv(&self) -> Option<T> {
        self.rx.try_recv().ok()
    }

    pub fn len(&self) -> usize { self.tx.len() }
    pub fn cap(&self) -> usize { self.tx.capacity().unwrap_or(0) }

    /// `select!`-internal: flume receiver for the main channel. Used by the
    /// macro to build a `flume::Selector` that parks on real channel state
    /// (no polling). Do NOT call from user code.
    #[doc(hidden)]
    pub fn __flume_rx(&self) -> &flume::Receiver<T> { &self.rx }

    /// `select!`-internal: shadow receiver that disconnects on `Close()`.
    /// Macro uses this so a recv/send arm fires immediately on a closed
    /// channel (recv → (zero, false); send → panic, matching Go).
    #[doc(hidden)]
    pub fn __flume_close_rx(&self) -> &flume::Receiver<()> { &self.close_rx }

    /// `select!`-internal: flume sender for the main channel. Macro builds
    /// `.send(&tx, v, handler)` arms around this for `send(c, v)` cases.
    #[doc(hidden)]
    pub fn __flume_tx(&self) -> &flume::Sender<T> { &self.tx }
}

impl<T> Clone for Inner<T> {
    fn clone(&self) -> Self {
        Inner {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
            closed: self.closed.clone(),
            close_signal: self.close_signal.clone(),
            close_rx: self.close_rx.clone(),
            close_tx_guard: self.close_tx_guard.clone(),
        }
    }
}

pub const ENGINE_NAME: &str = "flume";
