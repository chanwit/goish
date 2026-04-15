// sync: Go's sync package, ported.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   var mu sync.Mutex                   let mu = sync::Mutex::new();
//   mu.Lock() / mu.Unlock()             let _g = mu.Lock();  // auto-unlock on drop
//
//   var wg sync.WaitGroup               let wg = sync::WaitGroup::new();
//   wg.Add(n) / wg.Done() / wg.Wait()   wg.Add(n); wg.Done(); wg.Wait();
//
//   var once sync.Once                  let once = sync::Once::new();
//   once.Do(func() { ... })             once.Do(|| { ... });
//
// Cloneable handles — each sync primitive wraps an Arc internally so that
// multiple goroutines can share them by `clone()`.

use std::sync::{Arc, Condvar, Mutex as StdMutex, MutexGuard as StdMutexGuard, RwLock as StdRwLock};
use std::sync::atomic::{AtomicI64, Ordering};

// ── Mutex ──────────────────────────────────────────────────────────────

/// Protected data wrapped by a mutex. `Lock()` returns a guard that auto-
/// unlocks on drop — closest Rust analog to Go's `defer mu.Unlock()`.
#[derive(Clone)]
pub struct Mutex<T: ?Sized> {
    inner: Arc<StdMutex<T>>,
}

impl<T> Mutex<T> {
    pub fn new(v: T) -> Self
    where
        T: Sized,
    {
        Mutex { inner: Arc::new(StdMutex::new(v)) }
    }

    /// mu.Lock() — blocks until the mutex is free. Returns a guard that
    /// drops the lock when it goes out of scope.
    pub fn Lock(&self) -> StdMutexGuard<'_, T> {
        self.inner.lock().unwrap()
    }

    /// mu.TryLock() — non-blocking. (value, ok).
    pub fn TryLock(&self) -> Option<StdMutexGuard<'_, T>> {
        self.inner.try_lock().ok()
    }
}

/// Zero-argument form for `var mu sync.Mutex` style usage where you only
/// want to guard code, not data. Hold the guard in a local to keep the
/// lock alive for the block.
impl Mutex<()> {
    pub fn empty() -> Self {
        Mutex::new(())
    }
}

// ── RWMutex ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RWMutex<T: ?Sized> {
    inner: Arc<StdRwLock<T>>,
}

impl<T> RWMutex<T> {
    pub fn new(v: T) -> Self
    where
        T: Sized,
    {
        RWMutex { inner: Arc::new(StdRwLock::new(v)) }
    }

    pub fn Lock(&self) -> std::sync::RwLockWriteGuard<'_, T> {
        self.inner.write().unwrap()
    }

    pub fn RLock(&self) -> std::sync::RwLockReadGuard<'_, T> {
        self.inner.read().unwrap()
    }
}

// ── WaitGroup ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct WaitGroup {
    inner: Arc<WaitGroupInner>,
}

struct WaitGroupInner {
    count: AtomicI64,
    mu: StdMutex<()>,
    cv: Condvar,
}

impl WaitGroup {
    pub fn new() -> Self {
        WaitGroup {
            inner: Arc::new(WaitGroupInner {
                count: AtomicI64::new(0),
                mu: StdMutex::new(()),
                cv: Condvar::new(),
            }),
        }
    }

    /// wg.Add(delta) — increment the counter by delta.
    pub fn Add(&self, delta: crate::types::int64) {
        self.inner.count.fetch_add(delta, Ordering::SeqCst);
    }

    /// wg.Done() — shorthand for Add(-1).
    pub fn Done(&self) {
        let prev = self.inner.count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            // Counter reached zero — wake all waiters.
            let _g = self.inner.mu.lock().unwrap();
            self.inner.cv.notify_all();
        }
    }

    /// wg.Wait() — block until counter reaches zero.
    pub fn Wait(&self) {
        let mut g = self.inner.mu.lock().unwrap();
        while self.inner.count.load(Ordering::SeqCst) > 0 {
            g = self.inner.cv.wait(g).unwrap();
        }
    }
}

impl Default for WaitGroup {
    fn default() -> Self { Self::new() }
}

// ── Once ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Once {
    inner: Arc<std::sync::Once>,
}

impl Once {
    pub fn new() -> Self {
        Once { inner: Arc::new(std::sync::Once::new()) }
    }

    /// once.Do(f) — runs f exactly once across all clones of this Once.
    pub fn Do<F: FnOnce()>(&self, f: F) {
        self.inner.call_once(f);
    }
}

impl Default for Once {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutex_guards_data() {
        let mu = Mutex::new(0i64);
        {
            let mut g = mu.Lock();
            *g += 42;
        }
        assert_eq!(*mu.Lock(), 42);
    }

    #[test]
    fn mutex_try_lock() {
        let mu = Mutex::new(1i64);
        let _g = mu.Lock();
        // Another thread would block; in same thread TryLock returns None
        // because std Mutex is not reentrant.
        assert!(mu.TryLock().is_none());
    }

    #[test]
    fn rwmutex_many_readers() {
        let rw = RWMutex::new(5i64);
        let r1 = rw.RLock();
        let r2 = rw.RLock();
        assert_eq!(*r1 + *r2, 10);
    }

    #[test]
    fn waitgroup_blocks_until_done() {
        let wg = WaitGroup::new();
        wg.Add(3);
        for _ in 0..3 {
            let w = wg.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(10));
                w.Done();
            });
        }
        wg.Wait();
    }

    #[test]
    fn once_runs_body_once() {
        let once = Once::new();
        let counter = Arc::new(AtomicI64::new(0));
        for _ in 0..5 {
            let o = once.clone();
            let c = counter.clone();
            o.Do(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
