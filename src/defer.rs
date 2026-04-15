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

/// `recover!{ body }` — runs `body`; if it panics, captures the panic message
/// as `Some(String)`. If it completes normally, returns `None`.
///
/// Maps Go's `defer func() { if r := recover(); r != nil { ... } }()` pattern.
/// The difference: Go's defer-recover catches panics from surrounding code in
/// the same function; Rust requires the risky code to live inside a closure.
/// The goish macro wraps the block for you.
///
///   Go:
///       defer func() {
///           if r := recover(); r == nil {
///               t.Fatal("expected panic")
///           }
///       }()
///       FormatUint(12345678, 1)
///
///   goish:
///       let r = recover!{ strconv::FormatUint(12345678, 1) };
///       if r.is_none() { t.Fatal("expected panic"); }
#[macro_export]
macro_rules! recover {
    ($($body:tt)*) => {{
        let __result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
            $($body)*
        }));
        match __result {
            ::std::result::Result::Ok(_) => ::std::option::Option::<::std::string::String>::None,
            ::std::result::Result::Err(e) => {
                let msg = if let ::std::option::Option::Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let ::std::option::Option::Some(s) = e.downcast_ref::<::std::string::String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                ::std::option::Option::Some(msg)
            }
        }
    }};
}

#[cfg(test)]
mod recover_tests {
    #[test]
    fn recover_returns_none_on_no_panic() {
        let r = crate::recover!{ let _x = 1 + 1; };
        assert!(r.is_none());
    }

    #[test]
    fn recover_captures_panic_message() {
        let r = crate::recover!{ panic!("boom"); };
        assert_eq!(r.as_deref(), Some("boom"));
    }

    #[test]
    fn recover_captures_string_panic() {
        let r = crate::recover!{ panic!("{}", "formatted {}"); };
        assert!(r.is_some());
    }

    #[test]
    fn recover_matches_go_illegal_base_pattern() {
        // Simulates itoa_test.go's illegal-base panic check.
        let r = crate::recover!{ crate::strconv::FormatUint(12345678, 1); };
        assert!(r.is_some());
    }
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
