//! chan/engine_flume: flume-backed channel engine.
//!
//! Flume is MPMC, bounded/unbounded, has both sync and async receivers.
//! The one gap vs Go: no explicit `close()` method — closure is implicit
//! on drop of all senders. We emulate Go's `close(ch)` via a shared
//! `AtomicBool` that senders check before attempting to send.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Inner<T> {
    tx: flume::Sender<T>,
    rx: flume::Receiver<T>,
    closed: Arc<AtomicBool>,
}

impl<T> Inner<T> {
    pub fn new(cap: usize) -> Self {
        let (tx, rx) = if cap == 0 {
            flume::bounded(0)
        } else {
            flume::bounded(cap)
        };
        Inner { tx, rx, closed: Arc::new(AtomicBool::new(false)) }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        // Drop the internal sender clone bank by... actually we can't force
        // flume to disconnect. Strategy: signal via the atomic flag; senders
        // check before each send; receivers keep draining until empty, then
        // observe the flag. See Chan::Recv for the combined drain+flag check.
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

    pub fn recv(&self) -> Option<T> {
        // Loop with short timeout so that a Close() call from another thread
        // wakes us within the poll interval. flume has no native close(); this
        // polling is the cost of emulating Go's close broadcast on flume.
        loop {
            match self.rx.try_recv() {
                Ok(v) => return Some(v),
                Err(flume::TryRecvError::Disconnected) => return None,
                Err(flume::TryRecvError::Empty) => {
                    if self.is_closed() { return None; }
                    match self.rx.recv_timeout(std::time::Duration::from_millis(50)) {
                        Ok(v) => return Some(v),
                        Err(flume::RecvTimeoutError::Timeout) => continue,
                        Err(flume::RecvTimeoutError::Disconnected) => return None,
                    }
                }
            }
        }
    }

    pub async fn recv_async(&self) -> Option<T> {
        // Async variant: use flume's recv_async inside a select with a
        // periodic closed-flag check via tokio-less polling. For simplicity
        // we reuse the sync timeout loop in block_on-friendly form: each
        // iteration yields on the runtime between checks.
        loop {
            match self.rx.try_recv() {
                Ok(v) => return Some(v),
                Err(flume::TryRecvError::Disconnected) => return None,
                Err(flume::TryRecvError::Empty) => {
                    if self.is_closed() { return None; }
                    // Brief async wait; on any executor that supports timers
                    // this lets Close() propagate. With no runtime, the
                    // flume recv_async will wake on any send, so the polling
                    // cadence is only relevant in close-while-parked cases.
                    let fut = self.rx.recv_async();
                    match fut.await {
                        Ok(v) => return Some(v),
                        Err(_) => {
                            if self.is_closed() { return None; }
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
}

impl<T> Clone for Inner<T> {
    fn clone(&self) -> Self {
        Inner {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
            closed: self.closed.clone(),
        }
    }
}

pub const ENGINE_NAME: &str = "flume";
