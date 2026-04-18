// Coverage for v0.21.0 `ErrorType!` macro (friction #37, re-tested):
//   - one-line error-type declarations for user error shapes
//   - `.into()` lifting — no `FromDyn` at user call sites
//   - `errors::As::<T>(&err) -> Option<&T>` — Go-shaped recovery, no
//     `.downcast_ref::<T>()` at user call sites
//   - multierr-shaped port: MultiError holds a slice<error>, preserves
//     individual entries through the error return boundary.
//
// NO `Box<dyn>`, `as_any`, `FromDyn`, or `downcast_ref` leak to the
// call site. If any of those reappears here, it's a Rust-leak regression.

use goish::prelude::*;

ErrorType!{
    type MultiError struct {
        errs: slice<error>,
    }
    fn Error(&self) -> string {
        let mut buf = strings::Builder::new();
        for (i, e) in self.errs.iter().enumerate() {
            if i > 0 { buf.WriteString("; "); }
            buf.WriteString(&Sprintf!("%s", e));
        }
        buf.String()
    }
}

fn wrap_multi(es: slice<error>) -> error {
    MultiError { errs: es }.into()
}

test!{ fn TestErrorType_RoundTrips(t) {
    let a = errors::New("one");
    let b = errors::New("two");
    let err = wrap_multi(vec![a, b].into());

    // It's a non-nil error.
    if err == nil { t.Errorf("ErrorType!.into() returned nil".to_string()); }

    // Display / Sprintf see the generated Display impl.
    let s = Sprintf!("%s", err);
    if !strings::Contains(&s, "one") || !strings::Contains(&s, "two") {
        t.Errorf(Sprintf!("Display lost entries: got %q", s));
    }

    // errors::As::<T> recovers the original type AND the individual errors.
    let me = match errors::As::<MultiError>(&err) {
        Some(x) => x,
        None => return t.Fatal("errors::As::<MultiError> failed"),
    };
    if len!(me.errs) != 2 { t.Errorf(Sprintf!("want 2 errs, got %d", len!(me.errs))); }
    if Sprintf!("%s", me.errs[0i64]) != "one" { t.Errorf("errs[0] != one".to_string()); }
    if Sprintf!("%s", me.errs[1i64]) != "two" { t.Errorf("errs[1] != two".to_string()); }
}}

test!{ fn TestErrorType_PtrIdentityEq(t) {
    let a = wrap_multi(vec![errors::New("x")].into());
    let b = a.clone();
    // Clone shares the underlying Arc — PartialEq checks ptr identity.
    if a != b { t.Errorf("clone of Custom error not ==".to_string()); }

    let c = wrap_multi(vec![errors::New("x")].into());
    // Different Arc even with same contents — not equal (matches Go's
    // pointer-receiver == semantics).
    if a == c { t.Errorf("distinct Custom errors unexpectedly ==".to_string()); }
}}

test!{ fn TestErrorType_BuiltinAsReturnsNone(t) {
    let e = errors::New("boom");
    if errors::As::<MultiError>(&e).is_some() {
        t.Errorf("Builtin error shouldn't recover as MultiError".to_string());
    }
}}

test!{ fn TestErrorType_NilAsReturnsNone(t) {
    let e: error = nil;
    if errors::As::<MultiError>(&e).is_some() {
        t.Errorf("nil error shouldn't recover as any user type".to_string());
    }
}}

// Second ErrorType! — confirms the macro handles more than one in a
// single compilation unit (no name collisions on hidden helpers).
ErrorType!{
    type CausedBy struct {
        msg: string,
        cause: error,
    }
    fn Error(&self) -> string {
        Sprintf!("%s: %s", self.msg, self.cause)
    }
}

test!{ fn TestErrorType_Is_OnIdentity(t) {
    // errors::Is on custom types matches by Arc pointer identity
    // (matches the original Custom/Custom PartialEq contract).
    let outer: error = CausedBy {
        msg: "save failed".into(),
        cause: errors::New("disk full"),
    }.into();
    let other = outer.clone();
    if !errors::Is(&outer, &other) {
        t.Errorf("Is did not recognise Arc-identical errors".to_string());
    }

    // Display composes the user's Error() body.
    let s = Sprintf!("%s", outer);
    if !strings::Contains(&s, "save failed") || !strings::Contains(&s, "disk full") {
        t.Errorf(Sprintf!("CausedBy display lost parts: %q", s));
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
