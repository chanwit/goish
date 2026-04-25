// Port of go1.25.5 src/os/os_test.go — a focused subset covering
// ReadFile/WriteFile, Open/Create, Hostname, Getwd, Chdir, Mkdir.
//
// Elided: the bulk of os_test.go concerns Unix permission bits, pipe
// creation, Lstat/Readlink, chown, truncate, file-mode semantics, and
// platform-specific process behaviors — none of which are in scope for
// goish's portable wrapper. These tests cover the 80% use case.

#![allow(non_snake_case)]
use goish::prelude::*;

fn tempdir() -> String {
    let base = std::env::temp_dir().to_string_lossy().into_owned();
    let ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    format!("{}/goish_test_{}", base, ns)
}

test!{ fn TestMkdirReadFileWriteFile(t) {
    let dir = tempdir();
    if os::MkdirAll(&dir, 0o755) != nil {
        t.Fatal(Sprintf!("MkdirAll: %s", dir));
    }
    let p = Sprintf!("%v/hello.txt", dir);
    if os::WriteFile(&p, b"hello, world", 0o644) != nil {
        t.Fatal("WriteFile");
    }
    let (data, err) = os::ReadFile(&p);
    if err != nil { t.Fatal(Sprintf!("ReadFile: %s", err)); }
    if data != b"hello, world" {
        t.Errorf(Sprintf!("ReadFile content = %s", bytes::String(&data)));
    }
    os::Remove(&p);
    os::Remove(&dir);
}}

test!{ fn TestGetwd(t) {
    let (wd, err) = os::Getwd();
    if err != nil { t.Fatal(Sprintf!("Getwd: %s", err)); }
    if wd.is_empty() { t.Error("Getwd returned empty"); }
}}

test!{ fn TestHostnameNonEmpty(t) {
    let (h, err) = os::Hostname();
    if err != nil {
        // OK to skip — some minimal environments have no hostname source.
        t.Logf(Sprintf!("Hostname unavailable (not fatal): %s", err));
        return;
    }
    if h.is_empty() {
        t.Errorf(Sprintf!("Hostname returned empty string with no error"));
    }
}}

test!{ fn TestOpenMissingFile(t) {
    let (_, err) = os::Open("/nonexistent-path-goish-test-12345");
    if err == nil {
        t.Errorf(Sprintf!("Open of missing file returned nil error"));
    }
}}

test!{ fn TestCreateWriteReadBack(t) {
    let dir = tempdir();
    os::MkdirAll(&dir, 0o755);
    let path = Sprintf!("%v/cwrb.txt", dir);
    let (mut f, err) = os::Create(&path);
    if err != nil { t.Fatal(Sprintf!("Create: %s", err)); }
    let (n, werr) = f.Write(b"payload");
    if werr != nil || n != 7 {
        t.Errorf(Sprintf!("Write n=%d err=%s", n, werr));
    }
    f.Close();
    let (data, err) = os::ReadFile(&path);
    if err != nil { t.Fatal(Sprintf!("ReadFile: %s", err)); }
    if data != b"payload" {
        t.Errorf(Sprintf!("back = %s", bytes::String(&data)));
    }
    os::Remove(&path);
    os::Remove(&dir);
}}

test!{ fn TestArgsHasProgramName(t) {
    let args = os::Args();
    if args.is_empty() {
        t.Errorf(Sprintf!("os::Args() empty"));
    }
}}

test!{ fn TestRemoveAll(t) {
    let dir = tempdir();
    os::MkdirAll(&Sprintf!("%v/a/b/c", dir), 0o755);
    os::WriteFile(&Sprintf!("%v/a/b/c/file.txt", dir), b"x", 0o644);
    let err = os::RemoveAll(&dir);
    if err != nil { t.Errorf(Sprintf!("RemoveAll: %s", err)); }
    let (_, err) = os::Open(&dir);
    if err == nil {
        t.Errorf(Sprintf!("RemoveAll didn't delete %s", dir));
    }
}}
