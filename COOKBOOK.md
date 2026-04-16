# Goish Rust Cookbook

Go → Goish Rust translation reference. Every section shows the Go
idiom on the left and its Goish Rust equivalent on the right. The
import `use goish::prelude::*` is assumed throughout.

## Contents

- [Types](#types)
- [Composite literals](#composite-literals)
- [Built-ins](#built-ins)
- [Control flow](#control-flow)
- [Errors](#errors)
- [`fmt` and `Stringer`](#fmt-and-stringer)
- [Goroutines and channels](#goroutines-and-channels)
- [Defer / recover](#defer--recover)
- [Strings, bytes, strconv](#strings-bytes-strconv)
- [Time](#time)
- [Sync](#sync)
- [I/O, files, bufio](#io-files-bufio)
- [Encoding (JSON, base64, hex, CSV)](#encoding)
- [Networking (HTTP, URL, netip, mail, smtp)](#networking)
- [Multipart, cookies, MIME](#multipart-cookies-mime)
- [Generics-era helpers (`cmp`, `slices`, `maps`, `iter`)](#generics-era-helpers)
- [Text toolkit (`html`, `text/tabwriter`, `text/scanner`, `text/template`)](#text-toolkit)
- [Crypto and hashing](#crypto-and-hashing)
- [Container types (`heap`, `list`, `ring`)](#container-types)
- [Regexp, flag, log, math/rand, unicode](#smaller-packages)
- [Testing](#testing)

---

## Types

| Go | Goish Rust |
|---|---|
| `int` / `int64` / `float64` / `byte` / `rune` | `int` / `int64` / `float64` / `byte` / `rune` |
| `string` | `string` *(alias for `std::string::String`)* |
| `[]T` | `slice<T>` *(alias for `Vec<T>`)* |
| `map[K]V` | `map<K, V>` *(alias for `HashMap<K, V>`)* |
| `chan T` | `Chan<T>` |
| `error` | `error` *(newtype; `nil` is a zero-value constant)* |

---

## Composite literals

| Go | Goish Rust |
|---|---|
| `[]string{"a","b","c"}` | `slice!([]string{"a","b","c"})` |
| `[]int{1,2,3}` | `slice!([]int{1,2,3})` |
| `map[string]int{"a":1,"b":2}` | `map!([string]int{"a" => 1, "b" => 2})` |
| `make([]int, 5)` | `make!([]int, 5)` |
| `make([]T, 0, n)` | `make!([]T, 0, n)` |
| `make(map[K]V)` | `make!(map[K]V)` |
| `make(chan int, 10)` | `make!(chan int, 10)` or `chan!(int, 10)` |

---

## Built-ins

| Go | Goish Rust |
|---|---|
| `len(x)` | `len!(x)` |
| `append(s, x, y)` | `append!(s, x, y)` |
| `delete(m, k)` | `delete!(m, k)` |
| `close(ch)` | `close!(ch)` |

---

## Control flow

Go's `for i, v := range xs` loop:

```go
for i, v := range xs {
    fmt.Println(i, v)
}
```

Goish Rust — native `for ... in` with `range!`:

```rust
for (i, v) in range!(xs) {
    fmt::Println!(i, v);
}
```

Go's `select`:

```go
select {
case v := <-c:    fmt.Println(v)
case c <- 42:    // sent
default:
}
```

Goish Rust — proc-macro `select!`:

```rust
select! {
    recv(c) |v| => { fmt::Println!(v); },
    send(c, 42) => {},
    default => {},
}
```

---

## Errors

```go
err := errors.New("not found")
wrapped := fmt.Errorf("lookup: %w", err)
if errors.Is(wrapped, err) { ... }
```

```rust
let err = errors::New("not found");
let wrapped = Errorf!("lookup: %w", err);
if errors::Is(&wrapped, &err) { /* ... */ }
```

`errors::New` returns `error` directly — no `Some`/`Ok` wrapping.
`err != nil` works because `error` implements `PartialEq<error>`
with the `nil` constant.

---

## `fmt` and Stringer

```go
fmt.Println("n =", 42)
s := fmt.Sprintf("%-8s = %d", "answer", 42)
e := fmt.Errorf("code %d: %s", 500, "oops")
```

```rust
fmt::Println!("n =", 42);
let s = fmt::Sprintf!("%-8s = %d", "answer", 42);
let e = fmt::Errorf!("code %d: %s", 500, "oops");
```

**Stringer** — implement Go's `String() string` interface:

```go
type Color struct{ R, G, B int }
func (c Color) String() string {
    return fmt.Sprintf("#%02x%02x%02x", c.R, c.G, c.B)
}
```

```rust
struct Color { R: int, G: int, B: int }
fmt::stringer! {
    impl Color {
        fn String(&self) -> string {
            fmt::Sprintf!("#%02x%02x%02x", self.R, self.G, self.B)
        }
    }
}
```

---

## Goroutines and channels

```go
jobs := make(chan int, 4)
go func() {
    for i := 1; i <= 3; i++ { jobs <- i }
    close(jobs)
}()
for v := range jobs {
    fmt.Println("got:", v)
}
```

```rust
let jobs: Chan<int> = chan!(int, 4);
let g = go!{
    for i in 1..=3 { jobs.Send(i); }
    close!(jobs);
};
while let (v, true) = jobs.Recv() {
    fmt::Println!("got:", v);
}
let _ = g.Wait();
```

Inside `go!{ ... }`, channel method calls look identical to outside
— a proc-macro rewrites `.Send` / `.Recv` / `.Wait` into their
`.await` forms at compile time.

---

## Defer / recover

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

Go's `defer / recover`:

```go
defer func() {
    if r := recover(); r == nil {
        t.Fatal("want panic")
    }
}()
doPanickyThing()
```

```rust
let r = recover!{ do_panicky_thing() };
if r.is_none() { t.Fatal("expected panic"); }
```

---

## Strings, bytes, strconv

```go
strings.Contains(s, "go")
strings.Replace(s, "a", "b", -1)
strings.Split("a,b,c", ",")

var buf strings.Builder
buf.WriteString("hello")

strconv.Atoi("42")
strconv.Itoa(42)
strconv.FormatFloat(3.14, 'f', 2, 64)
```

```rust
strings::Contains(&s, "go");
strings::Replace(&s, "a", "b", -1);
strings::Split("a,b,c", ",");

let mut buf = strings::Builder::new();
buf.WriteString("hello");

strconv::Atoi("42");
strconv::Itoa(42);
strconv::FormatFloat(3.14, 'f', 2, 64);
```

---

## Time

```go
start := time.Now()
time.Sleep(100 * time.Millisecond)
elapsed := time.Since(start)
fmt.Printf("took %s\n", elapsed)

t := time.Date(2026, 4, 16, 12, 0, 0, 0, time.UTC)
s := t.Format("2006-01-02")
```

```rust
let start = time::Now();
time::Sleep(time::Millisecond * 100i64);
let elapsed = time::Since(start);
fmt::Printf!("took %s\n", elapsed);

let t = time::Date(2026, 4, 16, 12, 0, 0, 0, time::UTC);
let s = t.Format("2006-01-02");
```

---

## Sync

```go
var mu sync.Mutex
mu.Lock()
defer mu.Unlock()

var wg sync.WaitGroup
for i := 0; i < 4; i++ {
    wg.Add(1)
    go func() { defer wg.Done(); work() }()
}
wg.Wait()
```

```rust
let mu = sync::Mutex::new(0i64);
{ let mut g = mu.Lock(); *g += 1; } // auto-unlock at scope end

let wg = sync::WaitGroup::new();
for _ in 0..4 {
    wg.Add(1);
    let w = wg.clone();
    go!{ work(); w.Done(); };
}
wg.Wait();
```

---

## I/O, files, bufio

```go
data, err := os.ReadFile("/etc/hosts")
os.WriteFile("out.txt", data, 0644)

f, _ := os.Open("data.txt")
scanner := bufio.NewScanner(f)
for scanner.Scan() {
    line := scanner.Text()
}
```

```rust
let (data, err) = os::ReadFile("/etc/hosts");
os::WriteFile("out.txt", &data, 0o644);

let (f, _) = os::Open("data.txt");
let mut scanner = bufio::NewScanner(f);
while scanner.Scan() {
    let line = scanner.Text();
}
```

---

## Encoding

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

### base64 / hex

```go
base64.StdEncoding.EncodeToString(b)
hex.EncodeToString(b)
```

```rust
base64::StdEncoding.EncodeToString(&b);
hex::EncodeToString(&b);
```

### CSV

```go
r := csv.NewReader(strings.NewReader("a,b\n1,2\n"))
records, _ := r.ReadAll()
```

```rust
let mut r = csv::NewReader(strings::NewReader("a,b\n1,2\n"));
let (records, _) = r.ReadAll();
```

---

## Networking

### HTTP server

```go
http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
    fmt.Fprintf(w, "hi %s", r.URL.Path)
})
log.Fatal(http.ListenAndServe(":8080", nil))
```

```rust
http::HandleFunc("/", |w, r| {
    Fprintf!(w, "hi %s", r.URL.Path);
});
log::Fatalf!("%s", http::ListenAndServe(":8080", nil));
```

### URL

```go
u, _ := url.Parse("https://x.com/p?q=1")
v := url.Values{}
v.Set("k", "v")
v.Encode()
```

```rust
let (u, _) = url::Parse("https://x.com/p?q=1");
let mut v = url::Values::new();
v.Set("k", "v");
v.Encode();
```

### netip

```go
addr, _ := netip.ParseAddr("fe80::1%eth0")
addr.Is6()
addr.Zone()
p, _ := netip.ParsePrefix("10.0.0.0/8")
p.Contains(netip.MustParseAddr("10.1.2.3"))
```

```rust
let (addr, _) = netip::ParseAddr("fe80::1%eth0");
addr.Is6();
addr.Zone();
let (p, _) = netip::ParsePrefix("10.0.0.0/8");
p.Contains(netip::MustParseAddr("10.1.2.3"));
```

### net/mail

```go
a, _ := mail.ParseAddress(`"Joe Q." <joe@x.com>`)
a.Name     // "Joe Q."
a.Address  // "joe@x.com"
```

```rust
let (a, _) = mail::ParseAddress(r#""Joe Q." <joe@x.com>"#);
a.Name;
a.Address;
```

### net/smtp

```go
c, _ := smtp.Dial("mail.example.com:25")
c.Hello("localhost")
c.Mail("from@x")
c.Rcpt("to@y")
w, _ := c.Data()
w.Write([]byte("Subject: hi\r\n\r\nHello."))
w.Close()
c.Quit()
```

```rust
let (mut c, _) = smtp::Dial("mail.example.com:25");
c.Hello("localhost");
c.Mail("from@x");
c.Rcpt("to@y");
let (mut w, _) = c.Data();
w.Write(b"Subject: hi\r\n\r\nHello.");
w.Close();
c.Quit();
```

---

## Multipart, cookies, MIME

### mime/multipart

```go
var b bytes.Buffer
w := multipart.NewWriter(&b)
part, _ := w.CreateFormFile("f", "a.txt")
part.Write(data)
w.WriteField("k", "v")
w.Close()
```

```rust
let mut b = bytes::Buffer::new();
let mut w = multipart::NewWriter(&mut b);
let (mut part, _) = w.CreateFormFile("f", "a.txt");
part.Write(&data);
w.WriteField("k", "v");
w.Close();
```

### net/http cookies

```go
c := &http.Cookie{
    Name:     "session",
    Value:    "abc",
    Path:     "/",
    HttpOnly: true,
    SameSite: http.SameSiteLaxMode,
}
cookies, _ := http.ParseCookie("a=1; b=2")
```

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

### net/textproto

```go
r := textproto.NewReader(bufio.NewReader(strings.NewReader(s)))
h, _ := r.ReadMIMEHeader()
textproto.CanonicalMIMEHeaderKey("user-agent")  // "User-Agent"
```

```rust
let mut r = textproto::NewReader(strings::NewReader(s));
let (h, _) = r.ReadMIMEHeader();
textproto::CanonicalMIMEHeaderKey("user-agent");
```

---

## Testing

Goish Rust ships its own test harness modelled on Go's. Tests live
in `tests/` as integration tests and are registered with the
`test!` macro.

### Basic test

```go
func TestClean(t *testing.T) {
    for _, test := range tests {
        s := path.Clean(test.path)
        if s != test.result {
            t.Errorf("Clean(%q) = %q, want %q",
                test.path, s, test.result)
        }
    }
}
```

```rust
test!{ fn TestClean(t) {
    for test in &tests() {
        let s = path::Clean(&test.path);
        if s != test.result {
            t.Errorf(Sprintf!("Clean(%q) = %q, want %q",
                test.path, s, test.result));
        }
    }
}}
```

### Table-driven tests

```go
type PathTest struct { path, result string }

var tests = []PathTest{
    {"",    "."},
    {"abc", "abc"},
}
```

```rust
Struct!{ type PathTest struct { path, result string } }

fn tests() -> slice<PathTest> { slice!([]PathTest{
    PathTest!("",    "."),
    PathTest!("abc", "abc"),
})}
```

### Benchmarks

```go
func BenchmarkJoin(b *testing.B) {
    b.ReportAllocs()
    for i := 0; i < b.N; i++ {
        path.Join("a", "b")
    }
}
```

```rust
benchmark!{ fn BenchmarkJoin(b) {
    b.ReportAllocs();
    while b.Loop() {
        path::Join(&slice!([]string{"a", "b"}));
    }
}}
```

### TestMain

Go's `TestMain` ports as `test_main!` with a custom harness. In a
`harness = false` test target, use `test_h!` (instead of `test!`)
and wire setup/teardown around `m.Run()`:

```rust
// tests/my_test.rs — registered with harness = false in Cargo.toml
use goish::prelude::*;

test_h!{ fn TestAddition(t) { if 2+2 != 4 { t.Error("math broke"); } } }

test_main!{ fn TestMain(m) {
    setup();
    let code = m.Run();
    teardown();
    os::Exit(code);
}}
```

---

## Generics-era helpers

Go 1.21+ added generic standard library packages for slices, maps,
iteration, and comparison. Goish Rust mirrors the same call sites.

### `cmp`

```go
cmp.Compare(1, 2)       // -1
cmp.Less(1.5, 2.5)      // true
cmp.Or(0, 0, 42)        // 42  (first non-zero)
```

```rust
cmp::Compare(1, 2);
cmp::Less(1.5, 2.5);
cmp::Or(&[0, 0, 42]);
```

### `slices`

```go
slices.Contains([]int{1,2,3}, 2)
slices.Index([]string{"a","b"}, "b")
slices.Sort(xs)
slices.SortFunc(xs, func(a, b int) int { return a - b })
slices.Reverse(xs)
slices.Min(xs); slices.Max(xs)
slices.BinarySearch(sorted, 42)
slices.Insert(xs, 1, 99, 100)
slices.Delete(xs, 1, 3)
slices.Compact(xs)
slices.Concat(a, b, c)
slices.Repeat(xs, 3)
slices.Clone(xs)
```

```rust
slices::Contains(&slice!([]int{1,2,3}), &2);
slices::Index(&slice!([]string{"a","b"}), &"b".to_string());
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

```go
keys := maps.Keys(m)
values := maps.Values(m)
maps.Equal(a, b)
cloned := maps.Clone(m)
maps.Copy(dst, src)
maps.DeleteFunc(m, func(k, v) bool { return v < 0 })
```

```rust
let keys = maps::Keys(&m);
let values = maps::Values(&m);
maps::Equal(&a, &b);
let cloned = maps::Clone(&m);
maps::Copy(&mut dst, &src);
maps::DeleteFunc(&mut m, |_k, v| *v < 0);
```

### `iter`

```go
// Go: range-over-func iteration
for v := range slices.Values(xs) { use(v) }
```

```rust
// Goish Rust: the Seq/Seq2 traits bridge to Rust iterators.
for v in iter::Values(&xs) { use_v(v); }
```

---

## Text toolkit

### `html`

```go
html.EscapeString("<a>&\"'")   // "&lt;a&gt;&amp;&#34;&#39;"
html.UnescapeString("&lt;a&gt;")
```

```rust
html::EscapeString("<a>&\"'");
html::UnescapeString("&lt;a&gt;");
```

### `text/tabwriter`

```go
w := tabwriter.NewWriter(os.Stdout, 0, 8, 1, '\t', 0)
fmt.Fprintln(w, "name\tage")
fmt.Fprintln(w, "alice\t30")
w.Flush()
```

```rust
let mut w = tabwriter::NewWriter(&mut os::Stdout(), 0, 8, 1, b'\t', 0);
Fprintf!(&mut w, "name\tage\n");
Fprintf!(&mut w, "alice\t30\n");
w.Flush();
```

### `text/scanner`

```go
var s scanner.Scanner
s.Init(strings.NewReader("x + 3.14"))
for tok := s.Scan(); tok != scanner.EOF; tok = s.Scan() {
    fmt.Println(s.TokenText())
}
```

```rust
let mut s = scanner::Scanner::new();
s.Init(strings::NewReader("x + 3.14"));
loop {
    let tok = s.Scan();
    if tok == scanner::EOF { break; }
    fmt::Println!(s.TokenText());
}
```

### `text/template`

```go
t := template.Must(template.New("t").Parse("Hi {{.Name}}!"))
t.Execute(os.Stdout, map[string]any{"Name": "alice"})
```

```rust
let (t, _) = template::New("t").Parse("Hi {{.Name}}!");
let mut out = String::new();
t.Execute(&mut out, &serde_json::json!({"Name": "alice"}));
```

---

## Crypto and hashing

### `crypto/md5` / `sha1` / `sha256`

```go
h := sha256.New()
h.Write([]byte("hello"))
sum := h.Sum(nil)
fmt.Printf("%x\n", sum)

// one-shot
sum = sha256.Sum256([]byte("hello"))
```

```rust
let mut h = sha256::New();
h.Write(b"hello");
let sum = h.Sum(&[]);
fmt::Printf!("%x\n", sum);

let sum = sha256::Sum256(b"hello");
```

### `hash/crc32` / `hash/fnv`

```go
crc32.ChecksumIEEE([]byte("abc"))
fnv.New32a().Sum32()
```

```rust
crc32::ChecksumIEEE(b"abc");
let mut h = fnv::New32a();
h.Sum32();
```

---

## Container types

### `container/list`

```go
l := list.New()
l.PushBack(1); l.PushBack(2); l.PushFront(0)
for e := l.Front(); e != nil; e = e.Next() {
    fmt.Println(e.Value)
}
```

```rust
let mut l = container::list::New::<int>();
l.PushBack(1); l.PushBack(2); l.PushFront(0);
let mut e = l.Front();
while let Some(node) = e {
    fmt::Println!(node.Value);
    e = node.Next();
}
```

### `container/heap`

```go
h := &IntHeap{2, 1, 5}
heap.Init(h)
heap.Push(h, 3)
min := heap.Pop(h).(int)
```

```rust
let mut h = container::heap::Heap::<int>::new();
h.Push(2); h.Push(1); h.Push(5); h.Push(3);
let min = h.Pop();
```

### `container/ring`

```go
r := ring.New(3)
for i := 0; i < r.Len(); i++ {
    r.Value = i
    r = r.Next()
}
```

```rust
let r = container::ring::New::<int>(3);
let mut cur = r;
for i in 0..3 {
    cur.SetValue(i);
    cur = cur.Next();
}
```

---

## Smaller packages

### `regexp`

```go
re := regexp.MustCompile(`^\d+$`)
re.MatchString("123")
re.FindString("abc 42 xyz")
re.ReplaceAllString("abc 42", "*")
```

```rust
let re = regexp::MustCompile(r"^\d+$");
re.MatchString("123");
re.FindString("abc 42 xyz");
re.ReplaceAllString("abc 42", "*");
```

### `flag`

```go
port := flag.Int("port", 8080, "listen port")
name := flag.String("name", "goish", "name")
flag.Parse()
```

```rust
let port = flag::Int("port", 8080, "listen port");
let name = flag::String("name", "goish", "name");
flag::Parse();
```

### `log`

```go
log.Println("server started")
log.Printf("port %d\n", 8080)
log.Fatalf("boom: %v", err)
```

```rust
log::Println!("server started");
log::Printf!("port %d\n", 8080);
log::Fatalf!("boom: %s", err);
```

### `math/rand`

```go
rand.Seed(42)
rand.Intn(100)
rand.Float64()
rand.Shuffle(len(xs), func(i, j int) { xs[i], xs[j] = xs[j], xs[i] })
```

```rust
rand::Seed(42);
rand::Intn(100);
rand::Float64();
rand::Shuffle(len!(xs), |i, j| xs.swap(i as usize, j as usize));
```

### `unicode` / `unicode/utf8`

```go
unicode.IsLetter('A')
unicode.ToUpper('a')
utf8.RuneCountInString("héllo")
utf8.ValidString("...")
```

```rust
unicode::IsLetter('A');
unicode::ToUpper('a');
utf8::RuneCountInString("héllo");
utf8::ValidString("...");
```

