// context: Go's context package (subset).
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   ctx := context.Background()         let ctx = context::Background();
//   ctx, cancel := context.WithCancel(p) let (ctx, cancel) = context::WithCancel(p);
//   defer cancel()                      defer!{ cancel(); }
//   ctx, _ := context.WithTimeout(...)  let (ctx, _) = context::WithTimeout(p, d);
//   <-ctx.Done()                        ctx.Done()  // blocks until cancelled
//   if ctx.Err() != nil { ... }         if ctx.Err() != nil { ... }
//
// Implemented with Arc<AtomicBool> for the cancel flag plus a condvar for
// blocking waiters. No goroutine-per-context scheduling.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct Context {
    inner: Arc<ContextInner>,
}

struct ContextInner {
    cancelled: AtomicBool,
    err_mu: Mutex<Option<crate::errors::error>>,
    cv: Condvar,
    cv_mu: Mutex<()>,
    parent: Option<Arc<ContextInner>>,
    values: Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl Context {
    /// ctx.Done() — blocks until this context is cancelled (or parent is).
    pub fn Done(&self) {
        let mut g = self.inner.cv_mu.lock().unwrap();
        while !self.is_cancelled() {
            g = self.inner.cv.wait(g).unwrap();
        }
    }

    /// ctx.Err() — nil if not cancelled, otherwise the cancellation error.
    pub fn Err(&self) -> crate::errors::error {
        if self.is_cancelled() {
            self.inner.err_mu.lock().unwrap()
                .clone()
                .unwrap_or_else(|| crate::errors::New("context canceled"))
        } else {
            crate::errors::nil
        }
    }

    fn is_cancelled(&self) -> bool {
        let mut cur = Some(&self.inner);
        while let Some(n) = cur {
            if n.cancelled.load(Ordering::SeqCst) { return true; }
            cur = n.parent.as_ref();
        }
        false
    }

    /// ctx.Value(key) — look up a value stored via WithValue in this context
    /// or any ancestor. Returns None if unset.
    #[allow(non_snake_case)]
    pub fn Value<T: Any + Send + Sync + Clone>(&self, key: impl AsRef<str>) -> Option<T> {
        let k = key.as_ref();
        let mut cur = Some(&self.inner);
        while let Some(n) = cur {
            if let Some(v) = n.values.lock().unwrap().get(k) {
                if let Some(typed) = v.downcast_ref::<T>() {
                    return Some(typed.clone());
                }
            }
            cur = n.parent.as_ref();
        }
        None
    }
}

fn new_inner(parent: Option<Arc<ContextInner>>) -> Arc<ContextInner> {
    Arc::new(ContextInner {
        cancelled: AtomicBool::new(false),
        err_mu: Mutex::new(None),
        cv: Condvar::new(),
        cv_mu: Mutex::new(()),
        parent,
        values: Mutex::new(HashMap::new()),
    })
}

/// context.Background() — root context that is never cancelled.
#[allow(non_snake_case)]
pub fn Background() -> Context {
    Context { inner: new_inner(None) }
}

/// A cancellation function produced by WithCancel / WithTimeout.
pub struct CancelFunc {
    inner: Arc<ContextInner>,
    reason: &'static str,
}

impl CancelFunc {
    pub fn call(&self) {
        if !self.inner.cancelled.swap(true, Ordering::SeqCst) {
            *self.inner.err_mu.lock().unwrap() =
                Some(crate::errors::New(self.reason));
            let _g = self.inner.cv_mu.lock().unwrap();
            self.inner.cv.notify_all();
        }
    }
}

/// context.WithCancel(parent) — returns (ctx, cancel). Call cancel() to
/// cancel; run it via `defer!{ cancel.call(); }`.
#[allow(non_snake_case)]
pub fn WithCancel(parent: Context) -> (Context, CancelFunc) {
    let inner = new_inner(Some(parent.inner.clone()));
    let cancel = CancelFunc {
        inner: inner.clone(),
        reason: "context canceled",
    };
    (Context { inner }, cancel)
}

/// context.WithValue(parent, key, value) — child context that answers
/// Value(key) with the given value.
#[allow(non_snake_case)]
pub fn WithValue<T: Any + Send + Sync>(parent: Context, key: impl Into<String>, value: T) -> Context {
    let inner = new_inner(Some(parent.inner.clone()));
    inner.values.lock().unwrap().insert(key.into(), Arc::new(value));
    Context { inner }
}

/// context.WithDeadline(parent, t) — cancels automatically at the given Time.
#[allow(non_snake_case)]
pub fn WithDeadline(parent: Context, deadline: crate::time::Time) -> (Context, CancelFunc) {
    let d = deadline.Sub(crate::time::Now());
    WithTimeout(parent, d)
}

/// context.WithTimeout(parent, duration) — cancels automatically after `d`.
#[allow(non_snake_case)]
pub fn WithTimeout(parent: Context, d: crate::time::Duration) -> (Context, CancelFunc) {
    let (ctx, cancel) = WithCancel(parent);
    let ctx_inner = ctx.inner.clone();
    std::thread::spawn(move || {
        std::thread::sleep(d.to_std());
        if !ctx_inner.cancelled.swap(true, Ordering::SeqCst) {
            *ctx_inner.err_mu.lock().unwrap() =
                Some(crate::errors::New("context deadline exceeded"));
            let _g = ctx_inner.cv_mu.lock().unwrap();
            ctx_inner.cv.notify_all();
        }
    });
    (ctx, cancel)
}

// ── tests for Value + Deadline ─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time;

    #[test]
    fn background_is_not_cancelled() {
        let ctx = Background();
        assert!(ctx.Err() == crate::errors::nil);
    }

    #[test]
    fn with_cancel_cancels() {
        let ctx = Background();
        let (ctx, cancel) = WithCancel(ctx);
        assert!(ctx.Err() == crate::errors::nil);
        cancel.call();
        assert!(ctx.Err() != crate::errors::nil);
        assert!(format!("{}", ctx.Err()).contains("canceled"));
    }

    #[test]
    fn with_timeout_cancels_after_duration() {
        let ctx = Background();
        let (ctx, _cancel) = WithTimeout(ctx, time::Millisecond * 30i64);
        assert!(ctx.Err() == crate::errors::nil);
        ctx.Done();  // blocks until timeout
        assert!(ctx.Err() != crate::errors::nil);
        assert!(format!("{}", ctx.Err()).contains("deadline"));
    }

    #[test]
    fn cancelling_parent_propagates_to_child() {
        let (parent, pcancel) = WithCancel(Background());
        let (child, _ccancel) = WithCancel(parent);
        pcancel.call();
        assert!(child.Err() != crate::errors::nil);
    }

    #[test]
    fn value_stores_and_retrieves_through_ancestors() {
        let ctx = Background();
        let ctx = WithValue(ctx, "user", 42i64);
        let ctx = WithValue(ctx, "role", "admin".to_string());
        let (ctx, _c) = WithCancel(ctx);  // nested past a non-value frame

        let user: Option<i64> = ctx.Value("user");
        assert_eq!(user, Some(42));
        let role: Option<String> = ctx.Value("role");
        assert_eq!(role.as_deref(), Some("admin"));
        let missing: Option<i64> = ctx.Value("missing");
        assert_eq!(missing, None);
    }

    #[test]
    fn with_deadline_cancels_at_time() {
        let ctx = Background();
        let deadline = crate::time::Now().Add(crate::time::Millisecond * 30i64);
        let (ctx, _c) = WithDeadline(ctx, deadline);
        ctx.Done();
        assert!(format!("{}", ctx.Err()).contains("deadline"));
    }
}
