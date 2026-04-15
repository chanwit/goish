//! sync/atomic: atomic integer and boolean wrappers.
//!
//! Go returns the new value after Add; std returns the old — our wrapper
//! fixes this so the call site reads Go-identical.

use std::sync::atomic::{
    AtomicBool as StdAtomicBool, AtomicI32 as StdAtomicI32, AtomicI64 as StdAtomicI64,
    AtomicU32 as StdAtomicU32, AtomicU64 as StdAtomicU64, Ordering,
};
use std::sync::Arc;

macro_rules! atomic_int {
    ($name:ident, $std:ident, $ty:ty) => {
        #[derive(Clone, Default)]
        pub struct $name {
            inner: Arc<$std>,
        }

        impl $name {
            pub fn new(v: $ty) -> Self {
                Self { inner: Arc::new($std::new(v)) }
            }

            pub fn Load(&self) -> $ty {
                self.inner.load(Ordering::SeqCst)
            }

            pub fn Store(&self, v: $ty) {
                self.inner.store(v, Ordering::SeqCst);
            }

            pub fn Add(&self, delta: $ty) -> $ty {
                // Go returns the new value (post-add); std returns old.
                self.inner.fetch_add(delta, Ordering::SeqCst).wrapping_add(delta)
            }

            pub fn Swap(&self, v: $ty) -> $ty {
                self.inner.swap(v, Ordering::SeqCst)
            }

            pub fn CompareAndSwap(&self, old: $ty, new: $ty) -> bool {
                self.inner
                    .compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
            }
        }
    };
}

atomic_int!(Int32, StdAtomicI32, i32);
atomic_int!(Int64, StdAtomicI64, i64);
atomic_int!(Uint32, StdAtomicU32, u32);
atomic_int!(Uint64, StdAtomicU64, u64);

#[derive(Clone, Default)]
pub struct Bool {
    inner: Arc<StdAtomicBool>,
}

impl Bool {
    pub fn new(v: bool) -> Self {
        Self { inner: Arc::new(StdAtomicBool::new(v)) }
    }
    pub fn Load(&self) -> bool { self.inner.load(Ordering::SeqCst) }
    pub fn Store(&self, v: bool) { self.inner.store(v, Ordering::SeqCst); }
    pub fn Swap(&self, v: bool) -> bool { self.inner.swap(v, Ordering::SeqCst) }
    pub fn CompareAndSwap(&self, old: bool, new: bool) -> bool {
        self.inner.compare_exchange(old, new, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int64_load_store_add() {
        let n = Int64::new(10);
        assert_eq!(n.Load(), 10);
        n.Store(42);
        assert_eq!(n.Load(), 42);
        let post = n.Add(8);
        assert_eq!(post, 50);
        assert_eq!(n.Load(), 50);
    }

    #[test]
    fn int32_swap_and_cas() {
        let n = Int32::new(1);
        let old = n.Swap(99);
        assert_eq!(old, 1);
        assert_eq!(n.Load(), 99);
        assert!(n.CompareAndSwap(99, 100));
        assert_eq!(n.Load(), 100);
        assert!(!n.CompareAndSwap(99, 200));
        assert_eq!(n.Load(), 100);
    }

    #[test]
    fn bool_atomic() {
        let b = Bool::new(false);
        assert!(!b.Load());
        b.Store(true);
        assert!(b.Load());
        let old = b.Swap(false);
        assert!(old);
        assert!(!b.Load());
    }

    #[test]
    fn cross_thread_counter() {
        let n = Int64::new(0);
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let c = n.clone();
                std::thread::spawn(move || {
                    for _ in 0..1000 { c.Add(1); }
                })
            })
            .collect();
        for h in handles { h.join().unwrap(); }
        assert_eq!(n.Load(), 8000);
    }
}
