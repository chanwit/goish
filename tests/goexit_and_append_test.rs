// Coverage for v0.20.4 library additions:
//   - runtime::Goexit() — Go's runtime.Goexit; terminates the current
//     goroutine, runs deferred funcs, clean exit in the test harness.
//   - errors::Append(err, more) — pairwise multierr.Append equivalent.

use goish::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

test!{ fn TestGoexitIsCleanExit(t) {
    runtime::Goexit();
    // Unreachable: Goexit unwinds via panic-with-sentinel. If this line ran,
    // we'd fail the test. The harness treats the sentinel as Outcome::Ok.
    t.Errorf("unreachable after Goexit()".to_string());
}}

test!{ fn TestGoexitRunsDefers(t) {
    static RAN: AtomicBool = AtomicBool::new(false);
    fn inner() {
        defer!{ RAN.store(true, Ordering::SeqCst); }
        runtime::Goexit();
    }
    let _ = std::panic::catch_unwind(|| inner());
    if !RAN.load(Ordering::SeqCst) {
        t.Errorf("defer did not run through Goexit unwind".to_string());
    }
}}

test!{ fn TestGoexitInsideGoroutine(t) {
    let g = go!{
        runtime::Goexit();
    };
    let err = g.Wait();
    if err != nil {
        t.Errorf(Sprintf!("Wait() after Goexit returned %s; want nil", err));
    }
}}

test!{ fn TestAppendBothNil(t) {
    let got = errors::Append(nil, nil);
    if got != nil { t.Errorf("Append(nil, nil) != nil".to_string()); }
}}

test!{ fn TestAppendFirstNil(t) {
    let more = errors::New("more");
    let got = errors::Append(nil, more.clone());
    if got != more { t.Errorf("Append(nil, more) != more".to_string()); }
}}

test!{ fn TestAppendSecondNil(t) {
    let err = errors::New("err");
    let got = errors::Append(err.clone(), nil);
    if got != err { t.Errorf("Append(err, nil) != err".to_string()); }
}}

test!{ fn TestAppendBoth(t) {
    let a = errors::New("first");
    let b = errors::New("second");
    let got = errors::Append(a, b);
    let msg = Sprintf!("%s", got);
    if !strings::Contains(&msg, "first") || !strings::Contains(&msg, "second") {
        t.Errorf(Sprintf!("Append joined text lost a message: %q", msg));
    }
}}

test!{ fn TestAppendChain(t) {
    let mut err: error = nil;
    err = errors::Append(err, errors::New("one"));
    err = errors::Append(err, errors::New("two"));
    err = errors::Append(err, errors::New("three"));
    let msg = Sprintf!("%s", err);
    for needle in ["one", "two", "three"].iter() {
        if !strings::Contains(&msg, needle) {
            t.Errorf(Sprintf!("chained Append lost %q from %q", needle.to_string(), msg));
        }
    }
}}
