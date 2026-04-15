//! chan/engine: flume-backed channel engine.
//!
//! Flume is MPMC, bounded/unbounded, has both sync and async receivers.
//! Gap vs Go: no explicit `close()` that wakes parked receivers. We emulate
//! via a shared `AtomicBool` (fast check for senders) + a closable
//! `tokio::sync::Semaphore` that parked async receivers select on. When
//! `Chan::Close()` fires, the semaphore is closed and every async waiter
//! wakes within one scheduler cycle.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct Inner<T> {
    tx: flume::Sender<T>,
    rx: flume::Receiver<T>,
    closed: Arc<AtomicBool>,
    /// Never-acquired-as-permit semaphore; `.close()` on it makes every
    /// pending `acquire()` return `Err(AcquireError)`. Async receivers
    /// select-await on this so `Chan::Close()` wakes them.
    close_signal: Arc<Semaphore>,
}

impl<T> Inner<T> {
    pub fn new(cap: usize) -> Self {
        let (tx, rx) = if cap == 0 {
            flume::bounded(0)
        } else {
            flume::bounded(cap)
        };
        Inner {
            tx, rx,
            closed: Arc::new(AtomicBool::new(false)),
            close_signal: Arc::new(Semaphore::new(0)),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.close_signal.close();
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
}

impl<T> Clone for Inner<T> {
    fn clone(&self) -> Self {
        Inner {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
            closed: self.closed.clone(),
            close_signal: self.close_signal.clone(),
        }
    }
}

pub const ENGINE_NAME: &str = "flume";
