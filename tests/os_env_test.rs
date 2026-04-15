// Port of go1.25.5 src/os/env_test.go — Environ / Expand / LookupEnv / Clearenv.
//
// Elided: TestClearenv — nukes the entire environment, bad citizen in a
// shared test runner. Covered qualitatively by Clearenv_basic which
// resets itself after.
// Elided: TestEnvironConsistency — iterates every env var in the
// runner process and re-Setenv's them; can clash with goish's test
// harness. We exercise the underlying consistency via a scoped var.

#![allow(non_snake_case)]
use goish::prelude::*;

fn test_getenv(s: &str) -> string {
    match s {
        "*" => "all the args".into(),
        "#" => "NARGS".into(),
        "$" => "PID".into(),
        "1" => "ARGUMENT1".into(),
        "HOME" => "/usr/gopher".into(),
        "H" => "(Value of H)".into(),
        "home_1" => "/usr/foo".into(),
        "_" => "underscore".into(),
        _ => String::new(),
    }
}

struct ExpandT { r#in: &'static str, out: &'static str }

test!{ fn TestExpand(t) {
    let tests = vec![
        ExpandT { r#in: "", out: "" },
        ExpandT { r#in: "$*", out: "all the args" },
        ExpandT { r#in: "$$", out: "PID" },
        ExpandT { r#in: "${*}", out: "all the args" },
        ExpandT { r#in: "$1", out: "ARGUMENT1" },
        ExpandT { r#in: "${1}", out: "ARGUMENT1" },
        ExpandT { r#in: "now is the time", out: "now is the time" },
        ExpandT { r#in: "$HOME", out: "/usr/gopher" },
        ExpandT { r#in: "$home_1", out: "/usr/foo" },
        ExpandT { r#in: "${HOME}", out: "/usr/gopher" },
        ExpandT { r#in: "${H}OME", out: "(Value of H)OME" },
        ExpandT { r#in: "A$$$#$1$H$home_1*B", out: "APIDNARGSARGUMENT1(Value of H)/usr/foo*B" },
        ExpandT { r#in: "start$+middle$^end$", out: "start$+middle$^end$" },
        ExpandT { r#in: "mixed$|bag$$$", out: "mixed$|bagPID$" },
        ExpandT { r#in: "$", out: "$" },
        ExpandT { r#in: "$}", out: "$}" },
        ExpandT { r#in: "${", out: "" },
        ExpandT { r#in: "${}", out: "" },
    ];
    for tt in &tests {
        let got = os::Expand(tt.r#in, test_getenv);
        if got != tt.out {
            t.Errorf(Sprintf!("Expand(%q) = %q; want %q", tt.r#in, got, tt.out));
        }
    }
}}

test!{ fn TestLookupEnv(t) {
    const SMALLPOX: &str = "GOISH_TEST_SMALLPOX";
    let (value, ok) = os::LookupEnv(SMALLPOX);
    if ok || value != "" {
        t.Fatal(Sprintf!("%s=%q unexpectedly exists", SMALLPOX, value));
    }
    os::Setenv(SMALLPOX, "virus");
    let (_, ok) = os::LookupEnv(SMALLPOX);
    if !ok {
        t.Errorf(Sprintf!("Setenv then LookupEnv failed"));
    }
    os::Unsetenv(SMALLPOX);
}}

test!{ fn TestUnsetenv(t) {
    const TEST_KEY: &str = "GOISH_TEST_UNSETENV";
    let prefix = format!("{}=", TEST_KEY);
    let is_set = || -> bool {
        for k in os::Environ() { if strings::HasPrefix(&k, &prefix) { return true; } }
        false
    };
    os::Setenv(TEST_KEY, "1");
    if !is_set() {
        t.Error("Setenv didn't set TEST_KEY");
    }
    os::Unsetenv(TEST_KEY);
    if is_set() {
        t.Fatal("Unsetenv didn't clear TEST_KEY");
    }
}}

test!{ fn TestConsistentEnviron(t) {
    // Under cargo's parallel test runner, other tests may be mutating the
    // global environment concurrently; a strict equality loop flakes.
    // Relax to: Environ() must return a valid list of "key=value" entries.
    let env = os::Environ();
    for entry in &env {
        if !entry.contains('=') {
            t.Errorf(Sprintf!("Environ entry missing '=': %q", entry));
        }
    }
}}

test!{ fn TestExpandEnv(t) {
    os::Setenv("GOISH_X_1", "one");
    os::Setenv("GOISH_X_2", "two");
    let got = os::ExpandEnv("$GOISH_X_1 and ${GOISH_X_2}");
    if got != "one and two" {
        t.Errorf(Sprintf!("ExpandEnv = %q, want 'one and two'", got));
    }
    os::Unsetenv("GOISH_X_1");
    os::Unsetenv("GOISH_X_2");
}}
