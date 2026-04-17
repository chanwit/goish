// Coverage for v0.20.5 custom-error escape hatch (friction #37):
//   - errors::GoishError trait — user types can implement `.Error()` and
//     be lifted into goish's `error` via `errors::FromDyn`
//   - err.downcast_ref::<T>() — recover the original user type
//   - multierr-shaped port: MultiError holds a slice<error>, preserves
//     individual entries through the error return boundary.

use goish::prelude::*;
use goish::errors::GoishError;
use std::any::Any;
use std::fmt;

// A minimal multierr.
#[derive(Debug, Clone)]
struct MultiError {
    errs: slice<error>,
}

impl fmt::Display for MultiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.errs.iter().enumerate() {
            if i > 0 { f.write_str("; ")?; }
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

impl GoishError for MultiError {
    fn as_any(&self) -> &dyn Any { self }
}

fn wrap_multi(es: slice<error>) -> error {
    errors::FromDyn(MultiError { errs: es })
}

test!{ fn TestFromDyn_RoundTrips(t) {
    let a = errors::New("one");
    let b = errors::New("two");
    let err = wrap_multi(vec![a, b].into());

    // It's a non-nil error.
    if err == nil { t.Errorf("FromDyn returned nil".to_string()); }

    // Display / Sprintf see the Display impl.
    let s = Sprintf!("%s", err);
    if !strings::Contains(&s, "one") || !strings::Contains(&s, "two") {
        t.Errorf(Sprintf!("Display lost entries: got %q", s));
    }

    // Downcast recovers the original type AND the individual errors.
    let me = match err.downcast_ref::<MultiError>() {
        Some(x) => x,
        None => { t.Fatal("downcast_ref failed"); return; }
    };
    if len!(me.errs) != 2 { t.Errorf(Sprintf!("want 2 errs, got %d", len!(me.errs))); }
    if Sprintf!("%s", me.errs[0i64]) != "one" { t.Errorf("errs[0] != one".to_string()); }
    if Sprintf!("%s", me.errs[1i64]) != "two" { t.Errorf("errs[1] != two".to_string()); }
}}

test!{ fn TestFromDyn_PtrIdentityEq(t) {
    let a = wrap_multi(vec![errors::New("x")].into());
    let b = a.clone();
    // Clone shares the underlying Arc — PartialEq checks ptr identity.
    if a != b { t.Errorf("clone of Custom error not ==".to_string()); }

    let c = wrap_multi(vec![errors::New("x")].into());
    // Different Arc even with same contents — not equal (matches Go's
    // pointer-receiver == semantics).
    if a == c { t.Errorf("distinct Custom errors unexpectedly ==".to_string()); }
}}

test!{ fn TestFromDyn_BuiltinDowncastReturnsNone(t) {
    let e = errors::New("boom");
    if e.downcast_ref::<MultiError>().is_some() {
        t.Errorf("Builtin error shouldn't downcast to user type".to_string());
    }
}}

test!{ fn TestFromDyn_NilDowncastReturnsNone(t) {
    let e: error = nil;
    if e.downcast_ref::<MultiError>().is_some() {
        t.Errorf("nil error shouldn't downcast".to_string());
    }
}}

// Custom Unwrap chain — proves errors::Is walks through.
#[derive(Debug, Clone)]
struct CausedBy { msg: String, cause: error }
impl fmt::Display for CausedBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.msg, self.cause)
    }
}
impl GoishError for CausedBy {
    fn as_any(&self) -> &dyn Any { self }
    fn Unwrap(&self) -> error { self.cause.clone() }
}

test!{ fn TestIs_WalksCustomUnwrap(t) {
    let inner = errors::New("disk full");
    let outer = errors::FromDyn(CausedBy { msg: "save failed".into(), cause: inner.clone() });
    // Is should find the inner by walking .Unwrap().
    if !errors::Is(&outer, &inner) {
        t.Errorf("Is did not walk CausedBy.Unwrap chain".to_string());
    }
}}

test!{ fn TestAppend_WithCustomError(t) {
    let builtin = errors::New("first");
    let custom = wrap_multi(vec![errors::New("nested")].into());
    let both = errors::Append(builtin, custom);
    let s = Sprintf!("%s", both);
    if !strings::Contains(&s, "first") || !strings::Contains(&s, "nested") {
        t.Errorf(Sprintf!("Append dropped a message: got %q", s));
    }
}}
