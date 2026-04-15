# goish

**Write Rust using Go idioms.**

`goish` is a Rust crate that ports Go's standard library and built-in syntax
constructs into Rust so that Go programmers can write Rust code that reads and
*feels* like Go — `(val, err) := fn()`, `if err != nil`, `defer f.Close()`,
`go func() { ... }()`, `for i, v := range slice`, `[]string{"a","b"}` — while
still getting Rust's safety and performance under the hood.

```rust
use goish::prelude::*;

fn divide(a: int64, b: int64) -> (int64, error) {
    if b == 0 {
        return (0, errors::New("divide by zero"));
    }
    (a / b, nil)
}

fn main() {
    fmt::Println!("hello", "world", 42);

    let (q, err) = divide(10, 0);
    if err != nil {
        fmt::Println!("error:", err);
    } else {
        fmt::Printf!("q = %d\n", q);
    }

    let names = slice!([]string{"alice", "bob", "carol"});
    let env   = map!([string]int{"PORT" => 5432, "TTL" => 60});

    fmt::Printf!("%d names, %d env vars\n", len!(names), len!(env));

    defer!{ fmt::Println!("bye!"); }
}
```

## Install

Add to `Cargo.toml`:

```toml
[dependencies]
goish = { git = "https://github.com/chanwit/goish" }
```

Or, from crates.io:

```toml
[dependencies]
goish = "0.4"      # 0.4.1 = latest
```

Then in every file where you want Go-shaped code:

```rust
use goish::prelude::*;
```

## Cheat-sheet

### Types

| Go | goish |
|---|---|
| `int` / `int64` / `float64` / `byte` / `rune` | `int` / `int64` / `float64` / `byte` / `rune` |
| `string` | `string`  *(alias of `std::string::String`)* |
| `[]T` | `slice<T>`  *(alias of `Vec<T>`)* |
| `map[K]V` | `map<K, V>`  *(alias of `HashMap<K, V>`)* |
| `chan T` | `Chan<T>` |
| `error` | `error`  *(a newtype; `nil` is a zero-value constant)* |

### Composite literals

| Go | goish |
|---|---|
| `[]string{"a","b","c"}` | `slice!([]string{"a","b","c"})` |
| `[]int{1,2,3}` | `slice!([]int{1,2,3})` |
| `map[string]int{"a":1,"b":2}` | `map!([string]int{"a" => 1, "b" => 2})` |
| `make([]int, 5)` | `make!([]int, 5)` |
| `make([]T, 0, n)` | `make!([]T, 0, n)` |
| `make(map[K]V)` | `make!(map[K]V)` |
| `make(chan int, 10)` | `make!(chan int, 10)` or `chan!(int, 10)` |

### Builtins

| Go | goish |
|---|---|
| `len(x)` | `len!(x)` |
| `append(s, x, y)` | `append!(s, x, y)` |
| `delete(m, k)` | `delete!(m, k)` *(owned-key variables: `delete!(m, &k)`)* |
| `for i, v := range s` | `range!(s, \|i, v\| { ... })` |

### Errors

```rust
let err = errors::New("not found");
let wrapped = errors::Wrap(err.clone(), "lookup failed");
if errors::Is(&wrapped, &err) { ... }
```

`errors::New` returns `error` directly — no `Some`/`Ok` wrapping. `err != nil`
works because `error` implements `PartialEq<error>` with the `nil` constant.

### fmt and Stringer

```rust
fmt::Println!("n =", 42);
let s = fmt::Sprintf!("%-8s = %d", "answer", 42);
let e = fmt::Errorf!("code %d: %s", 500, "oops");

struct Color { r: int, g: int, b: int }
fmt::stringer! {
    impl Color {
        fn String(&self) -> string {
            fmt::Sprintf!("#%02x%02x%02x", self.r, self.g, self.b)
        }
    }
}
fmt::Println!("color:", Color { r: 255, g: 0, b: 0 }); // → color: #ff0000
```

### Goroutines and channels

```rust
let jobs: Chan<int> = chan!(int, 4);
let g = go!{
    for i in 1..=3 {
        jobs.Send(i);
    }
};
for _ in 0..3 {
    let (v, _) = jobs.Recv();
    fmt::Printf!("got: %d\n", v);
}
let _ = g.Wait();
```

### `defer!`

```rust
fn foo() {
    defer!{ fmt::Println!("cleanup"); }
    defer!{ fmt::Println!("inner"); }
    fmt::Println!("work");
}
// prints: work, inner, cleanup   (LIFO, same as Go)
```

### sync

```rust
let mu = sync::Mutex::new(0i64);
{ let mut g = mu.Lock(); *g += 1; }    // auto-unlock at scope end

let wg = sync::WaitGroup::new();
for _ in 0..4 {
    wg.Add(1);
    let w = wg.clone();
    go!{ do_work(); w.Done(); };
}
wg.Wait();
```

### time

```rust
let start = time::Now();
time::Sleep(time::Millisecond * 100i64);
let elapsed = time::Since(start);
fmt::Printf!("took %s\n", elapsed);   // e.g. "100.3ms"
```

### testing (v0.4) — port Go tests line-by-line

```rust
use goish::prelude::*;

// Go:   type PathTest struct { path, result string }
Struct!{ type PathTest struct { path, result string } }

// Go:   var tests = []PathTest{ {"", "."}, {"abc", "abc"}, ... }
fn tests() -> slice<PathTest> { slice!([]PathTest{
    PathTest!("",    "."),
    PathTest!("abc", "abc"),
    // ...
})}

test!{ fn TestClean(t) {
    for test in &tests() {
        let s = path::Clean(&test.path);
        if s != test.result {
            t.Errorf(Sprintf!("Clean(%q) = %q, want %q", test.path, s, test.result));
        }
    }
}}

benchmark!{ fn BenchmarkJoin(b) {
    b.ReportAllocs();
    while b.Loop() {
        path::Join(&slice!([]string{"a", "b"}));
    }
}}
```

Real Go tests ported as regression fixtures live in `tests/`:
- `tests/path_test.rs` — direct port of `go/src/path/path_test.go`
- `tests/itoa_test.rs` — direct port of `go/src/strconv/itoa_test.go`
- `tests/strings_compare_test.rs` — port of `go/src/strings/compare_test.go`
- `tests/bytes_test.rs` — subset of `go/src/bytes/bytes_test.go` (Equal/Index/LastIndex/IndexByte)

**Go's `defer-recover`** ports as `recover!{ … }`:

```rust
// Go:   defer func() { if r := recover(); r == nil { t.Fatal("want panic") } }()
//       doPanickyThing()
let r = recover!{ strconv::FormatUint(12345678, 1) };
if r.is_none() { t.Fatal("expected panic"); }
```

## Packages (current status)

| goish | Go package | status |
|---|---|---|
| `fmt` | `fmt` | Println/Printf/Sprintf/Fprintf/Errorf/Stringer |
| `errors` | `errors` | New/Wrap/Is/Unwrap + **Join/As** |
| `bytes` | `bytes` | `Buffer` |
| `strings` | `strings` | 20+ helpers + **`Builder`** |
| `strconv` | `strconv` | Atoi/Itoa/ParseInt/ParseFloat/ParseBool/Format* |
| `time` | `time` | Now/Since/Sleep + Duration arithmetic + **Format/Parse/Date + Ticker/Timer/AfterFunc** |
| `os` | `os` | Args/Getenv/Exit + ReadFile/WriteFile/Mkdir + Hostname/Getwd + **File/Open/Create + Stdin/Stdout/Stderr** |
| `io` | `io` | Reader/Writer traits, Copy, ReadAll, EOF |
| `bufio` | `bufio` | `Scanner` + **`NewReader`/`NewWriter`** |
| `log` | `log` | Println/Printf/Fatalf/Panic with timestamp |
| `sort` | `sort` | Ints/Strings/Float64s/Slice/SliceStable |
| `math` | `math` | constants + Abs/Pow/Sqrt/trig/log/IsNaN/IsInf |
| `filepath` | `path/filepath` | Join/Base/Dir/Ext/Clean |
| `sync` | `sync` | Mutex/RWMutex/WaitGroup/Once + **`atomic::{Int32,Int64,Bool}`** |
| `context` | `context` | Background/WithCancel/WithTimeout + **WithValue/WithDeadline** |
| `chan` | channel builtins | `Chan<T>` + `chan!(T[, n])` (**see gaps**) |
| `defer!` / `go!` | defer / go keywords | macro form |
| **`unicode`** | `unicode` | IsLetter/IsDigit/IsSpace/IsUpper/IsLower/ToUpper/ToLower |
| **`utf8`** | `unicode/utf8` | RuneCountInString/ValidString/DecodeRuneInString/EncodeRune/RuneLen |
| **`rand`** | `math/rand` | Int/Intn/Int63/Float64/Seed/Shuffle (xoshiro256**, no deps) |
| **`base64`** | `encoding/base64` | StdEncoding/URLEncoding/RawStdEncoding |
| **`hex`** | `encoding/hex` | EncodeToString/DecodeString |
| **`flag`** | `flag` | String/Int/Bool/Float64/Duration + Parse/Args/Arg |
| **`const_block!`** | `const (…) with iota` | auto-incrementing constants |
| **`bytes`** | `bytes` *(complete)* | `Buffer`/`Reader`/`NewReader` + Equal/Index/Split/Join/Trim/Replace/Fields/…  |
| **`strings`** | `strings` *(extended)* | + `Reader`/`Replacer`/`Map` + IndexAny/ContainsAny/Title |
| **`path`** | `path` | slash-only Base/Dir/Ext/Join/Clean/Split/Match (URLs) |
| **`sort`** | `sort` *(extended)* | + Search/SearchInts/SearchStrings/IntSlice/Reverse/ReverseInts |
| **`runtime`** | `runtime` | NumCPU/GOMAXPROCS/Gosched/GOOS/GOARCH/NumGoroutine/Version |
| **`exec`** | `os/exec` | Command/Run/Output/CombinedOutput/LookPath |
| **`binary`** | `encoding/binary` | BigEndian/LittleEndian + Uvarint/Varint |
| **`csv`** | `encoding/csv` | Reader/Writer with RFC 4180 quoting |
| **`hash::{crc32,fnv}`** | `hash/crc32`, `hash/fnv` | IEEE CRC-32, FNV-1/1a 32/64 |
| **`mime`** | `mime` | TypeByExtension/ExtensionsByType/AddExtensionType |
| **`container::{list,heap}`** | `container/list`, `container/heap` | doubly linked list, generic binary heap |
| **`url`** | `net/url` | Parse/URL/Userinfo + Values + QueryEscape/PathEscape |
| **`crypto::{md5,sha1,sha256}`** | `crypto/md5`, `crypto/sha1`, `crypto/sha256` | hand-rolled (no deps) |
| **`json`** | `encoding/json` | Marshal/Unmarshal/MarshalIndent over `Value` |
| **`regexp`** | `regexp` | Compile/MustCompile/MatchString/FindString/ReplaceAll/Split |
| **`testing`** | `testing` | `T`/`M`/`B` + `test!`/`benchmark!`/`test_main!` + `Struct!` for Go-style ports |

## Known gaps

- **`close(ch)` and `select!`** — `Chan<T>` bundles sender+receiver and is
  `Clone`, so an explicit `Close()` can't reliably close the channel while
  any clone still holds a sender. The common Go "sentinel job" pattern works
  (see [`examples/worker.rs`](examples/worker.rs)). A proper close + a
  `select!` macro will need a different sender/receiver split; tracking this
  as its own design pass.
- **Goroutines run on OS threads** (`std::thread::spawn`). No green-thread
  scheduler; don't expect to spawn 100k of them.
- **`string` is an alias for `String`** (owned, mutable). Go strings are
  immutable shared bytes; close enough for most code.

## Examples

```bash
cargo run --example hello       # (val, err), Stringer, bytes.Buffer
cargo run --example config      # strings + strconv config parser
cargo run --example literals    # slice! / map! / chan! / len / append / delete
cargo run --example worker      # goroutines + channel + defer + time::Since
cargo run --example pipeline    # sync.WaitGroup + sort + math + filepath + log + context
cargo run --example webscrape   # url + regexp + json + csv + sha256 + path  (v0.3)
```

## Design priority

The top priority is **Go idioms and call-site syntax** — if a Go programmer
reads a goish program, it should look like Go. Rust idioms (`Option`, `Some`,
`&mut`, trait-bound generics) live *under* the hood. When a Rust idiom has to
leak, it's called out explicitly and a wrapper is designed to minimise it.

## License

Dual-licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
