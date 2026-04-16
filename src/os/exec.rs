// exec: Go's os/exec package — spawn subprocesses.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   cmd := exec.Command("echo", "hi")   let mut cmd = exec::Command("echo", &["hi"]);
//   out, err := cmd.Output()            let (out, err) = cmd.Output();
//   err := cmd.Run()                    let err = cmd.Run();
//   out, err := cmd.CombinedOutput()    let (out, err) = cmd.CombinedOutput();
//
// `Output()` returns stdout bytes; `CombinedOutput()` merges stdout + stderr.
// For custom stdin / stdout piping, use `.Stdin(...)`, `.Stdout(...)`, etc.
//
// The command inherits the parent process environment by default. Use
// `.Env(&[("KEY","value")])` to replace the environment.

use crate::errors::{error, nil, New};
use crate::types::{byte, int, slice, string};
use std::process::{Command as RustCmd, Stdio};

pub struct Cmd {
    pub Path: string,
    pub Args: slice<string>,
    pub Env: Option<slice<string>>,
    pub Dir: Option<string>,
    stdin: Option<Vec<byte>>,
    /// Exit code of the last run, or -1 if not set.
    pub ProcessState: Option<ProcessState>,
}

#[derive(Debug, Clone)]
pub struct ProcessState {
    pub ExitCode: int,
    pub Success: bool,
}

impl Cmd {
    pub fn Run(&mut self) -> error {
        match self.spawn_and_wait(None, None) {
            Ok(_) => nil,
            Err(e) => e,
        }
    }

    pub fn Output(&mut self) -> (Vec<byte>, error) {
        let mut c = self.build();
        c.stdout(Stdio::piped());
        c.stderr(Stdio::piped());
        if self.stdin.is_some() { c.stdin(Stdio::piped()); }
        match c.spawn() {
            Ok(mut child) => {
                if let Some(data) = &self.stdin {
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        let _ = stdin.write_all(data);
                        drop(stdin);
                    }
                }
                match child.wait_with_output() {
                    Ok(o) => {
                        self.ProcessState = Some(ProcessState {
                            ExitCode: o.status.code().unwrap_or(-1) as int,
                            Success: o.status.success(),
                        });
                        if o.status.success() {
                            (o.stdout, nil)
                        } else {
                            (
                                o.stdout,
                                New(&format!(
                                    "exit status {}",
                                    o.status.code().unwrap_or(-1)
                                )),
                            )
                        }
                    }
                    Err(e) => (Vec::new(), New(&e.to_string())),
                }
            }
            Err(e) => (Vec::new(), New(&e.to_string())),
        }
    }

    pub fn CombinedOutput(&mut self) -> (Vec<byte>, error) {
        use std::io::Read;
        let mut c = self.build();
        c.stdout(Stdio::piped());
        c.stderr(Stdio::piped());
        if self.stdin.is_some() { c.stdin(Stdio::piped()); }
        match c.spawn() {
            Ok(mut child) => {
                if let Some(data) = &self.stdin {
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        let _ = stdin.write_all(data);
                        drop(stdin);
                    }
                }
                let mut out = Vec::new();
                if let Some(mut so) = child.stdout.take() {
                    let _ = so.read_to_end(&mut out);
                }
                if let Some(mut se) = child.stderr.take() {
                    let _ = se.read_to_end(&mut out);
                }
                match child.wait() {
                    Ok(status) => {
                        self.ProcessState = Some(ProcessState {
                            ExitCode: status.code().unwrap_or(-1) as int,
                            Success: status.success(),
                        });
                        if status.success() {
                            (out, nil)
                        } else {
                            (
                                out,
                                New(&format!(
                                    "exit status {}",
                                    status.code().unwrap_or(-1)
                                )),
                            )
                        }
                    }
                    Err(e) => (out, New(&e.to_string())),
                }
            }
            Err(e) => (Vec::new(), New(&e.to_string())),
        }
    }

    /// Set stdin bytes to be fed to the child on Output/CombinedOutput/Run.
    pub fn SetStdin(&mut self, data: &[byte]) {
        self.stdin = Some(data.to_vec());
    }

    fn build(&self) -> RustCmd {
        let mut c = RustCmd::new(self.Path.as_str());
        if self.Args.len() > 1 {
            let args: Vec<&str> = self.Args[1..].iter().map(|s| s.as_str()).collect();
            c.args(&args);
        }
        if let Some(d) = &self.Dir {
            c.current_dir(d.as_str());
        }
        if let Some(env) = &self.Env {
            c.env_clear();
            for s in env {
                if let Some((k, v)) = s.split_once('=') {
                    c.env(k, v);
                }
            }
        }
        c
    }

    fn spawn_and_wait(&mut self, _stdin: Option<Vec<byte>>, _stdout: Option<Vec<byte>>) -> Result<(), error> {
        let mut c = self.build();
        if self.stdin.is_some() { c.stdin(Stdio::piped()); }
        match c.spawn() {
            Ok(mut child) => {
                if let Some(data) = &self.stdin {
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        let _ = stdin.write_all(data);
                        drop(stdin);
                    }
                }
                match child.wait() {
                    Ok(status) => {
                        self.ProcessState = Some(ProcessState {
                            ExitCode: status.code().unwrap_or(-1) as int,
                            Success: status.success(),
                        });
                        if status.success() {
                            Ok(())
                        } else {
                            Err(New(&format!(
                                "exit status {}",
                                status.code().unwrap_or(-1)
                            )))
                        }
                    }
                    Err(e) => Err(New(&e.to_string())),
                }
            }
            Err(e) => Err(New(&e.to_string())),
        }
    }
}

/// exec.Command(name, args...) — construct a Cmd but do not run it.
///
/// Usage differs slightly from Go: the args come as a slice rather than a
/// varargs tail, which fits Rust macro ergonomics better.
#[allow(non_snake_case)]
pub fn Command(name: impl AsRef<str>, args: &[impl AsRef<str>]) -> Cmd {
    let name: string = name.as_ref().into();
    let mut all: slice<string> = slice::with_capacity(args.len() + 1);
    all.push(name.clone());
    for a in args {
        all.push(a.as_ref().into());
    }
    Cmd {
        Path: name,
        Args: all,
        Env: None,
        Dir: None,
        stdin: None,
        ProcessState: None,
    }
}

/// exec.LookPath(name) — find an executable in PATH. Returns the full path.
#[allow(non_snake_case)]
pub fn LookPath(name: impl AsRef<str>) -> (string, error) {
    let name = name.as_ref();
    // If already absolute or relative, use as-is if it exists.
    if name.contains('/') || name.contains('\\') {
        if std::path::Path::new(name).exists() {
            return (name.into(), nil);
        }
        return ("".into(), New(&format!("exec: \"{}\": file does not exist", name)));
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return (candidate.to_string_lossy().into_owned().into(), nil);
        }
    }
    ("".into(), New(&format!("exec: \"{}\": executable file not found in $PATH", name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo() -> &'static [&'static str] {
        if cfg!(unix) { &["/bin/echo"] } else { &["cmd", "/C", "echo"] }
    }

    #[test]
    fn output_captures_stdout() {
        let (path, _) = LookPath("echo");
        if path.is_empty() { return; }
        let mut cmd = Command(&path, &["hello"]);
        let (out, err) = cmd.Output();
        assert_eq!(err, nil);
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("hello"));
    }

    #[test]
    fn run_ok_for_true_binary() {
        let (path, _) = LookPath("true");
        if path.is_empty() { return; }
        let mut cmd = Command(&path, &[] as &[&str]);
        let err = cmd.Run();
        assert_eq!(err, nil);
        assert!(cmd.ProcessState.as_ref().unwrap().Success);
    }

    #[test]
    fn run_err_for_false_binary() {
        let (path, _) = LookPath("false");
        if path.is_empty() { return; }
        let mut cmd = Command(&path, &[] as &[&str]);
        let err = cmd.Run();
        assert!(err != nil);
        assert!(!cmd.ProcessState.as_ref().unwrap().Success);
    }

    #[test]
    fn combined_output_merges_streams() {
        if !cfg!(unix) { return; }
        let mut cmd = Command("/bin/sh", &["-c", "echo out; echo err 1>&2"]);
        let (out, _err) = cmd.CombinedOutput();
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("out"));
        assert!(s.contains("err"));
    }

    #[test]
    fn lookpath_finds_echo() {
        let (p, err) = LookPath("echo");
        assert_eq!(err, nil);
        assert!(!p.is_empty());
    }

    #[test]
    fn lookpath_missing_returns_error() {
        let (_, err) = LookPath("definitely_not_a_real_command_xyz");
        assert!(err != nil);
    }

    #[test]
    fn stdin_is_passed() {
        if !cfg!(unix) { return; }
        let mut cmd = Command("/bin/cat", &[] as &[&str]);
        cmd.SetStdin(b"piped data");
        let (out, err) = cmd.Output();
        assert_eq!(err, nil);
        assert_eq!(String::from_utf8_lossy(&out), "piped data");
    }

    #[test]
    fn echo_via_helper() {
        // smoke test against the platform-appropriate echo binary
        let cmd_parts = echo();
        let mut c = Command(cmd_parts[0], &cmd_parts[1..].iter().chain(["hi"].iter()).copied().collect::<Vec<_>>());
        let _ = c.Output();
    }
}
