# goish Port Progress

Tracks how much of the Go 1.25.5 standard library has been ported to
goish, per package. Numbers combine **API coverage** (how much of Go's
public surface is implemented) with **test-port coverage** (how many of
Go's own `*_test.go` cases have been brought over and pass verbatim).

Overall score = unweighted mean of per-package "Overall %". Packages
that goish does not target (unsafe, reflect, go/*, image, plugin, …)
are listed under **Out of scope** and excluded from the rollup.

## Overall: **53%**

_Last updated: v0.10.0 — 666 tests green._

## Ported packages (in rollup)

| Package | Go source | goish src | Tests ported | Impl % | Test % | **Overall** |
|---|---|---|---|---|---|---|
| **Builtins** (chan / defer / go / range / select / close / len / make / append / delete / map / slice) | – | `src/{chan,defer,range,types,struct_macro}.rs` | 22 (chan + runtime) | 85% | 55% | **70%** |
| `bufio` | `src/bufio` | `src/bufio.rs` | 17 | 70% | 40% | **55%** |
| `bytes` | `src/bytes` | `src/bytes.rs` | 4 | 55% | 30% | **40%** |
| `container/heap` + `container/list` | `src/container/*` | `src/container/*.rs` | 0 | 60% | 0% | **30%** |
| `context` | `src/context` | `src/context.rs` | 10 | 80% | 55% | **70%** |
| `crypto/md5` | `src/crypto/md5` | `src/crypto/md5.rs` | 0 | 80% | 0% | **40%** |
| `crypto/sha1` | `src/crypto/sha1` | `src/crypto/sha1.rs` | 0 | 80% | 0% | **40%** |
| `crypto/sha256` | `src/crypto/sha256` | `src/crypto/sha256.rs` | 0 | 80% | 0% | **40%** |
| `encoding/base64` | `src/encoding/base64` | `src/encoding/base64.rs` | 0 | 70% | 0% | **35%** |
| `encoding/binary` | `src/encoding/binary` | `src/encoding/binary.rs` | 0 | 50% | 0% | **25%** |
| `encoding/csv` | `src/encoding/csv` | `src/encoding/csv.rs` | 0 | 60% | 0% | **30%** |
| `encoding/hex` | `src/encoding/hex` | `src/encoding/hex.rs` | 0 | 75% | 0% | **40%** |
| `encoding/json` | `src/encoding/json` | `src/encoding/json.rs` | 28 | 55% | 35% | **45%** |
| `errors` | `src/errors` | `src/errors.rs` | 8 (in fmt_errors) | 85% | 50% | **70%** |
| `flag` | `src/flag` | `src/flag.rs` | 0 | 60% | 0% | **30%** |
| `fmt` | `src/fmt` | `src/fmt.rs` | ~30 | 65% | 30% | **50%** |
| `hash/crc32` | `src/hash/crc32` | `src/hash/crc32.rs` | 0 | 70% | 0% | **35%** |
| `hash/fnv` | `src/hash/fnv` | `src/hash/fnv.rs` | 0 | 80% | 0% | **40%** |
| `io` | `src/io` | `src/io.rs` | 19 | 80% | 65% | **75%** |
| `log` | `src/log` | `src/log.rs` | 0 | 40% | 0% | **20%** |
| `math` | `src/math` | `src/math/mod.rs` | 0 | 35% | 0% | **20%** |
| `math/rand` | `src/math/rand` | `src/math/rand.rs` | 0 | 60% | 0% | **30%** |
| `mime` | `src/mime` | `src/mime.rs` | 0 | 40% | 0% | **20%** |
| `net/http` | `src/net/http` | `src/net/http/*.rs` | 23 | 55% | 35% | **45%** |
| `net/url` | `src/net/url` | `src/net/url.rs` | 14 | 75% | 55% | **65%** |
| `os` | `src/os` | `src/os/mod.rs` | 7 | 60% | 35% | **50%** |
| `os/exec` | `src/os/exec` | `src/os/exec.rs` | 0 | 55% | 0% | **30%** |
| `path` | `src/path` | `src/path/mod.rs` | 6 | 85% | 70% | **80%** |
| `path/filepath` | `src/path/filepath` | `src/path/filepath.rs` | 12 | 65% | 55% | **60%** |
| `regexp` | `src/regexp` | `src/regexp.rs` | 15 | 70% | 45% | **60%** |
| `runtime` | `src/runtime` | `src/runtime.rs` | 5 (lib) | 20% | 20% | **20%** |
| `sort` | `src/sort` | `src/sort.rs` | 0 | 65% | 0% | **35%** |
| `strconv` | `src/strconv` | `src/strconv.rs` | 21 | 85% | 70% | **80%** |
| `strings` | `src/strings` | `src/strings.rs` | 26 | 80% | 65% | **75%** |
| `sync` | `src/sync` | `src/sync/mod.rs` | 11 | 65% | 45% | **55%** |
| `sync/atomic` | `src/sync/atomic` | `src/sync/atomic.rs` | 12 | 70% | 60% | **65%** |
| `testing` | `src/testing` | `src/testing.rs` | (self-hosted) | 60% | – | **40%** |
| `time` | `src/time` | `src/time.rs` | 61 | 85% | 70% | **80%** |
| `unicode` | `src/unicode` | `src/unicode/mod.rs` | 0 | 30% | 0% | **15%** |
| `unicode/utf8` | `src/unicode/utf8` | `src/unicode/utf8.rs` | 0 | 75% | 0% | **40%** |

**Count: 40 packages ported, mean 53%.**

## Not yet ported

These are in Go stdlib but absent from goish (0% across the board):

- `archive/tar`, `archive/zip`
- `cmp` (generic ordering helpers — Go 1.21+)
- `compress/{flate,gzip,bzip2,zlib,lzw}`
- `container/ring`
- `crypto/{aes,cipher,des,dsa,ecdsa,ecdh,ed25519,elliptic,hkdf,hmac,mlkem,pbkdf2,rand,rc4,rsa,sha3,sha512,subtle,tls,x509}`
- `database/{sql,driver}`
- `encoding/{ascii85,asn1,base32,gob,pem,xml}`
- `expvar`
- `hash/{adler32,crc64,maphash}`
- `html`, `html/template`
- `iter` (iterator helpers — Go 1.23+)
- `log/slog`, `log/syslog`
- `maps` (generic map helpers — Go 1.21+)
- `math/{big,bits,cmplx}`
- `net/{mail,smtp,netip,rpc,textproto}`
- `os/{signal,user}`
- `runtime/{debug,metrics,pprof,trace}`
- `slices` (generic slice helpers — Go 1.21+)
- `text/{scanner,tabwriter,template}`
- `time/tzdata`
- `unicode/utf16`
- `unique` (value canonicalisation — Go 1.23+)

## Out of scope

These Go packages do not port meaningfully to Rust:

- `builtin` — Go's predeclared identifiers (we do these via `prelude::*`).
- `debug/{dwarf,elf,macho,pe,plan9obj,gosym,buildinfo}` — platform binaries; tooling-only.
- `embed` — requires Go's `go build` magic.
- `go/{ast,parser,types,…}` — Go's own syntax tools; not portable.
- `image/{color,draw,gif,jpeg,png}` — better served by Rust's `image` crate.
- `index/suffixarray` — niche algorithm; not Go-specific.
- `plugin` — OS-specific shared object loading.
- `reflect` — Rust has static types; no runtime reflection.
- `simd` — architecture-specific intrinsics.
- `structs` — Go-specific struct field markers.
- `syscall` — low-level OS ABI; Rust has its own.
- `testing/{cryptotest,fstest,iotest,quick,slogtest,synctest}` — Go's internal test helpers.
- `unsafe` — Rust has `unsafe` already.
- `weak` — Go-specific weak pointers.

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

## See also

- [goish on crates.io](https://crates.io/crates/goish)
- [Milestones on GitHub](https://github.com/chanwit/goish/milestones)
- [Go 1.25.5 source](https://github.com/golang/go/tree/go1.25.5/src)
