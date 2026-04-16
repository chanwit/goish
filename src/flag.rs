// flag: Go's flag package — CLI flag parsing.
//
//   Go                                  goish
//   ─────────────────────────────────   ──────────────────────────────────
//   port := flag.Int("port", 8080, "…")    let port = flag::Int("port", 8080, "…");
//   host := flag.String("host", "…", "")   let host = flag::String("host", "db", "");
//   verbose := flag.Bool("v", false, "")   let verbose = flag::Bool("v", false, "");
//   flag.Parse()                            flag::Parse();
//
//   if *port == 80 { … }                    if port.Get() == 80 { … }
//   rest := flag.Args()                     let rest = flag::Args();
//
// The returned flag.* handles in Go are `*int` etc.; in goish they're
// `Flag<T>` wrappers with `.Get()` / `.Set()` so they can live across
// threads and be re-read. Use `.Get()` anywhere you'd dereference in Go.

use crate::errors::{error, nil, New};
use crate::types::{int, int64, slice, string};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

// ── Flag<T> handle ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Flag<T: Clone> {
    value: Arc<RwLock<T>>,
}

impl<T: Clone> Flag<T> {
    pub fn Get(&self) -> T { self.value.read().unwrap().clone() }
    pub fn Set(&self, v: T) { *self.value.write().unwrap() = v; }
}

// ── Internal flag spec ──────────────────────────────────────────────────

enum Spec {
    String(Flag<string>),
    Int(Flag<int>),
    Int64(Flag<int64>),
    Bool(Flag<bool>),
    Float64(Flag<f64>),
    Duration(Flag<crate::time::Duration>),
}

struct FlagSet {
    specs: Mutex<Vec<(string, string, Spec)>>, // (name, usage, spec)
    args: Mutex<slice<string>>,                // positional args after Parse
    parsed: Mutex<bool>,
}

fn cli() -> &'static FlagSet {
    static SET: OnceLock<FlagSet> = OnceLock::new();
    SET.get_or_init(|| FlagSet {
        specs: Mutex::new(Vec::new()),
        args: Mutex::new(Vec::new()),
        parsed: Mutex::new(false),
    })
}

fn register(name: &str, usage: &str, spec: Spec) {
    cli().specs.lock().unwrap().push((name.into(), usage.into(), spec));
}

// ── Definers ───────────────────────────────────────────────────────────

#[allow(non_snake_case)]
pub fn String(name: &str, default: &str, usage: &str) -> Flag<string> {
    let f = Flag { value: Arc::new(RwLock::new(default.into())) };
    register(name, usage, Spec::String(f.clone()));
    f
}

#[allow(non_snake_case)]
pub fn Int(name: &str, default: int, usage: &str) -> Flag<int> {
    let f = Flag { value: Arc::new(RwLock::new(default)) };
    register(name, usage, Spec::Int(f.clone()));
    f
}

#[allow(non_snake_case)]
pub fn Int64(name: &str, default: int64, usage: &str) -> Flag<int64> {
    let f = Flag { value: Arc::new(RwLock::new(default)) };
    register(name, usage, Spec::Int64(f.clone()));
    f
}

#[allow(non_snake_case)]
pub fn Bool(name: &str, default: bool, usage: &str) -> Flag<bool> {
    let f = Flag { value: Arc::new(RwLock::new(default)) };
    register(name, usage, Spec::Bool(f.clone()));
    f
}

#[allow(non_snake_case)]
pub fn Float64(name: &str, default: f64, usage: &str) -> Flag<f64> {
    let f = Flag { value: Arc::new(RwLock::new(default)) };
    register(name, usage, Spec::Float64(f.clone()));
    f
}

#[allow(non_snake_case)]
pub fn Duration(name: &str, default: crate::time::Duration, usage: &str) -> Flag<crate::time::Duration> {
    let f = Flag { value: Arc::new(RwLock::new(default)) };
    register(name, usage, Spec::Duration(f.clone()));
    f
}

// ── Parse ──────────────────────────────────────────────────────────────

fn find_spec(name: &str) -> Option<Spec> {
    cli().specs.lock().unwrap().iter().find_map(|(n, _, s)| {
        if n == name {
            Some(match s {
                Spec::String(f) => Spec::String(f.clone()),
                Spec::Int(f) => Spec::Int(f.clone()),
                Spec::Int64(f) => Spec::Int64(f.clone()),
                Spec::Bool(f) => Spec::Bool(f.clone()),
                Spec::Float64(f) => Spec::Float64(f.clone()),
                Spec::Duration(f) => Spec::Duration(f.clone()),
            })
        } else { None }
    })
}

fn apply(spec: Spec, val: &str) -> error {
    match spec {
        Spec::String(f) => { f.Set(val.into()); nil }
        Spec::Int(f) => {
            let (n, err) = crate::strconv::Atoi(val);
            if err != nil { return err; }
            f.Set(n);
            nil
        }
        Spec::Int64(f) => {
            let (n, err) = crate::strconv::ParseInt(val, 10, 64);
            if err != nil { return err; }
            f.Set(n);
            nil
        }
        Spec::Bool(f) => {
            let (b, err) = crate::strconv::ParseBool(val);
            if err != nil { return err; }
            f.Set(b);
            nil
        }
        Spec::Float64(f) => {
            let (n, err) = crate::strconv::ParseFloat(val, 64);
            if err != nil { return err; }
            f.Set(n);
            nil
        }
        Spec::Duration(_) => {
            // Simple subset: parse forms like "500ms", "2s", "1h30m".
            match parse_duration(val) {
                Ok(d) => {
                    if let Some(Spec::Duration(f)) = find_spec_by_default(val) {
                        f.Set(d);
                    }
                    nil
                }
                Err(e) => New(&format!("flag: invalid duration {:?}: {}", val, e)),
            }
        }
    }
}

// Dummy to avoid lifetime trick; the Duration branch is simpler:
fn find_spec_by_default(_: &str) -> Option<Spec> { None }

fn parse_duration(s: &str) -> Result<crate::time::Duration, String> {
    // Go's duration parser handles h/m/s/ms/us/ns sequences. Minimal version
    // that handles one or more such units.
    use crate::time::*;
    let mut total = Duration::from_nanos(0);
    let mut rest = s;
    while !rest.is_empty() {
        let num_end = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').count();
        if num_end == 0 { return Err(format!("bad number in {:?}", s)); }
        let (num_s, after_num) = rest.split_at(num_end);
        let unit_end = after_num.chars().take_while(|c| c.is_alphabetic()).count();
        if unit_end == 0 { return Err(format!("missing unit in {:?}", s)); }
        let (unit, after_unit) = after_num.split_at(unit_end);
        let n: f64 = num_s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let unit_dur = match unit {
            "ns" => Nanosecond,
            "us" | "µs" => Microsecond,
            "ms" => Millisecond,
            "s" => Second,
            "m" => Minute,
            "h" => Hour,
            _ => return Err(format!("unknown unit {:?}", unit)),
        };
        total = total + (unit_dur * n);
        rest = after_unit;
    }
    Ok(total)
}

/// flag.Parse() — read os::Args(), assigning values to the flags defined above.
#[allow(non_snake_case)]
pub fn Parse() {
    let args = crate::os::Args();
    ParseArgs(&args[1..]); // skip program name
}

/// flag.ParseArgs(args) — like Parse but explicit. Accepts --name=value,
/// --name value, and --flag for bools.
#[allow(non_snake_case)]
pub fn ParseArgs(args: &[string]) {
    let mut positional: slice<string> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--" {
            positional.extend_from_slice(&args[i + 1..]);
            break;
        }
        let stripped = if let Some(s) = a.strip_prefix("--") {
            Some(s)
        } else if let Some(s) = a.strip_prefix('-') {
            Some(s)
        } else {
            None
        };
        match stripped {
            Some(body) => {
                let (name, inline_val) = if let Some(eq) = body.find('=') {
                    (&body[..eq], Some(&body[eq + 1..]))
                } else {
                    (body, None)
                };
                match find_spec(name) {
                    Some(Spec::Bool(f)) => {
                        let v = match inline_val {
                            Some(v) => {
                                let (b, err) = crate::strconv::ParseBool(v);
                                if err != nil { panic!("flag: bad bool for --{}: {}", name, v); }
                                b
                            }
                            None => true,
                        };
                        f.Set(v);
                        i += 1;
                    }
                    Some(Spec::Duration(f)) => {
                        let v = match inline_val {
                            Some(v) => v.into(),
                            None => {
                                let v = args.get(i + 1).cloned().unwrap_or_default();
                                i += 1;
                                v
                            }
                        };
                        match parse_duration(&v) {
                            Ok(d) => f.Set(d),
                            Err(e) => panic!("flag: invalid duration --{}: {}", name, e),
                        }
                        i += 1;
                    }
                    Some(spec) => {
                        let v = match inline_val {
                            Some(v) => v.into(),
                            None => {
                                let v = args.get(i + 1).cloned().unwrap_or_default();
                                i += 1;
                                v
                            }
                        };
                        let err = apply(spec, &v);
                        if err != nil { panic!("flag: --{}: {}", name, err); }
                        i += 1;
                    }
                    None => {
                        panic!("flag: unknown flag --{}", name);
                    }
                }
            }
            None => {
                positional.push(a.clone());
                i += 1;
            }
        }
    }
    *cli().args.lock().unwrap() = positional;
    *cli().parsed.lock().unwrap() = true;
}

/// flag.Args() — positional arguments left after Parse.
#[allow(non_snake_case)]
pub fn Args() -> slice<string> {
    cli().args.lock().unwrap().clone()
}

/// flag.NArg()
#[allow(non_snake_case)]
pub fn NArg() -> int { Args().len() as int }

/// flag.Arg(i) — returns i-th positional or "".
#[allow(non_snake_case)]
pub fn Arg(i: int) -> string {
    Args().get(i as usize).cloned().unwrap_or_default()
}

/// flag.Parsed()
#[allow(non_snake_case)]
pub fn Parsed() -> bool { *cli().parsed.lock().unwrap() }

#[cfg(test)]
mod tests {
    use super::*;

    // Tests use ParseArgs with explicit slices to avoid touching os::Args,
    // and each test uses distinct flag names to avoid colliding in the
    // global FlagSet (tests share state).

    #[test]
    fn string_int_bool_flags() {
        let host = String("test_host", "db", "hostname");
        let port = Int("test_port", 8080, "port");
        let verbose = Bool("test_verbose", false, "verbose");

        ParseArgs(&["--test_host=db.local".into(),
                    "--test_port".into(), "9090".into(),
                    "--test_verbose".into(),
                    "remaining".into()]);
        assert_eq!(host.Get(), "db.local");
        assert_eq!(port.Get(), 9090);
        assert_eq!(verbose.Get(), true);
        let want: Vec<string> = vec!["remaining".into()];
        assert_eq!(Args(), want);
        assert_eq!(NArg(), 1);
        assert_eq!(Arg(0), "remaining");
    }

    #[test]
    fn duration_flag() {
        let t = Duration("test_timeout", crate::time::Second, "timeout");
        ParseArgs(&["--test_timeout=500ms".into()]);
        assert_eq!(t.Get().Milliseconds(), 500);
    }

    #[test]
    fn short_form_dash() {
        let v = Bool("test_verbose_short", false, "");
        ParseArgs(&["-test_verbose_short".into()]);
        assert!(v.Get());
    }
}
