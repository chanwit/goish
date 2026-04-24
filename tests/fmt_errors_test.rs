// Port of go1.25.5/src/fmt/errors_test.go — fmt.Errorf with %w wrapping.
// We port the single-%w subset (Go ≤ 1.19 semantics).
// Multi-%w (Go 1.20+ Unwrap() []error) is tracked separately — see
// goish #42 follow-up work in v0.7+.

#![allow(non_snake_case)]
use goish::prelude::*;

test!{ fn TestErrorf(t) {
    let wrapped = errors::New("inner error");

    struct Case<'a> {
        err: error,
        want_text: &'a str,
        want_unwrap: error,
    }

    let cases = vec![
        Case {
            err: Errorf!("%w", wrapped.clone()),
            want_text: "inner error",
            want_unwrap: wrapped.clone(),
        },
        Case {
            err: Errorf!("added context: %w", wrapped.clone()),
            want_text: "added context: inner error",
            want_unwrap: wrapped.clone(),
        },
        Case {
            err: Errorf!("%w with added context", wrapped.clone()),
            want_text: "inner error with added context",
            want_unwrap: wrapped.clone(),
        },
        Case {
            err: Errorf!("%s %w %v", "prefix", wrapped.clone(), "suffix"),
            want_text: "prefix inner error suffix",
            want_unwrap: wrapped.clone(),
        },
        Case {
            err: Errorf!("%v", wrapped.clone()),
            want_text: "inner error",
            want_unwrap: nil,
        },
        Case {
            err: Errorf!("added context: %v", wrapped.clone()),
            want_text: "added context: inner error",
            want_unwrap: nil,
        },
        Case {
            err: Errorf!("%v with added context", wrapped.clone()),
            want_text: "inner error with added context",
            want_unwrap: nil,
        },
        Case {
            err: Errorf!("%w is not an error", "not-an-error"),
            want_text: "%!w(string=not-an-error) is not an error",
            want_unwrap: nil,
        },
    ];

    for c in cases {
        let got_text = Sprintf!("%v", c.err);
        if got_text != c.want_text {
            t.Errorf(Sprintf!("err.Error() = %q, want %q", got_text, c.want_text));
        }
        let got_unwrap = errors::Unwrap(c.err.clone());
        if got_unwrap != c.want_unwrap {
            t.Errorf(Sprintf!("errors.Unwrap() = %v, want %v", got_unwrap, c.want_unwrap));
        }
    }
}}
