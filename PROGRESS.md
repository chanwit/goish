# goish Port Progress

Tracks how much of the Go 1.25.5 standard library has been ported to
goish, per package. Numbers combine **API coverage** (how much of Go's
public surface is implemented) with **test-port coverage** (how many of
Go's own `*_test.go` cases have been brought over and pass verbatim).

Overall score = unweighted mean of per-package "Overall %". Packages
that goish does not target (unsafe, reflect, go/*, image, plugin, …)
are listed under **Out of scope** and excluded from the rollup.

## Overall: **64%**

_Last updated: v0.14.0 — 878 tests green._

## Ported packages (in rollup)

| Package | Go source | goish src | Tests ported | Impl % | Test % | **Overall** |
|---|---|---|---|---|---|---|
| **Builtins** (chan / defer / go / range / select / close / len / make / append / delete / map / slice) | – | `src/{chan,defer,range,types,struct_macro}.rs` | 27 (chan + runtime + select semantics) | 85% | 65% | **75%** |
| `cmp` | `src/cmp` | `src/cmp.rs` | 6 | 85% | 75% | **80%** |
| `slices` | `src/slices` | `src/slices.rs` | 14 | 70% | 65% | **70%** |
| `maps` | `src/maps` | `src/maps.rs` | 6 | 80% | 70% | **75%** |
| `iter` | `src/iter` | `src/iter.rs` | 5 | 60% | 55% | **60%** |
| `html` | `src/html` | `src/html.rs` | 5 | 40% | 55% | **50%** |
| `text/tabwriter` | `src/text/tabwriter` | `src/text/tabwriter.rs` | 5 | 50% | 55% | **55%** |
| `text/scanner` | `src/text/scanner` | `src/text/scanner.rs` | 7 | 40% | 55% | **50%** |
| `text/template` | `src/text/template` | `src/text/template.rs` | 10 | 30% | 50% | **40%** |
| `bufio` | `src/bufio` | `src/bufio.rs` | 17 | 70% | 40% | **55%** |
| `bytes` | `src/bytes` | `src/bytes.rs` | 4 | 55% | 30% | **40%** |
| `container/heap` + `container/list` + `container/ring` | `src/container/*` | `src/container/*.rs` | 20 | 70% | 60% | **65%** |
| `context` | `src/context` | `src/context.rs` | 10 | 80% | 55% | **70%** |
| `crypto/md5` | `src/crypto/md5` | `src/crypto/md5.rs` | 3 | 80% | 65% | **75%** |
| `crypto/sha1` | `src/crypto/sha1` | `src/crypto/sha1.rs` | 3 | 80% | 65% | **75%** |
| `crypto/sha256` | `src/crypto/sha256` | `src/crypto/sha256.rs` | 3 | 80% | 65% | **75%** |
| `encoding/base64` | `src/encoding/base64` | `src/encoding/base64.rs` | 9 | 70% | 60% | **65%** |
| `encoding/binary` | `src/encoding/binary` | `src/encoding/binary.rs` | 9 | 50% | 55% | **55%** |
| `encoding/csv` | `src/encoding/csv` | `src/encoding/csv.rs` | 8 | 60% | 55% | **60%** |
| `encoding/hex` | `src/encoding/hex` | `src/encoding/hex.rs` | 5 | 75% | 65% | **70%** |
| `encoding/json` | `src/encoding/json` | `src/encoding/json.rs` | 28 | 55% | 35% | **45%** |
| `errors` | `src/errors` | `src/errors.rs` | 8 (in fmt_errors) | 85% | 50% | **70%** |
| `flag` | `src/flag` | `src/flag.rs` | 0 | 60% | 0% | **30%** |
| `fmt` | `src/fmt` | `src/fmt.rs` | ~30 | 65% | 30% | **50%** |
| `hash/crc32` | `src/hash/crc32` | `src/hash/crc32.rs` | 3 | 70% | 55% | **65%** |
| `hash/fnv` | `src/hash/fnv` | `src/hash/fnv.rs` | 6 | 80% | 65% | **75%** |
| `io` | `src/io` | `src/io.rs` | 19 | 80% | 65% | **75%** |
| `log` | `src/log` | `src/log.rs` | 0 | 40% | 0% | **20%** |
| `math` | `src/math` | `src/math/mod.rs` | 10 | 35% | 55% | **45%** |
| `math/rand` | `src/math/rand` | `src/math/rand.rs` | 7 | 60% | 55% | **60%** |
| `mime` | `src/mime` | `src/mime.rs` | 0 | 40% | 0% | **20%** |
| `net/http` | `src/net/http` | `src/net/http/*.rs` | 23 | 55% | 35% | **45%** |
| `net/url` | `src/net/url` | `src/net/url.rs` | 14 | 75% | 55% | **65%** |
| `os` | `src/os` | `src/os/mod.rs` | 7 | 60% | 35% | **50%** |
| `os/exec` | `src/os/exec` | `src/os/exec.rs` | 0 | 55% | 0% | **30%** |
| `path` | `src/path` | `src/path/mod.rs` | 6 | 85% | 70% | **80%** |
| `path/filepath` | `src/path/filepath` | `src/path/filepath.rs` | 12 | 65% | 55% | **60%** |
| `regexp` | `src/regexp` | `src/regexp.rs` | 15 | 70% | 45% | **60%** |
| `runtime` | `src/runtime` | `src/runtime.rs` | 5 (lib) | 20% | 20% | **20%** |
| `sort` | `src/sort` | `src/sort.rs` | 9 | 65% | 55% | **60%** |
| `strconv` | `src/strconv` | `src/strconv.rs` | 21 | 85% | 70% | **80%** |
| `strings` | `src/strings` | `src/strings.rs` | 26 | 80% | 65% | **75%** |
| `sync` | `src/sync` | `src/sync/mod.rs` | 11 | 65% | 45% | **55%** |
| `sync/atomic` | `src/sync/atomic` | `src/sync/atomic.rs` | 12 | 70% | 60% | **65%** |
| `testing` | `src/testing` | `src/testing.rs` | (self-hosted) | 60% | – | **40%** |
| `time` | `src/time` | `src/time.rs` | 61 | 85% | 70% | **80%** |
| `unicode` | `src/unicode` | `src/unicode/mod.rs` | 8 | 30% | 55% | **45%** |
| `unicode/utf8` | `src/unicode/utf8` | `src/unicode/utf8.rs` | 6 | 75% | 65% | **70%** |

**Count: 48 packages ported, mean 62%.**

## Not yet ported — scheduled on milestones

Tracked on [GitHub milestones v0.11–v0.19](https://github.com/chanwit/goish/milestones).
Each bullet has a tracker issue + per-file porting issues.

- **v0.15.0** (networking depth) — `net/{netip,mail,smtp,textproto}`,
  plus `net/http` multipart + cookies.
- **v0.16.0** (compression + archive) — `compress/{flate,gzip,zlib}`,
  `archive/{tar,zip}`.
- **v0.17.0** (crypto completeness) — `crypto/{aes,cipher,rand,hmac,
  ed25519,rsa,sha512}`.
- **v0.18.0** (encoding completeness) — `encoding/{asn1,base32,gob,pem,
  xml}`, `hash/{adler32,crc64,maphash}`.
- **v0.19.0** (math depth) — `math/{big,bits,cmplx}`, `unicode/utf16`.

## Not yet ported — long tail

Portable but not scheduled. Most could land with user demand; open an
issue if you need one.

- `compress/{bzip2,lzw}` — `flate/gzip/zlib` cover the 95% case.
- `crypto/{des,dsa,ecdsa,ecdh,elliptic,hkdf,hpke,mlkem,pbkdf2,rc4,rsa,
  sha3,subtle,tls,x509}` — lower demand after v0.17 lands the
  widely-used subset.
- `database/{sql,driver}` — multi-driver abstraction with its own
  ecosystem.
- `encoding/ascii85` — rarely used.
- `net/rpc` — Go-specific wire format.
- `os/{signal,user}` — doable; platform-specific surface.
- `time/tzdata` — `chrono-tz` is the Rust idiom.
- `html/template` — contextual-escape engine coupled to HTML parsing.

## Excluded from scope — honest categorisation

Earlier drafts lumped everything goish won't ship under a single
"out of scope" heading. That hid the fact that most of those packages
are portable in principle; they're just deprioritised. This section
splits the exclusion into four honest buckets.

### A. Genuinely incompatible with goish's premise

Goish's premise is *static Rust types with Go-shaped syntax*. These
three packages fight that premise and a port would be a leaky fake:

- **`reflect`** — Go's reflection works because every value carries
  runtime type metadata. Building that into goish means tagging every
  value and wrapping every API — the opposite of "idiomatic Rust
  under the hood". Crates like `bevy_reflect` prove it's possible,
  but a goish version would contradict the project's design.
- **`unsafe`** — Rust already has the `unsafe` keyword with different
  semantics from `unsafe.Pointer`. Wrapping one as the other confuses
  both Go and Rust readers.
- **`builtin`** — Not a package; documentation of `int`, `len`, `make`,
  `append`, etc. Goish implements these via `types.rs` and the macro
  prelude, so the package itself has no porting target.

### B. Would require reimplementing large Go tooling

Portable in principle but the effort budget is prohibitive and the
goish audience (Rust devs writing Go-flavored code) doesn't need them:

- **`go/{ast,parser,types,token,scanner,format,printer,doc,build,constant,importer,version}`** —
  a Go parser + type checker in Rust is ~50k LoC. Won't do.
- **`plugin`** — Go's runtime linker needs .so symbol tables in a
  Go-specific layout. `libloading` works for arbitrary shared objects
  but the API shape doesn't map cleanly. Won't do.

### C. Better served by Rust's existing ecosystem

Portable, but duplicating well-funded Rust crates would be busywork:

- **`image/{color,draw,gif,jpeg,png}`** — the `image` crate is ubiquitous.
- **`debug/{dwarf,elf,macho,pe,plan9obj,gosym,buildinfo}`** — `object`
  + `gimli` + `goblin` cover this.
- **`syscall`** — `libc` and `std::os::{unix,windows}` already exist;
  Go's own `syscall` is deprecated in favor of `golang.org/x/sys`.
- **`simd`** — `std::simd` and `core::arch` are Rust's idiomatic
  intrinsics.
- **`index/suffixarray`** — `suffix` crate exists.

### D. Honestly deprioritised — could port, not planning to

These are portable and the code isn't hard. They're on the "nice to
have, not scheduled" list:

- **`embed`** — achievable via a macro wrapping `include_bytes!` +
  a simulated `embed.FS`. Likely v0.20+.
- **`weak`** — wraps `std::sync::Weak`; ~50 lines of glue.
- **`unique`** — simple HashMap-backed value interner.
- **`structs`** — marker types only; a doc-only mapping is trivial.
- **`testing/{quick,fstest,iotest,slogtest,synctest,cryptotest}`** —
  all portable. `testing/quick` (property-based testing) is the
  most useful. Deprioritised because goish users write application
  code, not test infrastructure.
- **`expvar`** — exports `/debug/vars`; could wrap atomics + net/http.
- **`log/slog`, `log/syslog`** — structured logging and syslog client.
- **`runtime/{debug,metrics,pprof,trace}`** — runtime diagnostics;
  partial mapping onto Rust profilers possible.

If you want any of bucket D sooner, open an issue and I'll move it
into a milestone.

## Per-version rollup

| Milestone | Theme | Packages touched | Tests added | Status |
|---|---|---|---|---|
| v0.4.0 | testing framework | `testing` | – | ✅ |
| v0.5.0 | net/http, TestMain | `net/http` | – | ✅ |
| v0.6.0 | foundations | `fmt`, `strings`, `strconv` | ~100 | ✅ |
| v0.7.0 | dates + data | `time`, `encoding/json` | 89 | ✅ |
| v0.8.0 | I/O + OS | `io`, `bufio`, `os`, `path/filepath` | 60 | ✅ |
| v0.9.0 | networking + regexp | `net/url`, `regexp`, `net/http` | 52 | ✅ |
| v0.10.0 | concurrency deep dive | `sync`, `sync/atomic`, `context`, `runtime/chan` | 41 | ✅ |
| v0.10.1 | select! proc-macro rewrite | `chan/select` — 5 CSP-derived bug fixes | 5 | ✅ |
| v0.11.0 | crypto + encoding + hash | `crypto/{md5,sha1,sha256}`, `encoding/{base64,binary,csv,hex}`, `hash/{crc32,fnv}` | 49 | ✅ |
| v0.12.0 | sort + container + math + unicode | `sort`, `math`, `math/rand`, `unicode`, `unicode/utf8`, `container/{heap,list,ring}` + new `container/ring` impl | 65 | ✅ |
| v0.13.0 | generics-era helpers | `cmp`, `slices`, `maps`, `iter` (all new impls + tests) | 48 | ✅ |
| v0.14.0 | text toolkit | `html`, `text/{tabwriter,scanner,template}` (all new impls + tests) | 45 | ✅ |
| v0.11.0 | crypto + encoding | `crypto/{md5,sha1,sha256}`, `encoding/{base64,binary,csv,hex}`, `hash/{crc32,fnv}` | – | ⏳ planned |
| v0.12.0 | sort + container + math + unicode | `sort`, `container/*`, `math`, `math/rand`, `unicode`, `unicode/utf8` | – | ⏳ planned |
| v0.13.0 | generics-era helpers | `slices`, `maps`, `cmp`, `iter` | – | 📋 |
| v0.14.0 | text toolkit | `text/{template,tabwriter,scanner}`, `html` | – | 📋 |
| v0.15.0 | networking depth | `net/{mail,smtp,netip,textproto}`, `net/http` multipart, cookies | – | 📋 |
| v0.16.0 | compression + archive | `compress/{flate,gzip}`, `archive/{tar,zip}` | – | 📋 |
| v0.17.0 | crypto completeness | `crypto/{aes,cipher,rand,hmac,ed25519,rsa}`, `crypto/sha512` | – | 📋 |
| v0.18.0 | encoding completeness | `encoding/{asn1,base32,gob,pem,xml}`, `hash/{adler32,crc64,maphash}` | – | 📋 |
| v0.19.0 | math + big numbers | `math/{big,bits,cmplx}`, `unicode/utf16` | – | 📋 |
| v1.0.0 | stabilisation | – | – | 🏁 |

## How "Overall %" is calculated

For each ported package:

```
Overall = round( (Impl% + Test%) / 2 )
```

where:

- **Impl %** — what fraction of the Go public functions, types, and
  methods exist on goish's package. A best-effort estimate from code
  review.
- **Test %** — what fraction of Go's `*_test.go` test *functions* for
  that package have been ported to `tests/` and pass. Exact count
  where possible; estimate where Go's table-driven tests were only
  partially ported.

For the project-wide number at the top, we take the unweighted mean of
per-package Overall across the 40 rows in the "Ported packages" table.
Packages in **Not yet ported** count as 0% when included in a rollup
for a future milestone; they are excluded from the _current_ overall
to keep the number reflective of what's shipped.

Packages in the four **Excluded** buckets (genuinely incompatible,
tooling-too-large, better-served-by-Rust, deprioritised) never enter
the rollup. Including them would deflate the number to 20-something %
and misrepresent how close goish is to covering the Go that's worth
covering.

## See also

- [goish on crates.io](https://crates.io/crates/goish)
- [Milestones on GitHub](https://github.com/chanwit/goish/milestones)
- [Go 1.25.5 source](https://github.com/golang/go/tree/go1.25.5/src)
