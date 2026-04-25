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

use std::sync::{Arc, Condvar, Mutex as StdMutex, MutexGuard as StdMutexGuard,
                RwLock as StdRwLock,
                RwLockReadGuard as StdRwLockReadGuard,
                RwLockWriteGuard as StdRwLockWriteGuard};
use std::sync::atomic::{AtomicI64, Ordering};

// ── Mutex ──────────────────────────────────────────────────────────────

/// Protected data wrapped by a mutex. `Lock()` returns a guard that auto-
/// unlocks on drop — closest Rust analog to Go's `defer mu.Unlock()`.
#[derive(Clone)]
pub struct Mutex<T: ?Sized> {
    inner: Arc<StdMutex<T>>,
}

/// Goish-shaped guard returned by `Mutex::Lock` / `TryLock`. Wraps
/// `std::sync::MutexGuard` so the std type name doesn't surface in
/// rustdoc / IDE return-type tooltips. Transparent at call sites via
/// `Deref` + `DerefMut` (`*g = v`, `g.field`, `g.method()`).
pub struct MutexGuard<'a, T: ?Sized> {
    #[doc(hidden)]
    pub inner: StdMutexGuard<'a, T>,
}

impl<'a, T: ?Sized> std::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { &self.inner }
}
impl<'a, T: ?Sized> std::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T { &mut self.inner }
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
    ///
    /// Absorbs Rust's poisoning: if a previous holder panicked, the next
    /// Lock() still succeeds and observes the post-panic state. Matches
    /// Go's `sync.Mutex`, which has no poison concept.
    pub fn Lock(&self) -> MutexGuard<'_, T> {
        MutexGuard { inner: self.inner.lock().unwrap_or_else(|p| p.into_inner()) }
    }

    /// mu.TryLock() — non-blocking. Returns the guard when the lock is
    /// available (including the poisoned case, matching Go semantics),
    /// None only when the lock is currently held by another thread.
    pub fn TryLock(&self) -> Option<MutexGuard<'_, T>> {
        use std::sync::TryLockError;
        match self.inner.try_lock() {
            Ok(g) => Some(MutexGuard { inner: g }),
            Err(TryLockError::Poisoned(p)) => Some(MutexGuard { inner: p.into_inner() }),
            Err(TryLockError::WouldBlock) => None,
        }
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

/// Goish-shaped write guard for `RWMutex::Lock` / `TryLock`. Wraps
/// `std::sync::RwLockWriteGuard`. Transparent via `Deref` + `DerefMut`.
pub struct RWMutexWriteGuard<'a, T: ?Sized> {
    #[doc(hidden)]
    pub inner: StdRwLockWriteGuard<'a, T>,
}

impl<'a, T: ?Sized> std::ops::Deref for RWMutexWriteGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { &self.inner }
}
impl<'a, T: ?Sized> std::ops::DerefMut for RWMutexWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T { &mut self.inner }
}

/// Goish-shaped read guard for `RWMutex::RLock` / `TryRLock`. Wraps
/// `std::sync::RwLockReadGuard`. Transparent via `Deref`.
pub struct RWMutexReadGuard<'a, T: ?Sized> {
    #[doc(hidden)]
    pub inner: StdRwLockReadGuard<'a, T>,
}

impl<'a, T: ?Sized> std::ops::Deref for RWMutexReadGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { &self.inner }
}

impl<T> RWMutex<T> {
    pub fn new(v: T) -> Self
    where
        T: Sized,
    {
        RWMutex { inner: Arc::new(StdRwLock::new(v)) }
    }

    /// RWMutex.Lock() / RLock() absorb Rust poisoning the same way
    /// `Mutex::Lock` does — Go's `sync.RWMutex` never poisons.
    pub fn Lock(&self) -> RWMutexWriteGuard<'_, T> {
        RWMutexWriteGuard { inner: self.inner.write().unwrap_or_else(|p| p.into_inner()) }
    }

    pub fn RLock(&self) -> RWMutexReadGuard<'_, T> {
        RWMutexReadGuard { inner: self.inner.read().unwrap_or_else(|p| p.into_inner()) }
    }

    /// Non-blocking write lock. Treats a poisoned lock as available
    /// (matches Go), returns None only when another thread holds it.
    #[allow(non_snake_case)]
    pub fn TryLock(&self) -> Option<RWMutexWriteGuard<'_, T>> {
        use std::sync::TryLockError;
        match self.inner.try_write() {
            Ok(g) => Some(RWMutexWriteGuard { inner: g }),
            Err(TryLockError::Poisoned(p)) => Some(RWMutexWriteGuard { inner: p.into_inner() }),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    /// Non-blocking read lock. Same poison semantics as `TryLock`.
    #[allow(non_snake_case)]
    pub fn TryRLock(&self) -> Option<RWMutexReadGuard<'_, T>> {
        use std::sync::TryLockError;
        match self.inner.try_read() {
            Ok(g) => Some(RWMutexReadGuard { inner: g }),
            Err(TryLockError::Poisoned(p)) => Some(RWMutexReadGuard { inner: p.into_inner() }),
            Err(TryLockError::WouldBlock) => None,
        }
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
            // Counter reached zero — wake all waiters. Poison absorbed
            // to keep Go's panic-transparent semantics (see Mutex::Lock).
            let _g = self.inner.mu.lock().unwrap_or_else(|p| p.into_inner());
            self.inner.cv.notify_all();
        }
    }

    /// wg.Wait() — block until counter reaches zero.
    pub fn Wait(&self) {
        let mut g = self.inner.mu.lock().unwrap_or_else(|p| p.into_inner());
        while self.inner.count.load(Ordering::SeqCst) > 0 {
            g = self.inner.cv.wait(g).unwrap_or_else(|p| p.into_inner());
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

// ── sync/atomic ────────────────────────────────────────────────────────
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   var n atomic.Int64                  let n = sync::atomic::Int64::new(0);
//   n.Store(42)                         n.Store(42);
//   v := n.Load()                       let v = n.Load();
//   old := n.Swap(10)                   let old = n.Swap(10);
//   ok := n.CompareAndSwap(10, 20)      let ok = n.CompareAndSwap(10, 20);
//   n.Add(5)                            n.Add(5);

pub mod atomic;

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
    fn mutex_absorbs_poison_after_panic() {
        // Go's sync.Mutex never poisons. If a holder panics, a later
        // Lock()/TryLock() on another thread must still succeed and
        // observe the mid-panic state, not propagate a poison panic.
        let mu = Mutex::new(0i64);
        let m2 = mu.clone();
        let _ = std::thread::spawn(move || {
            let mut g = m2.Lock();
            *g = 42;
            panic!("deliberate panic while holding lock");
        }).join();

        // Blocking Lock — must not panic, must see the 42 written
        // before the poisoning panic.
        {
            let g = mu.Lock();
            assert_eq!(*g, 42);
        }
        // TryLock — same story; Poisoned must be treated as available.
        {
            let g = mu.TryLock().expect("TryLock after poison must succeed");
            assert_eq!(*g, 42);
        }
    }

    #[test]
    fn rwmutex_absorbs_poison_after_panic() {
        let rw = RWMutex::new(0i64);
        let r2 = rw.clone();
        let _ = std::thread::spawn(move || {
            let mut g = r2.Lock();
            *g = 7;
            panic!("deliberate panic while holding write lock");
        }).join();

        assert_eq!(*rw.Lock(), 7);
        assert_eq!(*rw.RLock(), 7);
        assert_eq!(*rw.TryLock().expect("TryLock after poison"), 7);
        assert_eq!(*rw.TryRLock().expect("TryRLock after poison"), 7);
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
