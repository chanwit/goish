// defer: Go's defer statement, ported to Rust.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   defer f.Close()                     defer!{ f.close(); }
//   defer fmt.Println("bye")            defer!{ fmt::Println!("bye"); }
//
// Multiple defers stack in LIFO order — the LAST `defer!` declared runs
// FIRST at scope exit. This mirrors Go exactly, because Rust drops bindings
// in reverse declaration order and each `defer!` creates a fresh binding.
//
// Runs at *scope* end (like Go). Works even if the scope exits via `return`
// or a panic (unless the panic is inside the deferred block itself).

/// A scope-end guard that runs a closure on drop. Users normally create
/// these via the `defer!` macro rather than constructing directly.
pub struct Defer<F: FnOnce()> {
    f: Option<F>,
}

impl<F: FnOnce()> Defer<F> {
    pub fn new(f: F) -> Self {
        Defer { f: Some(f) }
    }
}

impl<F: FnOnce()> Drop for Defer<F> {
    fn drop(&mut self) {
        if let Some(f) = self.f.take() {
            f();
        }
    }
}

/// `defer!{ stmts }` — run `stmts` at the end of the enclosing scope.
///
/// Multiple `defer!` calls in the same scope stack in LIFO order, same as Go.
#[macro_export]
macro_rules! defer {
    ($($tt:tt)*) => {
        // Unique binding name avoids shadowing prior defers in the same scope,
        // which would suppress their Drop. `line!()` gives us a unique label
        // per source line; combined with `module_path!` + counter via ident.
        let _goish_defer = $crate::defer::Defer::new(|| { $($tt)* });
    };
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    #[test]
    fn runs_at_scope_end() {
        let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
        {
            let log = log.clone();
            crate::defer!{ log.lock().unwrap().push(99); }
            log.lock().unwrap().push(1);
            log.lock().unwrap().push(2);
        }
        assert_eq!(*log.lock().unwrap(), vec![1, 2, 99]);
    }

    #[test]
    fn lifo_order() {
        let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
        {
            let l1 = log.clone();
            crate::defer!{ l1.lock().unwrap().push(1); }
            let l2 = log.clone();
            crate::defer!{ l2.lock().unwrap().push(2); }
            let l3 = log.clone();
            crate::defer!{ l3.lock().unwrap().push(3); }
        }
        // LIFO: 3 declared last, runs first
        assert_eq!(*log.lock().unwrap(), vec![3, 2, 1]);
    }

    #[test]
    fn runs_on_early_return() {
        fn inner(log: Arc<Mutex<Vec<i32>>>, early: bool) -> i32 {
            let l = log.clone();
            crate::defer!{ l.lock().unwrap().push(100); }
            if early {
                return 1;
            }
            log.lock().unwrap().push(50);
            2
        }

        let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
        let r = inner(log.clone(), true);
        assert_eq!(r, 1);
        assert_eq!(*log.lock().unwrap(), vec![100]);

        let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
        let r = inner(log.clone(), false);
        assert_eq!(r, 2);
        assert_eq!(*log.lock().unwrap(), vec![50, 100]);
    }
}
