# Goish Rust

> **Write Rust using Go idioms.**

Goish Rust is a Rust crate that ports Go's standard library and built-in
syntax to Rust so Go programmers can write Rust code that reads — and
*feels* — like Go. Safety, ownership, and zero-cost abstractions come
for free; the call sites stay familiar.

```rust
use goish::prelude::*;

fn divide(a: int64, b: int64) -> (int64, error) {
    if b == 0 {
        return (0, errors::New("divide by zero"));
    }
    (a / b, nil)
}
```

## Install

```toml
[dependencies]
goish = "0.15"
```

Then:

```rust
use goish::prelude::*;
```

## Three things Goish Rust does well

### 1. Goroutines and channels

Tokio-backed `go!{}` spawns a goroutine as a ~200 B async task.
`Chan<T>` rendezvous and buffered channels look synchronous at the
call site. **Proven at 1,000,000 concurrent goroutines**
(`tests/million_goroutines.rs`).

```rust
use goish::prelude::*;

fn main() {
    let jobs:    Chan<int> = chan!(int, 4);
    let results: Chan<int> = chan!(int, 4);

    for w in 1..=3 {
        let j = jobs.clone();
        let r = results.clone();
        go!{
            for (job, _) in std::iter::from_fn(|| Some(j.Recv())).take(3) {
                r.Send(job * 2);
                fmt::Printf!("worker %d did job %d\n", w, job);
            }
        };
    }

    for n in 1..=3 { jobs.Send(n); }
    for _ in 0..3  { let (v, _) = results.Recv(); fmt::Printf!("got %d\n", v); }
}
```

### 2. net/http server + client

Backed by hyper 1.x + tokio, with Go's handler signature.

```rust
use goish::prelude::*;

fn main() {
    http::HandleFunc("/hello", |w, r| {
        Fprintf!(w, "hi %s", r.URL.Path);
    });
    log::Fatalf!("%s", http::ListenAndServe(":8080", nil));
}
```

Client-side, with context-bound deadlines that cancel in-flight
requests:

```rust
let (ctx, _) = context::WithTimeout(context::Background(), 5i64 * time::Second);
let (req, _) = http::NewRequestWithContext(ctx, "GET", url, &[]);
let (resp, err) = http::Do(req);
```

### 3. Testing — line-by-line Go test ports

`test!` registers a test; `Struct!` declares a Go-style table entry
type; `range!` + Rust's `for ... in` produce a Go-shape
`for i, v := range` loop. The result reads like a direct port of
Go's `*_test.go`.

```rust
use goish::prelude::*;

Struct!{ type Case struct { input: string, want: int64 } }

fn cases() -> slice<Case> { slice!([]Case{
    Case!("42",   42),
    Case!("-7",   -7),
    Case!("0",     0),
})}

test!{ fn TestAtoi(t) {
    for (i, c) in range!(cases()) {
        let (got, err) = strconv::Atoi(&c.input);
        if err != nil {
            t.Errorf(Sprintf!("case %d: %s", i as i64, err));
            continue;
        }
        if got != c.want {
            t.Errorf(Sprintf!("case %d: Atoi(%s) = %d, want %d",
                i as i64, c.input, got, c.want));
        }
    }
}}
```

Real Go test files ported as regression fixtures live in `tests/` —
e.g. `path_test.rs`, `strings_strings_test.rs`, `net_netip_test.rs`.
**934 tests pass**.

## Documentation

- **[COOKBOOK.md](COOKBOOK.md)** — Go → Goish Rust translation reference
  covering every ported package (types, errors, fmt, channels, http,
  testing, and 40+ others).
- **[PROGRESS.md](PROGRESS.md)** — per-package port coverage vs
  Go 1.25.5.
- **[docs/scheduler.md](docs/scheduler.md)** — runtime decision
  (tokio + flume, 1M goroutines).

## Design priority

The top priority in Goish Rust is **Go idioms and call-site syntax**.
If a Go programmer reads a Goish Rust program, it should look like
Go. Rust idioms (`Option`, `Some`, `&mut`, trait-bound generics) live
*under* the hood. When a Rust idiom has to leak, it is called out
explicitly and a wrapper is designed to minimise it.

## License

Dual-licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

at your option.
