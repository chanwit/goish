// context: Go's context package (subset).
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   ctx := context.Background()         let ctx = context::Background();
//   ctx, cancel := context.WithCancel(p) let (ctx, cancel) = context::WithCancel(p);
//   defer cancel()                      defer!{ cancel.call(); }
//   ctx, _ := context.WithTimeout(...)  let (ctx, _) = context::WithTimeout(p, d);
//   <-ctx.Done()                        let (_, _) = ctx.Done().Recv();
//   select { case <-ctx.Done(): ... }   select!{ recv(ctx.Done()) => {...}, ... }
//   if ctx.Err() != nil { ... }         if ctx.Err() != nil { ... }
//
// Done() returns a `Chan<()>` that is closed when this context (or any
// ancestor) is cancelled. `Close()` on a goish channel wakes every parked
// receiver, giving Go's `<-ctx.Done()` broadcast shape exactly.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct Context {
    inner: Arc<ContextInner>,
}

/// Default = `context.Background()`. Matches Go's zero-value context.
impl Default for Context {
    fn default() -> Self { Background() }
}

struct ContextInner {
    cancelled: AtomicBool,
    err_mu: Mutex<Option<crate::errors::error>>,
    cv: Condvar,
    cv_mu: Mutex<()>,
    done_ch: crate::chan::Chan<()>,
    parent: Option<Arc<ContextInner>>,
    values: Mutex<HashMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl Context {
    /// ctx.Done() — returns a channel that is closed when this context
    /// (or any ancestor) is cancelled. Usage:
    ///
    ///   let (_, _) = ctx.Done().Recv();          // block until cancel
    ///   select!{ recv(ctx.Done()) => { ... } }   // non-blocking / racing
    ///
    /// Matches Go's `<-ctx.Done()` channel shape.
    pub fn Done(&self) -> crate::chan::Chan<()> {
        self.inner.done_ch.clone()
    }

    /// ctx.Wait() — convenience: blocks the current thread until this
    /// context is cancelled. Equivalent to `ctx.Done().Recv()` but without
    /// the tuple.
    pub fn Wait(&self) {
        let _ = self.inner.done_ch.Recv();
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
        done_ch: crate::chan::Chan::<()>::new(0),
        parent,
        values: Mutex::new(HashMap::new()),
    })
}

fn fire_cancel(inner: &Arc<ContextInner>, reason: &'static str) {
    if !inner.cancelled.swap(true, Ordering::SeqCst) {
        *inner.err_mu.lock().unwrap() = Some(crate::errors::New(reason));
        // Close the done channel — broadcasts to every parked receiver.
        inner.done_ch.Close();
        // Legacy condvar path — Wait() uses it when called on an
        // already-cancelled context (recv short-circuits to (_, false)).
        let _g = inner.cv_mu.lock().unwrap();
        inner.cv.notify_all();
    }
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
    pub fn call(&self) { fire_cancel(&self.inner, self.reason); }
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
    // If parent is already cancelled, fire immediately.
    if parent.is_cancelled() {
        fire_cancel(&inner, "context canceled");
    } else {
        // Otherwise spawn a tiny watcher: when parent closes its done
        // channel, propagate to our child context.
        let child = inner.clone();
        let parent_done = parent.inner.done_ch.clone();
        std::thread::spawn(move || {
            let _ = parent_done.Recv();
            fire_cancel(&child, "context canceled");
        });
    }
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
        fire_cancel(&ctx_inner, "context deadline exceeded");
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
        ctx.Wait();  // blocks until timeout
        assert!(ctx.Err() != crate::errors::nil);
        assert!(format!("{}", ctx.Err()).contains("deadline"));
    }

    #[test]
    fn done_channel_closes_on_cancel() {
        // ctx.Done() returns a channel; Close-on-cancel gives us a
        // broadcast so multiple readers can race-detect cancellation.
        let (ctx, cancel) = WithCancel(Background());
        let d1 = ctx.Done();
        let d2 = ctx.Done();
        let start = std::time::Instant::now();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            cancel.call();
        });
        // Both should see (zero, false) once the channel is closed.
        let (_, ok1) = d1.Recv();
        let (_, ok2) = d2.Recv();
        assert!(!ok1 && !ok2);
        assert!(start.elapsed() >= std::time::Duration::from_millis(15));
    }

    #[test]
    fn select_on_done_fires_when_cancelled() {
        let (ctx, cancel) = WithCancel(Background());
        cancel.call();
        let mut fired = false;
        crate::select!{
            recv(ctx.Done()) => { fired = true; },
            default => {},
        }
        assert!(fired, "select should see closed ctx.Done()");
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
        ctx.Wait();
        assert!(format!("{}", ctx.Err()).contains("deadline"));
    }
}
