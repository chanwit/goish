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

use std::fmt::{self, Display};

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct error(Option<GoError>);

impl error {
    pub fn is_nil(&self) -> bool { self.0.is_none() }

    /// e.Error() — message string (panics if nil, matching Go).
    pub fn Error(&self) -> String {
        match &self.0 {
            Some(e) => format!("{}", e),
            None => panic!("runtime error: invalid memory address or nil pointer dereference"),
        }
    }
}

impl Display for error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(e) => Display::fmt(e, f),
            None => write!(f, "<nil>"),
        }
    }
}

/// `nil` — the zero value of `error`.
///
/// Compares equal to any nil error: `if err == nil { ... }`.
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
    error(Some(GoError::new(msg)))
}

/// Internal helper — build an error with a specific source chain. Used
/// by `fmt::Errorf!` when the format string contains `%w`. The `msg`
/// already contains the wrapped error's text (the format scanner
/// substituted `%w` with `.Error()`), so Display should emit only `msg`
/// and not re-concatenate the source.
#[doc(hidden)]
#[allow(non_snake_case)]
pub fn New_with_source(msg: &str, source: error) -> error {
    match source.0 {
        Some(inner) => error(Some(GoError {
            msg: msg.to_string(),
            source: Some(Box::new(inner)),
            msg_includes_source: true,
        })),
        None => New(msg),
    }
}

/// errors.Wrap(err, "context")  →  closest to Go's fmt.Errorf("ctx: %w", err).
/// Returns nil if err is nil (matches Go's typical wrap helpers).
pub fn Wrap(err: error, msg: &str) -> error {
    match err.0 {
        Some(inner) => error(Some(GoError {
            msg: msg.to_string(),
            source: Some(Box::new(inner)),
            msg_includes_source: false,
        })),
        None => nil,
    }
}

/// errors.Is(err, target) — walks the wrap chain looking for a match.
pub fn Is(err: &error, target: &error) -> bool {
    let target_msg = match &target.0 {
        Some(t) => &t.msg,
        None => return err.0.is_none(),
    };
    let mut cur = err.0.as_ref();
    while let Some(e) = cur {
        if &e.msg == target_msg {
            return true;
        }
        cur = e.source.as_deref();
    }
    false
}

/// errors.Unwrap(err) — returns the next error in the chain, or nil.
pub fn Unwrap(err: error) -> error {
    match err.0 {
        Some(e) => match e.source {
            Some(src) => error(Some(*src)),
            None => nil,
        },
        None => nil,
    }
}

/// errors.Join(errs...) — combine multiple errors into one whose Error()
/// string joins the individual messages with newlines. nil errors are
/// skipped; if the resulting list is empty, returns nil.
pub fn Join(errs: &[error]) -> error {
    let msgs: Vec<&String> = errs
        .iter()
        .filter_map(|e| e.0.as_ref().map(|g| &g.msg))
        .collect();
    if msgs.is_empty() {
        return nil;
    }
    let joined: String = msgs
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    error(Some(GoError::new(joined)))
}

/// errors.As(err, target) — if any error in the wrap chain has the same
/// message as target, write it into *target and return true. In Go this is
/// type-based; here we simulate with message-equality since our error type
/// is a single concrete GoError.
pub fn As(err: &error, target: &mut error) -> bool {
    let target_msg = match &target.0 {
        Some(t) => t.msg.clone(),
        None => return false,
    };
    let mut cur = err.0.as_ref();
    while let Some(e) = cur {
        if e.msg == target_msg {
            *target = error(Some(e.clone()));
            return true;
        }
        cur = e.source.as_deref();
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
