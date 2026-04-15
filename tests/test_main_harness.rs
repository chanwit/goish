// TestMain — Go's setup/teardown pattern, ported as a `harness = false`
// integration test. Mirrors the shape used by `go/src/os/os_test.go`:
//
//   func TestMain(m *testing.M) {
//       setup()
//       code := m.Run()
//       teardown()
//       os.Exit(code)
//   }
//
// Registered in Cargo.toml as:
//
//   [[test]]
//   name = "test_main_harness"
//   path = "tests/test_main_harness.rs"
//   harness = false
//
// Under harness = false, use `test_h!` (not `test!`) so tests aren't
// hidden by rustc's #[test] attribute. `test_main!` generates the
// entry point and `m.Run()` iterates every registered test.

#![allow(non_snake_case)]

use goish::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static SETUP_RAN: AtomicUsize = AtomicUsize::new(0);
static TEARDOWN_RAN: AtomicUsize = AtomicUsize::new(0);

test_h!{ fn TestAlwaysPasses(t) {
    if SETUP_RAN.load(Ordering::SeqCst) == 0 {
        t.Error("TestMain setup did not run before tests");
    }
    let _ = t;
}}

test_h!{ fn TestStringsContains(t) {
    let s = "hello world".to_owned();
    if !strings::Contains(&s, "world") {
        t.Errorf(Sprintf!("expected 'world' in %q", s));
    }
}}

test_h!{ fn TestArithmetic(t) {
    if 2 + 2 != 4 { t.Fatal("math is broken"); }
    let _ = t;
}}

test_main!{ fn TestMain(m) {
    // Go: shared setup
    SETUP_RAN.store(1, Ordering::SeqCst);
    fmt::Println!("--- TestMain: setup complete");

    let code = m.Run();

    // Go: shared teardown
    TEARDOWN_RAN.store(1, Ordering::SeqCst);
    fmt::Println!("--- TestMain: teardown complete");

    // Go: os.Exit(code) propagates pass/fail to the process.
    os::Exit(code);
}}
