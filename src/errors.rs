// errors: Go's errors package, ported.
//
//   Go                                goish
//   ───────────────────────────────   ──────────────────────────────────
//   var ErrX = errors.New("boom")     fn ErrX() -> error { errors::New("boom") }
//   err := errors.New("boom")         let err = errors::New("boom");
//   err := fmt.Errorf("x: %w", e)     let err = errors::Wrap(e, "x");
//   if err == nil { ... }             if err == nil { ... }
//   if err != nil { ... }             if err != nil { ... }
//   if errors.Is(err, ErrX) { ... }   if errors::Is(&err, &ErrX()) { ... }
//   inner := errors.Unwrap(err)       let inner = errors::Unwrap(err);
//
// `error` is a newtype around an optional GoError. Its Display impl prints
// the message (or "<nil>" when nil), so `fmt.Println("error:", err)` works
// the same as in Go without any unwrapping at the call site.

use std::any::Any;
use std::fmt::{self, Debug, Display};
use std::sync::Arc;

// ── GoishError: user-implementable error interface ────────────────────
//
// Go's `error` is an interface `{ Error() string }`. Until v0.20.4 the
// goish `error` was a single concrete newtype — great for Go → goish
// ports of simple error-returning code, but no way to introduce a new
// error *shape* (multierr's list-of-errors, errgroup, wrapped typed
// errors with extra fields). v0.20.5 opens the hatch:
//
//   impl std::fmt::Display for MyErr { ... }
//   impl std::fmt::Debug   for MyErr { ... }
//   impl errors::GoishError for MyErr {
//       fn as_any(&self) -> &dyn std::any::Any { self }
//   }
//
//   let err = errors::FromDyn(MyErr { ... });   // → `error`
//   if let Some(me) = err.downcast_ref::<MyErr>() { ... }   // recover
//
// Trait upcasting to `dyn Any` isn't stable on MSRV 1.70, so we require
// an explicit `as_any` method — one line per impl.

pub trait GoishError: Display + Debug + Send + Sync + 'static {
    /// Go's `.Error()` interface method. Defaults to Display.
    fn Error(&self) -> String { format!("{}", self) }
    /// Go's `errors.Unwrap` contract — return the wrapped error, if any.
    fn Unwrap(&self) -> error { nil }
    /// Explicit upcast for `downcast_ref`. Required impl: `self`.
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoError {
    msg: String,
    source: Option<Box<GoError>>,
    /// When true, Display emits just `msg` (Errorf-produced errors, where
    /// `msg` already contains the source's text verbatim). When false,
    /// Display emits `msg: source` (the classic `Wrap` shape).
    msg_includes_source: bool,
}

impl GoError {
    fn new(msg: impl Into<String>) -> Self {
        GoError { msg: msg.into(), source: None, msg_includes_source: false }
    }
}

impl Display for GoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)?;
        if !self.msg_includes_source {
            if let Some(ref src) = self.source {
                write!(f, ": {}", src)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for GoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|s| s as &dyn std::error::Error)
    }
}

// ── error: the Go-style return type ────────────────────────────────────
//
// Holds either the built-in GoError (fast path, created by `errors::New`
// and `Errorf!`) or an Arc'd user type (`errors::FromDyn`). PartialEq
// distinguishes: Builtin/Builtin → msg equality; Custom/Custom → Arc
// pointer identity (the Go `==` on pointer-receiver error values).

#[derive(Debug, Clone)]
enum ErrorKind {
    Builtin(GoError),
    Custom(Arc<dyn GoishError>),
}

#[derive(Debug, Clone, Default)]
pub struct error(Option<ErrorKind>);

impl PartialEq for error {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(ErrorKind::Builtin(a)), Some(ErrorKind::Builtin(b))) => a == b,
            (Some(ErrorKind::Custom(a)), Some(ErrorKind::Custom(b))) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for error {}

impl error {
    pub fn is_nil(&self) -> bool { self.0.is_none() }

    /// e.Error() — message string (panics if nil, matching Go).
    pub fn Error(&self) -> String {
        match &self.0 {
            Some(ErrorKind::Builtin(e)) => format!("{}", e),
            Some(ErrorKind::Custom(a)) => a.Error(),
            None => panic!("runtime error: invalid memory address or nil pointer dereference"),
        }
    }

    /// Recover the original user type from a `FromDyn`-constructed error.
    /// Returns None for nil errors and for Builtin (message-based) errors.
    pub fn downcast_ref<T: GoishError>(&self) -> Option<&T> {
        match &self.0 {
            Some(ErrorKind::Custom(a)) => a.as_any().downcast_ref::<T>(),
            _ => None,
        }
    }
}

impl Display for error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(ErrorKind::Builtin(e)) => Display::fmt(e, f),
            Some(ErrorKind::Custom(a)) => Display::fmt(&**a, f),
            None => write!(f, "<nil>"),
        }
    }
}

// ── IsNil trait: Go's universal nil check ─────────────────────────────
//
// Go's `nil` is untyped — it adopts the type of whatever it's compared to.
// In goish, `nil` is typed as `error` (for `return nil` ergonomics in the
// 99% case). Cross-type `PartialEq<error>` impls on other nil-able types
// (Chan<T>, future Box<dyn Trait>) simulate Go's untyped nil comparison.
//
// The `IsNil` trait formalizes the contract: any goish type with a nil
// state implements it, enabling generic nil-checking code.
//
// Future (tracked): true polymorphic nil via a `NilValue` unit type +
// `From<NilValue>` for each nil-able type, so `return nil` works for ALL
// nil-able return types (not just error). Would require changing all
// `return nil` to `return nil.into()` or introducing return-type coercion
// via a proc macro. Deferred until the migration cost is justified.

/// Trait for types that have a nil (zero-value) state, mirroring Go's
/// polymorphic `== nil` check.
///
/// Implemented by: `error`, `Chan<T>`. Future: `Box<dyn Trait>` wrappers.
pub trait IsNil {
    fn is_nil(&self) -> bool;
}

impl IsNil for error {
    fn is_nil(&self) -> bool { self.0.is_none() }
}

/// `nil` — the zero value of `error`.
///
/// Also serves as Go's polymorphic nil for comparisons with other nil-able
/// types (Chan<T>) via cross-type `PartialEq` impls. See `IsNil` trait.
///
///   if err == nil { ... }     // error nil check
///   if ch  != nil { ... }     // channel nil check (same `nil`)
///   return nil;               // works in -> error functions
#[allow(non_upper_case_globals)]
pub const nil: error = error(None);

/// `var!` — Go's `var` keyword for lazy-initialized module-level values.
///
/// Three forms:
///
/// ```ignore
/// // Error sentinel (90% case) — RHS is any expr returning `error`.
/// // Type defaults to `error`, preserving the Go source expression:
/// var!(ErrShortRead = Errorf!("ioutil: short read"));
/// var!(ErrExpectEOF = errors::New("ioutil: expect EOF"));
///
/// // Typed lazy value — explicit type for non-error values:
/// var!(DefaultTimeout time::Duration = time::Second * 30i64);
///
/// // Block form — Go's `var ( ... )`:
/// var! {
///     ErrShortRead = Errorf!("ioutil: short read");
///     ErrExpectEOF = errors::New("ioutil: expect EOF");
/// }
/// ```
///
/// Each expands to a zero-arg `pub fn` backed by `OnceLock` — one
/// allocation per process lifetime. Call-site: `ErrShortRead()`.
#[macro_export]
macro_rules! var {
    // Block form: var! { Name = expr; Name2 = expr2; ... }
    ({ $( $name:ident = $expr:expr );+ $(;)? }) => {
        $( $crate::var!($name = $expr); )+
    };
    // Block form (typed): var! { Name Type = expr; ... }
    ({ $( $name:ident $t:ty = $expr:expr );+ $(;)? }) => {
        $( $crate::var!($name $t = $expr); )+
    };

    // Error sentinel: var!(Name = expr) — return type defaults to `error`
    ($name:ident = $expr:expr) => {
        #[allow(non_snake_case)]
        pub fn $name() -> $crate::errors::error {
            static __ONCE: ::std::sync::OnceLock<$crate::errors::error>
                = ::std::sync::OnceLock::new();
            __ONCE.get_or_init(|| $expr).clone()
        }
    };

    // Typed lazy value: var!(Name Type = expr)
    ($name:ident $t:ty = $expr:expr) => {
        #[allow(non_snake_case)]
        pub fn $name() -> $t {
            static __ONCE: ::std::sync::OnceLock<$t>
                = ::std::sync::OnceLock::new();
            __ONCE.get_or_init(|| $expr).clone()
        }
    };
}

/// Backwards-compat alias — prefer `var!` in new code.
#[macro_export]
#[doc(hidden)]
macro_rules! static_err {
    ($name:ident = $msg:expr) => {
        $crate::var!($name = $crate::errors::New($msg));
    };
}

// ── errors.{New, Wrap, Is, Unwrap} ─────────────────────────────────────

pub fn New(msg: &str) -> error {
    error(Some(ErrorKind::Builtin(GoError::new(msg))))
}

/// Lift any user-defined `GoishError` into an `error`. The value is
/// Arc-wrapped so the resulting `error` is still cheap to clone.
/// Recover via `err.downcast_ref::<T>()`.
#[allow(non_snake_case)]
pub fn FromDyn<T: GoishError>(e: T) -> error {
    error(Some(ErrorKind::Custom(Arc::new(e))))
}

/// Internal helper — build an error with a specific source chain. Used
/// by `fmt::Errorf!` when the format string contains `%w`. The `msg`
/// already contains the wrapped error's text (the format scanner
/// substituted `%w` with `.Error()`), so Display should emit only `msg`
/// and not re-concatenate the source.
#[doc(hidden)]
#[allow(non_snake_case)]
pub fn New_with_source(msg: &str, source: error) -> error {
    // %w only carries builtin chains. If the source is a Custom, capture
    // its message at wrap time (losing type identity — this matches Go's
    // `Errorf("%w", custom)` which returns a fresh *wrapError that walks
    // to the original via Unwrap, so chain-walk semantics still hold for
    // message-based matchers).
    let source_builtin = match source.0 {
        Some(ErrorKind::Builtin(g)) => Some(g),
        Some(ErrorKind::Custom(a)) => Some(GoError::new(a.Error())),
        None => None,
    };
    match source_builtin {
        Some(inner) => error(Some(ErrorKind::Builtin(GoError {
            msg: msg.to_string(),
            source: Some(Box::new(inner)),
            msg_includes_source: true,
        }))),
        None => New(msg),
    }
}

/// errors.Wrap(err, "context")  →  closest to Go's fmt.Errorf("ctx: %w", err).
/// Returns nil if err is nil (matches Go's typical wrap helpers).
pub fn Wrap(err: error, msg: &str) -> error {
    let inner = match err.0 {
        Some(ErrorKind::Builtin(g)) => g,
        Some(ErrorKind::Custom(a)) => GoError::new(a.Error()),
        None => return nil,
    };
    error(Some(ErrorKind::Builtin(GoError {
        msg: msg.to_string(),
        source: Some(Box::new(inner)),
        msg_includes_source: false,
    })))
}

/// errors.Is(err, target) — walks the wrap chain looking for a match.
///
/// Matching rules mirror Go:
///   - target nil        → true iff err is nil
///   - target Builtin    → walk err's chain, match by message equality
///   - target Custom     → walk err's chain, match when cur == target
///                         (Arc pointer identity or .Unwrap() deep-equal)
pub fn Is(err: &error, target: &error) -> bool {
    if target.0.is_none() { return err.0.is_none(); }
    let mut cur = err.clone();
    loop {
        if cur == *target { return true; }
        // Message match for Builtin targets — Go's `errors.Is` considers
        // a sentinel error equal if the chain contains the same value.
        if let (Some(ErrorKind::Builtin(c)), Some(ErrorKind::Builtin(t))) =
            (&cur.0, &target.0)
        {
            if c.msg == t.msg { return true; }
        }
        let next = Unwrap(cur.clone());
        if next == nil { return false; }
        cur = next;
    }
}

/// errors.Unwrap(err) — returns the next error in the chain, or nil.
pub fn Unwrap(err: error) -> error {
    match err.0 {
        Some(ErrorKind::Builtin(g)) => match g.source {
            Some(src) => error(Some(ErrorKind::Builtin(*src))),
            None => nil,
        },
        Some(ErrorKind::Custom(a)) => a.Unwrap(),
        None => nil,
    }
}

/// errors.Join(errs...) — combine multiple errors into one whose Error()
/// string joins the individual messages with newlines. nil errors are
/// skipped; if the resulting list is empty, returns nil.
pub fn Join(errs: &[error]) -> error {
    let msgs: Vec<String> = errs
        .iter()
        .filter_map(|e| match &e.0 {
            Some(ErrorKind::Builtin(g)) => Some(g.msg.clone()),
            Some(ErrorKind::Custom(a)) => Some(a.Error()),
            None => None,
        })
        .collect();
    if msgs.is_empty() {
        return nil;
    }
    let joined = msgs.join("\n");
    error(Some(ErrorKind::Builtin(GoError::new(joined))))
}

/// errors.Append(err, more) — idiomatic pairwise append, mirroring uber's
/// multierr.Append. Short-circuits on nil so:
///   Append(nil, nil)  → nil
///   Append(err, nil)  → err
///   Append(nil, more) → more
///   otherwise         → Join(&[err, more])
///
/// Chain by re-assigning: `err = errors::Append(err, more);`
pub fn Append(err: error, more: error) -> error {
    if err == nil { return more; }
    if more == nil { return err; }
    Join(&[err, more])
}

/// errors.As(err, target) — if any error in the wrap chain has the same
/// message as target, write it into *target and return true. In Go this is
/// type-based; here we simulate with message-equality since our error type
/// is a single concrete GoError.
pub fn As(err: &error, target: &mut error) -> bool {
    let target_msg = match &target.0 {
        Some(ErrorKind::Builtin(g)) => g.msg.clone(),
        Some(ErrorKind::Custom(a)) => a.Error(),
        None => return false,
    };
    // Walk the builtin chain (custom errors don't expose a source internal).
    let mut cur_opt = match &err.0 {
        Some(ErrorKind::Builtin(g)) => Some(g.clone()),
        Some(ErrorKind::Custom(a)) => Some(GoError::new(a.Error())),
        None => None,
    };
    while let Some(e) = cur_opt {
        if e.msg == target_msg {
            *target = error(Some(ErrorKind::Builtin(e)));
            return true;
        }
        cur_opt = e.source.map(|b| *b);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_macro_error_sentinel() {
        // var!(Name = expr) — default return type `error`.
        crate::var!(ErrMockSentinel = crate::errors::New("mock: short read"));
        let a = ErrMockSentinel();
        let b = ErrMockSentinel();
        assert_eq!(format!("{}", a), "mock: short read");
        assert!(crate::errors::Is(&a, &b));
    }

    #[test]
    fn var_macro_typed() {
        // var!(Name Type = expr) — explicit type.
        crate::var!(TheAnswer i64 = 42i64);
        assert_eq!(TheAnswer(), 42);
        // Same value on every call (cached).
        assert_eq!(TheAnswer(), TheAnswer());
    }

    #[test]
    fn static_err_compat() {
        // Backwards-compat alias still works.
        crate::static_err!(ErrOldStyle = "old style");
        assert_eq!(format!("{}", ErrOldStyle()), "old style");
    }

    #[test]
    fn new_displays_message() {
        let e = New("boom");
        assert_eq!(format!("{}", e), "boom");
    }

    #[test]
    fn nil_displays_as_nil() {
        assert_eq!(format!("{}", nil), "<nil>");
    }

    #[test]
    fn nil_equality() {
        let n: error = nil;
        assert!(n == nil);
        assert!(New("x") != nil);
    }

    #[test]
    fn wrap_chains() {
        let inner = New("disk full");
        let outer = Wrap(inner, "save failed");
        assert_eq!(format!("{}", outer), "save failed: disk full");
    }

    #[test]
    fn wrap_nil_returns_nil() {
        assert!(Wrap(nil, "ctx") == nil);
    }

    #[test]
    fn is_walks_chain() {
        let sentinel = New("not found");
        let wrapped = Wrap(sentinel.clone(), "lookup");
        assert!(Is(&wrapped, &sentinel));
        assert!(!Is(&wrapped, &New("other")));
    }

    #[test]
    fn unwrap_returns_inner_or_nil() {
        let inner = New("inner");
        let outer = Wrap(inner, "outer");
        assert_eq!(format!("{}", Unwrap(outer)), "inner");
        assert!(Unwrap(New("solo")) == nil);
    }

    #[test]
    fn join_combines_messages() {
        let e = Join(&[New("a"), New("b"), nil, New("c")]);
        assert_eq!(format!("{}", e), "a\nb\nc");
    }

    #[test]
    fn join_of_nils_is_nil() {
        assert!(Join(&[nil, nil]) == nil);
        assert!(Join(&[]) == nil);
    }

    #[test]
    fn as_finds_wrapped_sentinel() {
        let sentinel = New("not found");
        let wrapped = Wrap(sentinel.clone(), "lookup");
        let mut target = New("not found");
        assert!(As(&wrapped, &mut target));
        // target written with the matched error
        assert_eq!(format!("{}", target), "not found");

        let mut target = New("other");
        assert!(!As(&wrapped, &mut target));
    }
}
