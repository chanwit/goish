# Goish Rust — Go → Rust Syntax Reference

A single-file conversion table for every Go-like syntax that Goish Rust
supports. Left column is Go; right column is the equivalent Goish Rust.
All examples assume:

```rust
use goish::prelude::*;
```

## Contents

- [1. Primitive types](#1-primitive-types)
- [2. Composite types](#2-composite-types)
- [3. Composite literals](#3-composite-literals)
- [4. `make` / `new` — allocation builtins](#4-make--new--allocation-builtins)
- [5. Built-in functions](#5-built-in-functions)
- [6. Variables, constants, `iota`](#6-variables-constants-iota)
- [7. Control flow](#7-control-flow)
- [8. Functions — multi-return, `_`, variadic](#8-functions--multi-return-_-variadic)
- [9. Type declarations (`type X Y`)](#9-type-declarations-type-x-y)
- [10. Structs — decl + literal + methods](#10-structs--decl--literal--methods)
- [11. Interfaces — Stringer, sort.Interface](#11-interfaces--stringer-sortinterface)
- [12. Errors — `error`, `nil`, `Errorf`, wrap](#12-errors--error-nil-errorf-wrap)
- [13. `fmt` — Print / Sprintf / verbs](#13-fmt--print--sprintf--verbs)
- [14. Goroutines + channels + `select`](#14-goroutines--channels--select)
- [15. `defer` / `recover`](#15-defer--recover)
- [16. Slices — indexing, slicing, `append`, `copy`](#16-slices--indexing-slicing-append-copy)
- [17. Maps](#17-maps)
- [18. Strings / bytes / strconv](#18-strings--bytes--strconv)
- [19. `time`, `sync`, `os`, `io`, `bufio`](#19-time-sync-os-io-bufio)
- [20. Encoding — JSON / base64 / hex / CSV](#20-encoding--json--base64--hex--csv)
- [21. `net/http`, `net/url`, `net/mail`, `net/smtp`, `netip`](#21-nethttp-neturl-netmail-netsmtp-netip)
- [22. Generics-era — `cmp`, `slices`, `maps`, `iter`](#22-generics-era--cmp-slices-maps-iter)
- [23. `crypto`, `hash`, `container/*`, `regexp`, `flag`, `log`, `math/rand`, `unicode`](#23-crypto-hash-container-regexp-flag-log-mathrand-unicode)
- [24. Text toolkit — `html`, `tabwriter`, `scanner`, `template`](#24-text-toolkit--html-tabwriter-scanner-template)
- [25. Testing — `t.Error`, table-driven, benchmarks, `TestMain`](#25-testing--terror-table-driven-benchmarks-testmain)
- [26. Known divergences](#26-known-divergences)

---

## 1. Primitive types

| Go | Goish Rust | Backing Rust type |
|---|---|---|
| `int` | `int` | `i64` |
| `int8` | `int8` | `i8` |
| `int16` | `int16` | `i16` |
| `int32` | `int32` | `i32` |
| `int64` | `int64` | `i64` |
| `uint` | `uint` | `u64` |
| `uint8` | `uint8` | `u8` |
| `uint16` | `uint16` | `u16` |
| `uint32` | `uint32` | `u32` |
| `uint64` | `uint64` | `u64` |
| `float32` | `float32` | `f32` |
| `float64` | `float64` | `f64` |
| `byte` | `byte` | `u8` |
| `rune` | `rune` | `i32` |
| `bool` | `bool` | `bool` |
| `string` | `string` | `GoString` newtype |

Go's `int` is platform-sized; Goish pins it to `i64`.

---

## 2. Composite types

| Go | Goish Rust |
|---|---|
| `[]T` | `slice<T>` (Arc-backed, O(1) re-slice) |
| `map[K]V` | `map<K, V>` |
| `chan T` | `Chan<T>` |
| `error` | `error` (newtype; comparable with `nil`) |
| `*T` | `&T` / `&mut T` / `Arc<T>` as appropriate |
| `interface{}` | `&dyn Trait` or concrete generic |
| `[N]T` (array) | `[T; N]` (Rust array) |

---

## 3. Composite literals

| Go | Goish Rust |
|---|---|
| `[]string{"a","b"}` | `slice!([]string{"a", "b"})` |
| `[]int{1,2,3}` | `slice!([]int{1, 2, 3})` |
| `[]uint64{10, 500}` | `slice!([]uint64{10, 500})` *(bare literals widen via `as`)* |
| `[]T{v1, v2}` | `slice!([]T{v1, v2})` *(any type with `From<_>`)* |
| *(typed alt)* | `slice![T; v1, v2]` |
| *(untyped alt)* | `slice![v1, v2, v3]` |
| `map[string]int{"a":1}` | `map!([string]int{"a" => 1})` |
| *(inferred alt)* | `map!{1i64 => "a", 2i64 => "b"}` |

String literals (`&str`) auto-widen to `string`; integer literals widen
to the declared element type via `as`.

---

## 4. `make` / `new` — allocation builtins

| Go | Goish Rust |
|---|---|
| `make([]T, n)` | `make!([]T, n)` |
| `make([]T, 0, cap)` | `make!([]T, 0, cap)` |
| `make([]T, len, cap)` | `make!([]T, len, cap)` |
| `make(map[K]V)` | `make!(map[K]V)` |
| `make(chan T)` | `make!(chan T)` *or* `chan!(T)` |
| `make(chan T, n)` | `make!(chan T, n)` *or* `chan!(T, n)` |
| `new(T)` | `T::default()` (heap allocation is rarely what you actually want; reach for `Arc<T>` / `Rc<T>` at the module boundary if needed) |

---

## 5. Built-in functions

| Go | Goish Rust |
|---|---|
| `len(x)` | `len!(x)` |
| `cap(x)` | `cap!(x)` |
| `append(s, x, y)` | `append!(s, x, y)` |
| `copy(dst, src)` | `copy!(dst, src)` |
| `delete(m, k)` | `delete!(m, k)` |
| `close(ch)` | `close!(ch)` |
| `panic("msg")` | `panic!("msg")` |
| `recover()` | `recover!{ body }` *(block form)* |
| `print` / `println` | `Println!(...)` |

`len!` works on `string`, `&str`, `slice<T>`, `map<K,V>`, `Chan<T>`.
`cap!` works on `slice<T>` and `Chan<T>`. Both return Go's `int`.

---

## 6. Variables, constants, `iota`

Go:

```go
var x int = 42
x := 42

var ErrNotFound = errors.New("not found")

const Pi = 3.14

const (
    Sunday = iota
    Monday
    Tuesday
)

const (
    _ = iota
    KB = 1 << (10 * iota)
    MB
    GB
)
```

Goish Rust:

```rust
let x: int = 42;
let x = 42i64;

var!(ErrNotFound = errors::New("not found"));
// Call-site:  ErrNotFound()   — returns `error`

const PI: float64 = 3.14;

Const! {
    Sunday = iota;
    Monday;
    Tuesday;
}

Const! {
    _ignored = 1 << (10 * iota);
    KB;
    MB;
    GB;
}
```

`var!` has three forms:

```rust
var!(ErrShortRead = Errorf!("short read"));                // error sentinel
var!(DefaultTimeout time::Duration = time::Second * 30i64); // typed lazy
var! {                                                     // block form
    ErrOne = errors::New("one");
    ErrTwo = errors::New("two");
}
```

---

## 7. Control flow

### `if` / `else`

```go
if err != nil { return err }
if x, err := f(); err != nil { ... }
```

```rust
if err != nil { return err; }
let (x, err) = f();
if err != nil { /* ... */ }
```

### `for` loops

| Go | Goish Rust |
|---|---|
| `for i := 0; i < n; i++ { ... }` | `for i in 0..n { ... }` |
| `for i := 1; i <= n; i++ { ... }` | `for i in 1..=n { ... }` |
| `for cond { ... }` | `while cond { ... }` |
| `for { ... }` | `loop { ... }` |
| `for i, v := range xs { ... }` | `for (i, v) in range!(xs) { ... }` |
| `for _, v := range xs { ... }` | `for (_, v) in range!(xs) { ... }` |
| `for k, v := range m { ... }` | `for (k, v) in range!(m) { ... }` |
| `for i, r := range s` *(string)* | `for (i, r) in range!(s) { ... }` |
| `break` / `continue` | `break;` / `continue;` |

### `switch`

Go:

```go
switch x {
case 1, 2: f()
case 3:    g()
default:   h()
}

// tag-less
switch {
case x > 0: pos()
case x < 0: neg()
}
```

Goish Rust — use Rust's `match`:

```rust
match x {
    1 | 2 => f(),
    3 => g(),
    _ => h(),
}

if      x > 0 { pos(); }
else if x < 0 { neg(); }
```

### `select`

```go
select {
case v := <-c:  fmt.Println(v)
case c <- 42:
default:
}
```

```rust
select! {
    recv(c) |v| => { fmt::Println!(v); },
    send(c, 42) => {},
    default => {},
}
```

---

## 8. Functions — multi-return, `_`, variadic

```go
func divide(a, b int64) (int64, error) {
    if b == 0 { return 0, errors.New("zero") }
    return a / b, nil
}

q, err := divide(10, 2)
_, _   = divide(10, 0)

func sum(xs ...int) int { /* ... */ }
sum(1, 2, 3)
sum(xs...)
```

```rust
fn divide(a: int64, b: int64) -> (int64, error) {
    if b == 0 { return (0, errors::New("zero")); }
    (a / b, nil)
}

let (q, err) = divide(10, 2);
let (_, _)   = divide(10, 0);

fn sum(xs: &[int]) -> int { xs.iter().sum() }
sum(&[1, 2, 3]);
sum(&xs);
```

---

## 9. Type declarations (`type X Y`)

Go:

```go
type ID uint64
type IDSlice []ID
type Headers map[string]string
type Status string
type Priority int
```

Goish Rust — use `Type!` (dispatches on shape):

```rust
Type!(ID = uint64);                    // → IntNewtype!(ID = uint64)
Type!(IDSlice = []ID);                 // → SliceNewtype!(IDSlice = ID)
Type!(Headers = map[string]string);    // → MapNewtype!(Headers = string, string)
Enum!(Status);                         // string enum (const-constructible)
Enum!(Priority = int);                 // int enum

// Lower-level (also public):
IntNewtype!(ID = uint64);
SliceNewtype!(IDSlice = ID);
MapNewtype!(Headers = string, string);
```

`IntNewtype!` gives derives + `From<i32/i64/u32/u64/usize>` so
`slice!([]ID{10, 20})` accepts bare literals.
`SliceNewtype!` derefs to `slice<T>`, so all slice methods flow through.
`MapNewtype!` derefs to `map<K, V>`, so `.Get()`, `[&key]`, and the full
`HashMap` API flow through; it also adds `From<HashMap<K,V>>` and
`IntoIterator` for `(&name)` / `(&mut name)` / by-value.
None emit `Display` — layer with `stringer!`.

### Error-typed declarations — `ErrorType!`

Go:

```go
type MultiError struct { errs []error }
func (m *MultiError) Error() string { /* ... */ }

// Call site:
var err error = &MultiError{errs: es}
```

Goish Rust:

```rust
ErrorType!{
    type MultiError struct {
        errs: slice<error>,
    }
    fn Error(&self) -> string {
        let mut buf = strings::Builder::new();
        for (i, e) in self.errs.iter().enumerate() {
            if i > 0 { buf.WriteString("; "); }
            buf.WriteString(&Sprintf!("%s", e));
        }
        buf.String()
    }
}

// Call site — `.into()` lifts into `error`:
let err: error = MultiError { errs: es }.into();
```

`ErrorType!` emits the struct (derives `Clone, Debug`), an inherent
`Error()` method, `Display`, the hidden `GoishError` impl, and
`From<T> for error`. Recovery at the call site is `errors::As::<T>` —
see §12.

---

## 10. Structs — decl + literal + methods

Go:

```go
type PathTest struct {
    path, result string
}

var t = PathTest{"x", "y"}
t2 := PathTest{path: "x", result: "y"}

func (p PathTest) String() string { return p.path + "=" + p.result }
```

Goish Rust:

```rust
Struct!{ type PathTest struct {
    path, result string
} }

let t = PathTest!("x", "y");                          // positional
let t2 = PathTest { path: "x".into(), result: "y".into() }; // named

stringer! {
    impl PathTest {
        fn String(&self) -> string {
            Sprintf!("%s=%s", self.path, self.result)
        }
    }
}
```

`Struct!` derives `Clone, Debug, Default, PartialEq, Eq, Hash`.
The companion `PathTest!(...)` macro takes positional fields with
automatic `.into()` on string fields.

### Embedded structs & anonymous fields

Go's embedding doesn't have a 1:1 port; compose explicitly:

```go
type Inner struct { X int }
type Outer struct { Inner; Y int }
```

```rust
Struct!{ type Inner struct { X int } }
Struct!{ type Outer struct { inner: Inner, Y: int } }
// Access: o.inner.X  (explicit — no method promotion)
```

---

## 11. Interfaces — Stringer, sort.Interface

### `fmt.Stringer`

```go
type Color struct{ R, G, B int }
func (c Color) String() string {
    return fmt.Sprintf("#%02x%02x%02x", c.R, c.G, c.B)
}
```

```rust
Struct!{ type Color struct { R, G, B int } }

stringer! {
    impl Color {
        fn String(&self) -> string {
            Sprintf!("#%02x%02x%02x", self.R, self.G, self.B)
        }
    }
}
// Now Sprintf!("%s", c), Sprintf!("%v", c), Println!(c) all work.
```

### `sort.Interface`

```go
type ByLen []string
func (s ByLen) Len() int           { return len(s) }
func (s ByLen) Less(i, j int) bool { return len(s[i]) < len(s[j]) }
func (s ByLen) Swap(i, j int)      { s[i], s[j] = s[j], s[i] }

sort.Sort(ByLen(names))
```

```rust
SliceNewtype!(ByLen = string);

impl sort::Interface for ByLen {
    fn Len(&self) -> int { len!(self.0) }
    fn Less(&self, i: int, j: int) -> bool {
        self.0[i].len() < self.0[j].len()
    }
    fn Swap(&mut self, i: int, j: int) { self.0.Swap(i, j); }
}

let mut names: ByLen = /* ... */;
sort::Sort(&mut names);
```

### User interfaces — `Interface!`

Go:

```go
type Core interface {
    Write(msg string)
    With(tag string) Core
    Tags() []string
}

// Composition via embedding:
type LevelEnabler interface { Enabled(lvl int) bool }
type TraceCore interface {
    LevelEnabler
    Emit(lvl int, msg string)
}
```

Goish Rust:

```rust
Interface!{
    type Core interface {
        fn Write(&self, msg: &str);
        fn With(&self, tag: &'static str) -> Core;
        fn Tags(&self) -> Vec<&'static str>;
    }
}

Interface!{
    type LevelEnabler interface {
        fn Enabled(&self, lvl: i32) -> bool;
    }
}

// Supertrait clause — `type Name: Super[+Super]* interface { … }`:
Interface!{
    type TraceCore: LevelEnabler interface {
        fn Emit(&self, lvl: i32, msg: &str);
    }
}
```

Implementing an interface:

```rust
#[derive(Clone)]
struct InMem { tags: Vec<&'static str>, sink: Arc<Mutex<Vec<String>>> }

Interface!{
    impl Core for InMem {
        fn Write(&self, msg: &str) { /* ... */ }
        fn With(&self, tag: &'static str) -> Core {
            let mut tags = self.tags.clone();
            tags.push(tag);
            InMem { tags, sink: self.sink.clone() }.into()  // .into() lifts into Core
        }
        fn Tags(&self) -> Vec<&'static str> { self.tags.clone() }
    }
}
```

Call site — transparent cloning, no `Box<dyn …>` or `clone_trait_object!`
at user surface:

```rust
let base: Core = InMem { /* ... */ }.into();
let child = base.clone();                 // interface values clone
let worker = base.With("worker");         // With returns Core
worker.Write("hello");
```

Notes:

- Method signatures use Rust shape (`fn <name>(&self[, …]) [-> T];`,
  trailing `;`) — parameter/return types can be Go-shaped (`string`,
  `slice<T>`, `int`, `error`) or Rust-shaped.
- Any concrete type implementing the interface must `#[derive(Clone)]`.
- `.into()` lifts a concrete impl into the interface newtype.
- Supertrait clause: bare idents are name-mangled to the hidden trait;
  path forms (e.g. `io::Writer`) pass through verbatim. Bound-only in
  v0.21 — parent methods aren't auto-forwarded on the child newtype.

---

## 12. Errors — `error`, `nil`, `Errorf`, wrap

```go
err := errors.New("not found")
if err != nil { return err }

wrapped := fmt.Errorf("lookup %s: %w", key, err)
if errors.Is(wrapped, err) { ... }

inner := errors.Unwrap(wrapped)
```

```rust
let err = errors::New("not found");
if err != nil { return err; }

let wrapped = Errorf!("lookup %s: %w", key, err);
if errors::Is(&wrapped, &err) { /* ... */ }

let inner = errors::Unwrap(&wrapped);
```

Key facts:

- `error` is a newtype, not `Option<…>` — `errors::New("…")` returns `error` directly, no `Ok`/`Some`.
- `nil` is a constant `error` value. `err != nil` compiles.
- `Chan<T>`, future `Pointer<T>`, etc. implement `PartialEq<error>` so
  `ch != nil` works polymorphically.
- `IsNil` trait provides `.is_nil()` when you need the method form.

Sentinel errors:

```rust
var!(ErrNotFound   = errors::New("not found"));
var!(ErrPermission = errors::New("permission denied"));
// Call site:  if err == ErrNotFound() { ... }
```

---

## 13. `fmt` — Print / Sprintf / verbs

| Go | Goish Rust |
|---|---|
| `fmt.Println(a, b)` | `fmt::Println!(a, b)` or `Println!(a, b)` |
| `fmt.Printf(fmt, a)` | `fmt::Printf!(fmt, a)` or `Printf!(fmt, a)` |
| `fmt.Sprintf(fmt, a)` | `fmt::Sprintf!(fmt, a)` or `Sprintf!(fmt, a)` |
| `fmt.Fprintf(w, ...)` | `Fprintf!(&mut w, ...)` |
| `fmt.Errorf(...)` | `Errorf!(...)` |

Verb support — takes anything `Display` *or* `Debug` (universal, like Rust's `format!`):

| Verb | Meaning |
|---|---|
| `%v`, `%+v` | default value (Display; falls back to Debug) |
| `%s` | string / Stringer |
| `%d` | decimal int |
| `%x`, `%X` | hex |
| `%o`, `%b` | octal, binary |
| `%f`, `%e`, `%g` | float |
| `%t` | bool |
| `%c` | rune (character) |
| `%q` | quoted string |
| `%p` | pointer |
| `%w` | wrap error (in `Errorf!` only) |
| Width / precision | `%5d`, `%-8s`, `%.2f`, `%08x` |

---

## 14. Goroutines + channels + `select`

```go
jobs := make(chan int, 4)
go func() {
    for i := 1; i <= 3; i++ { jobs <- i }
    close(jobs)
}()
for v := range jobs { fmt.Println(v) }
```

```rust
let jobs: Chan<int> = chan!(int, 4);
let g = go!{
    for i in 1..=3 { jobs.Send(i); }
    close!(jobs);
};
while let (v, true) = jobs.Recv() {
    fmt::Println!(v);
}
let _ = g.Wait();
```

| Go | Goish Rust |
|---|---|
| `go f()` | `go!{ f(); }` — returns `Goroutine` |
| `ch <- v` | `ch.Send(v)` |
| `v := <-ch` | `let (v, _) = ch.Recv()` |
| `v, ok := <-ch` | `let (v, ok) = ch.Recv()` |
| `close(ch)` | `close!(ch)` |
| `cap(ch)` / `len(ch)` | `cap!(ch)` / `len!(ch)` |
| `select { ... }` | `select! { ... }` |

Inside `go!{ ... }`, `.Send/.Recv/.Wait` are rewritten to their
`.await` forms at compile time by `goish-macros`.

`select!` arms:

```rust
select! {
    recv(ch)      |v|      => { ... },       // v, ok = <-ch (closed → ok=false)
    recv(ch, _)   |v, ok|  => { ... },       // explicit two-value form
    send(ch, expr)         => { ... },
    default                => { ... },
}
```

Nil channel: `Chan::<T>::default()` returns a nil channel — sends and
receives block forever, matching Go semantics.

---

## 15. `defer` / `recover`

```go
func foo() {
    defer fmt.Println("cleanup")
    defer fmt.Println("inner")
    fmt.Println("work")
}
// prints: work, inner, cleanup
```

```rust
fn foo() {
    defer!{ fmt::Println!("cleanup"); }
    defer!{ fmt::Println!("inner"); }
    fmt::Println!("work");
}
```

Recover — Go uses a deferred closure; Goish uses a `recover!` block that
returns `Option<String>`:

```go
defer func() {
    if r := recover(); r != nil { log.Printf("panic: %v", r) }
}()
doPanickyThing()
```

```rust
if let Some(r) = recover!{ do_panicky_thing() } {
    log::Printf!("panic: %s\n", r);
}
```

---

## 16. Slices — indexing, slicing, `append`, `copy`

Go slice header semantics — Goish `slice<T>` is Arc-backed so re-slicing
is O(1):

| Go | Goish Rust |
|---|---|
| `s[i]` | `s[i as int]` (accepts `int`/`usize`) |
| `s[i] = v` | `s[i as int] = v` |
| `s[i:j]` | `s.Slice(i, j)` |
| `s[i:]` | `s.SliceFrom(i)` |
| `s[:j]` | `s.SliceTo(j)` |
| `s[i:j:k]` | *(deferred — Go's three-index slice)* |
| `len(s)`, `cap(s)` | `len!(s)`, `cap!(s)` |
| `append(s, x)` | `append!(s, x)` |
| `append(s, a...)` | `append!(s, a0, a1, ...)` *(no direct splat; use `.extend`)* |
| `copy(dst, src)` | `copy!(dst, src)` |
| `s[i], s[j] = s[j], s[i]` | `s.Swap(i, j)` |

Arc-backed mutation: a shared slice forks on write via copy-on-write
(`.cow()`). For tight loops you can pre-fork once:
`s.cow(); for ... { s[i] = v; }`.

**Integer indexing.** `slice<T>` accepts `int` (Go's default integer) and
`usize` directly — `s[i as int]` / `s[i as usize]`. Use `as int` at the
call site when the index starts as a Rust `i32` or an arithmetic result,
so the Go `s[i]` shape ports verbatim.

---

## 17. Maps

| Go | Goish Rust |
|---|---|
| `m := make(map[K]V)` | `let mut m = make!(map[K]V);` or `let mut m: map<K, V> = map::new();` |
| `m[k] = v` | `m[&k] = v` *(insert-on-miss, IndexMut)* |
| `v := m[k]` | `let v = m[&k].clone();` *(zero-value on miss)* |
| `v, ok := m[k]` | `let (v, ok) = m.Get(&k);` |
| `_, ok := m[k]` | `let (_, ok) = m.Get(&k);` |
| `delete(m, k)` | `delete!(m, &k)` |
| `len(m)` | `len!(m)` |
| `for k, v := range m` | `for (k, v) in range!(m) { ... }` |

Missing-key behavior matches Go: `m[k]` returns the zero value of `V`
(any `V: Default`).

**Deref to `HashMap`.** `map<K, V>` is a thin newtype that derefs to
`std::collections::HashMap<K, V>`, so the full std API (`insert`,
`remove`, `contains_key`, `iter`, `entry`, …) is available on any `map`
value — including map newtypes emitted by `MapNewtype!` / `Type!(Name =
map[K]V)`, which themselves deref to `map<K, V>`.

---

## 18. Strings / bytes / strconv

```go
strings.Contains(s, "go")
strings.Replace(s, "a", "b", -1)
strings.Split("a,b,c", ",")
strings.ToUpper(s); strings.ToLower(s)
strings.TrimSpace(s)
strings.HasPrefix(s, "go")
strings.HasSuffix(s, "lang")

var buf strings.Builder
buf.WriteString("hello"); buf.WriteRune('!')
s := buf.String()

strconv.Atoi("42")
strconv.Itoa(42)
strconv.FormatFloat(3.14, 'f', 2, 64)
strconv.ParseBool("true")
strconv.Quote("hi\n")

bytes.Equal(a, b)
bytes.NewBuffer(data).String()
```

```rust
strings::Contains(&s, "go");
strings::Replace(&s, "a", "b", -1);
strings::Split("a,b,c", ",");
strings::ToUpper(&s); strings::ToLower(&s);
strings::TrimSpace(&s);
strings::HasPrefix(&s, "go");
strings::HasSuffix(&s, "lang");

let mut buf = strings::Builder::new();
buf.WriteString("hello"); buf.WriteRune('!');
let s = buf.String();

strconv::Atoi("42");
strconv::Itoa(42);
strconv::FormatFloat(3.14, 'f', 2, 64);
strconv::ParseBool("true");
strconv::Quote("hi\n");

bytes::Equal(&a, &b);
bytes::NewBuffer(data).String();
```

String interop helpers:

| Form | Use |
|---|---|
| `string::from("lit")` | `&str` → `string` |
| `s.as_str()` | `string` → `&str` |
| `cat!(a, b, c)` | Go-shaped `+` concat |
| `string!(...)` | builds `string` from any Display args |

---

## 19. `time`, `sync`, `os`, `io`, `bufio`

### `time`

```go
start := time.Now()
time.Sleep(100 * time.Millisecond)
elapsed := time.Since(start)
t := time.Date(2026, 4, 17, 12, 0, 0, 0, time.UTC)
s := t.Format("2006-01-02")
```

```rust
let start = time::Now();
time::Sleep(time::Millisecond * 100i64);
let elapsed = time::Since(start);
let t = time::Date(2026, 4, 17, 12, 0, 0, 0, time::UTC);
let s = t.Format("2006-01-02");
```

### `sync`

```go
var mu sync.Mutex
mu.Lock(); defer mu.Unlock()

var wg sync.WaitGroup
wg.Add(1); go func() { defer wg.Done(); work() }()
wg.Wait()

var once sync.Once
once.Do(func() { init() })
```

```rust
let mu = sync::Mutex::new(0i64);
{ let mut g = mu.Lock(); *g += 1; }   // auto-unlock

let wg = sync::WaitGroup::new();
wg.Add(1);
let w = wg.clone();
go!{ work(); w.Done(); };
wg.Wait();

let once = sync::Once::new();
once.Do(|| init());
```

`sync/atomic` — `AtomicInt64`, `AtomicUint64`, `AtomicBool`, etc. with Go-style
`.Load()`, `.Store()`, `.Add()`, `.CompareAndSwap()` methods.

### `os`, `io`, `bufio`

```go
data, err := os.ReadFile("/etc/hosts")
os.WriteFile("out.txt", data, 0644)

f, _ := os.Open("data.txt")
defer f.Close()
scanner := bufio.NewScanner(f)
for scanner.Scan() { line := scanner.Text() }

os.Getenv("HOME")
os.Setenv("KEY", "val")
os.Exit(0)
```

```rust
let (data, err) = os::ReadFile("/etc/hosts");
os::WriteFile("out.txt", &data, 0o644);

let (f, _) = os::Open("data.txt");
defer!{ f.Close(); }
let mut scanner = bufio::NewScanner(f);
while scanner.Scan() { let line = scanner.Text(); }

os::Getenv("HOME");
os::Setenv("KEY", "val");
os::Exit(0);
```

---

## 20. Encoding — JSON / base64 / hex / CSV

### JSON

```go
data, _ := json.Marshal(map[string]int{"a": 1})
var out map[string]int
json.Unmarshal(data, &out)
```

```rust
let (data, _) = json::Marshal(&map!([string]int{"a" => 1}));
let (out, _) = json::Unmarshal::<HashMap<string, int>>(&data);
```

### base64 / hex / binary

```rust
base64::StdEncoding.EncodeToString(&b);
base64::StdEncoding.DecodeString(&s);
hex::EncodeToString(&b);
hex::DecodeString(&s);
binary::BigEndian.Uint32(&buf);
binary::LittleEndian.PutUint64(&mut buf, x);
```

### CSV

```rust
let mut r = csv::NewReader(strings::NewReader("a,b\n1,2\n"));
let (records, _) = r.ReadAll();

let mut w = csv::NewWriter(&mut os::Stdout());
w.Write(&slice!([]string{"a", "b"}));
w.Flush();
```

---

## 21. `net/http`, `net/url`, `net/mail`, `net/smtp`, `netip`

### HTTP server

```rust
http::HandleFunc("/hello", |w, r| {
    Fprintf!(w, "hi %s", r.URL.Path);
});
log::Fatalf!("%s", http::ListenAndServe(":8080", nil));
```

### HTTP client + context

```rust
let (ctx, _)      = context::WithTimeout(context::Background(), 5i64 * time::Second);
let (req, _)      = http::NewRequestWithContext(ctx, "GET", url, &[]);
let (resp, err)   = http::Do(req);
let (body, _)     = io::ReadAll(resp.Body);
```

### Cookies

```rust
let c = Cookie!{
    Name:     "session",
    Value:    "abc",
    Path:     "/",
    HttpOnly: true,
    SameSite: http::SameSiteLaxMode,
};
let (cookies, _) = http::ParseCookie("a=1; b=2");
```

### URL / netip / mail / smtp

```rust
let (u, _) = url::Parse("https://x.com/p?q=1");
let mut v = url::Values::new();
v.Set("k", "v"); v.Encode();

let (addr, _) = netip::ParseAddr("fe80::1%eth0");
addr.Is6(); addr.Zone();
let (p, _) = netip::ParsePrefix("10.0.0.0/8");
p.Contains(netip::MustParseAddr("10.1.2.3"));

let (a, _) = mail::ParseAddress(r#""Joe Q." <joe@x.com>"#);
a.Name; a.Address;

let (mut c, _) = smtp::Dial("mail.example.com:25");
c.Hello("localhost"); c.Mail("from@x"); c.Rcpt("to@y");
let (mut w, _) = c.Data();
w.Write(b"Subject: hi\r\n\r\nHello."); w.Close();
c.Quit();
```

---

## 22. Generics-era — `cmp`, `slices`, `maps`, `iter`

### `cmp`

```rust
cmp::Compare(1, 2);       // -1
cmp::Less(1.5, 2.5);      // true
cmp::Or(&[0, 0, 42]);     // 42
```

### `slices`

```rust
slices::Contains(&xs, &2);
slices::Index(&names, &"b".to_string());
slices::Sort(&mut xs);
slices::SortFunc(&mut xs, |a, b| a - b);
slices::Reverse(&mut xs);
slices::Min(&xs); slices::Max(&xs);
slices::BinarySearch(&sorted, &42);
slices::Insert(&mut xs, 1, &[99, 100]);
slices::Delete(&mut xs, 1, 3);
slices::Compact(&mut xs);
slices::Concat(&[&a, &b, &c]);
slices::Repeat(&xs, 3);
slices::Clone(&xs);
```

### `maps`

```rust
let keys = maps::Keys(&m);
let values = maps::Values(&m);
maps::Equal(&a, &b);
let cloned = maps::Clone(&m);
maps::Copy(&mut dst, &src);
maps::DeleteFunc(&mut m, |_k, v| *v < 0);
```

### `iter`

```rust
for v in iter::Values(&xs) { use_v(v); }
for (k, v) in iter::All(&m) { /* ... */ }
```

---

## 23. `crypto`, `hash`, `container/*`, `regexp`, `flag`, `log`, `math/rand`, `unicode`

### `crypto/{md5,sha1,sha256}`

```rust
let mut h = sha256::New();
h.Write(b"hello");
let sum = h.Sum(&[]);
Printf!("%x\n", sum);

let sum = sha256::Sum256(b"hello"); // one-shot
```

### `hash/{crc32, fnv}`

```rust
crc32::ChecksumIEEE(b"abc");
let mut h = fnv::New32a();
h.Sum32();
```

### `container/{list, heap, ring}`

```rust
// list / ring use `make!` to mirror Go's `list.New()` / `ring.New(n)`
// without the turbofish.
let mut l = make!(list[int]);
l.PushBack(1); l.PushBack(2); l.PushFront(0);
let mut e = l.Front();
while let Some(node) = e { fmt::Println!(node.Value); e = node.Next(); }

// heap takes a less-fn closure, so it stays at the explicit constructor;
// turbofish is the trade for keeping the comparator visible at the
// call site (matches Go's `container/heap.Init(h)` ceremony).
let mut h = container::heap::New::<int>(|a, b| a < b);
h.Push(2); h.Push(1); h.Push(5);
let min = h.Pop();

let r = make!(ring[int], 3);
```

### `regexp`

```rust
let re = regexp::MustCompile(r"^\d+$");
re.MatchString("123");
re.FindString("abc 42 xyz");
re.ReplaceAllString("abc 42", "*");
re.FindAllStringSubmatch("a=1 b=2", -1);
```

### `flag` / `log` / `math/rand` / `unicode`

```rust
let port = flag::Int("port", 8080, "listen port");
let name = flag::String("name", "goish", "name");
flag::Parse();

log::Println!("server started");
log::Printf!("port %d\n", 8080);
log::Fatalf!("boom: %s", err);

rand::Seed(42);
rand::Intn(100);
rand::Float64();
rand::Shuffle(len!(xs), |i, j| xs.swap(i as usize, j as usize));

unicode::IsLetter('A');
unicode::ToUpper('a');
utf8::RuneCountInString("héllo");
utf8::ValidString("...");
```

---

## 24. Text toolkit — `html`, `tabwriter`, `scanner`, `template`

```rust
html::EscapeString("<a>&\"'");
html::UnescapeString("&lt;a&gt;");

let mut w = tabwriter::NewWriter(&mut os::Stdout(), 0, 8, 1, b'\t', 0);
Fprintf!(&mut w, "name\tage\n"); Fprintf!(&mut w, "alice\t30\n");
w.Flush();

let mut s = scanner::Scanner::new();
s.Init(strings::NewReader("x + 3.14"));
loop { let tok = s.Scan(); if tok == scanner::EOF { break; } }

let (t, _) = template::New("t").Parse("Hi {{.Name}}!");
let mut out = String::new();
t.Execute(&mut out, &serde_json::json!({"Name": "alice"}));
```

---

## 25. Testing — `t.Error`, table-driven, benchmarks, `TestMain`

### Basic test

```go
func TestClean(t *testing.T) {
    for _, tc := range tests {
        got := path.Clean(tc.path)
        if got != tc.want {
            t.Errorf("Clean(%q) = %q, want %q", tc.path, got, tc.want)
        }
    }
}
```

```rust
test!{ fn TestClean(t) {
    for tc in &tests() {
        let got = path::Clean(&tc.path);
        if got != tc.want {
            t.Errorf(Sprintf!("Clean(%q) = %q, want %q",
                tc.path, got, tc.want));
        }
    }
}}
```

### Table-driven

```rust
Struct!{ type PathTest struct { path, want string } }

fn tests() -> slice<PathTest> { slice!([]PathTest{
    PathTest!("",    "."),
    PathTest!("abc", "abc"),
})}
```

### Benchmarks

```rust
benchmark!{ fn BenchmarkJoin(b) {
    b.ReportAllocs();
    while b.Loop() {
        path::Join(&slice!([]string{"a", "b"}));
    }
}}
```

### `TestMain` / custom harness

```rust
// tests/my_test.rs  — in Cargo.toml: harness = false
use goish::prelude::*;

test_h!{ fn TestAddition(t) {
    if 2 + 2 != 4 { t.Error("math broke"); }
}}

test_main!{ fn TestMain(m) {
    setup();
    let code = m.Run();
    teardown();
    os::Exit(code);
}}
```

### `testing.T` methods

| Go | Goish Rust |
|---|---|
| `t.Error("msg")` | `t.Error("msg");` |
| `t.Errorf(fmt, a)` | `t.Errorf(Sprintf!(fmt, a));` |
| `t.Fatal("msg")` | `t.Fatal("msg");` |
| `t.Fatalf(fmt, a)` | `t.Fatalf(Sprintf!(fmt, a));` |
| `t.Log(...)` | `t.Log(&Sprintf!(...));` |
| `t.Skip("why")` | `t.Skip("why");` |
| `t.Run(name, f)` | `t.Run(name, &mut |t| { ... });` |
| `t.Helper()` | *(no-op; Rust has no Helper machinery)* |

---

## 26. Known divergences

These are spots where full Go semantics can't be expressed in safe Rust,
or where the port deliberately simplifies:

- **Arc-backed slice CoW.** `slice<T>` clones are Arc-shared; mutating a
  shared slice forks via `.cow()`. Writes are *not* visible across
  previously-shared clones (Go would let the aliased writer mutate the
  backing array). Explicit CoW is the safety cost.
- **Three-index slice `s[i:j:k]`.** Deferred — re-slice currently uses
  `Slice(i, j)` / `SliceFrom(i)` / `SliceTo(j)`.
- **Embedded structs & method promotion.** No 1:1 port; use explicit
  composition and reach through the field.
- **`interface{}` / `any`.** Use concrete generics or `&dyn Trait`; there
  is no runtime type-switch on arbitrary `any`.
- **Variadic splat `f(xs...)`.** Rust has no splat. Take `&[T]` and call
  `f(&xs)`; for appends, rewrite `append(s, xs...)` as
  `{ let mut s = s; s.extend_from_slice(&xs); s }`.
- **Pointers vs references.** Go's `*T` maps to `&T` / `&mut T` /
  `Arc<T>` case-by-case; pick what fits ownership.
- **`goto`, labeled break/continue.** Not supported. Factor the loop.
- **`%q` on `[]byte` and a few type-aware verbs.** Deferred.
- **Nil-for-every-type (`NilValue`).** Deferred. `nil` is `error`-typed;
  cross-type comparisons work where we've added `PartialEq<error>`
  (currently `Chan<T>`; more coming).
- **`static AtomicBool` / `static AtomicI64`.** Goish's
  `sync::atomic::{Bool,Int64}` are `Arc`-backed (`Clone`, no const-fn
  constructor). Static contexts that need a const initializer keep
  `std::sync::atomic::*` directly — no Goish wrapper applies.
- **`AtomicI64::fetch_add` for OLD-value semantics.** Goish's
  `sync::atomic::Int64::Add` returns the *new* value (Go convention);
  call sites that rely on the OLD value (test ordering probes,
  swap-style counters) keep `std::sync::atomic::AtomicI64` + `fetch_add`.
- **Turbofish `::<T>` that is Goish-correct (not leaks).** When the
  type parameter carries dispatch information that the caller would
  have to write *somewhere* (Go uses a different syntactic slot for
  the same info), the turbofish is the natural Goish shape:
  `errors::As::<MultiError>(&err)` mirrors Go's `errors.As(err, &target)`
  where `target` is `*MultiError` — the type drives recovery either
  way. Same for `json::Unmarshal::<HashMap<string, int>>(&data)`
  (Go: `json.Unmarshal(data, &out)` where `out` is the typed target),
  `Chan::<T>::default()` for nil channels (no value to infer from),
  and `container::heap::New::<int>(|a, b| a < b)` (the comparator
  closure can't always pin T). Bare turbofish on a *no-arg constructor
  with type-only inference burden* (e.g. `list::New::<int>()`,
  `ring::New::<int>(3)`) IS a leak — fix with a `make!`-family arm.
- **`'static` lifetimes that are Goish-correct (not leaks).** Three
  patterns where `'static` in a public signature mirrors Go's actual
  surface and shouldn't be sealed: (a) **singleton returns** —
  `net::http::DefaultClient() -> &'static Client`,
  `hash::crc32::IEEETable() -> &'static Table` mirror Go's package-
  level singletons (`http.DefaultClient`, `crc32.IEEETable`); (b)
  **closure-storage bounds** — `time::AfterFunc<F: FnOnce() + Send + 'static>`,
  `container::heap::New(less: impl Fn(...) + Send + 'static)` —
  closures stored beyond the call need `'static`, and Go's runtime
  pins the closure's captured env equivalently; (c) **internal-test
  scaffolding** — `net::smtp::NullConn::with_reader<R: Read + Send + 'static>`
  wraps the value in `Box<dyn Read + Send>` for storage. Only `'static`
  on a *plain reference return* (e.g. `pub fn Foo() -> &'static str`)
  is a leak — return Goish `string` instead.
- **`Vec<T>` left in tests where Goish has no slice-equivalent
  surface.** Sweep policy: tests/examples should read like Go, but
  five categories keep `Vec<T>`:
  (a) `Vec<&'static str>` and `Vec<&str>` — Goish has no `slice<&str>`
  form (the slice<T> Owned model assumes T owns); static borrowed
  string literals stay in `Vec<&str>`.
  (b) Tuple-element vecs (`Vec<(K, V)>`, `Vec<(&str, bool)>`) — the
  `slice!([]T { ... })` macro doesn't compose cleanly through tuple
  constructors; bare `Vec<(...)>` stays.
  (c) `Vec<u8>` as an `io::Writer` sink — `slice<byte>` doesn't
  implement `std::io::Write`, only `Vec<u8>` and `bytes::Buffer` do.
  Test sinks that pass `&mut dst` to `gio::Copy/CopyN/WriteString`
  etc. keep `Vec<u8>`. Affects `tests/io_test.rs`,
  `tests/bufio_test.rs`, `tests/json_encode_test.rs`.
  (d) `String::with_capacity(n) + push_str` — Goish `string` is
  immutable; the build-by-pushing pattern (hex digits, etc.) keeps
  `String`. Affects `tests/crypto_{md5,sha1,sha256}_test.rs`.
  (e) Index-by-`usize` callers — `slice<T>` indexes by `i64`, not
  `usize`, so loops like `for i in 1..data.len() { data[i] }` cannot
  migrate without rewriting the loop. Affects most of
  `tests/sort_test.rs` and `tests/container_heap_test.rs::drain_to_vec`.
  Per-callsite library widening (Index<usize>) would unblock them but
  is out of scope for the sweep.
- **`std::thread::spawn` over `go!{}` in four categories.** `go!{}` is
  tokio-task-based, which is wrong for: (a) **panic capture via
  `JoinHandle::join() -> Result<_, _>`** — tokio task panics surface
  through different machinery; (b) **mock servers using sync std I/O**
  (`TcpListener::accept`, `BufReader::read_line`) — blocking calls park
  a worker thread indefinitely, fine on `thread::spawn`; (c) **explicit
  thread-vs-task benchmarks** (`tests/chan_bench.rs`); (d) **hyper-owned
  runtime tests** (`tests/http_client_serve_test.rs`) — hyper drives its
  own runtime, mixing in tokio-tasks dead-locks. All four keep
  `std::thread::spawn`.

---

For a package-by-package port coverage status, see
[PROGRESS.md](PROGRESS.md). For task-oriented recipes, see
[COOKBOOK.md](COOKBOOK.md).
