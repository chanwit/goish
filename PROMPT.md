# Autonomous Mission: Seal All Rust Idiom Leaks in Goish

## North Star

Goish (`/home/chanwit/Dropbox/projects/goro-workspace/`) ports Go's stdlib + syntax to Rust. **Go idioms first — hide Rust at the library layer.** A user writing Goish should see Go shapes at the call site (`make!`, `slice<T>`, `string`, `error`, `c.Send(v)`, `if x, ok := …`), not Rust shapes (`Vec<T>`, `String::from_utf8`, `Arc<dyn Fn>`, `match Some/None`, turbofish, lifetimes). Internal `src/` code may use any Rust mechanism it likes; only the *boundary* matters.

When in doubt: "would a Go programmer recognize this line?" If no, it's a leak.

## Per-iteration loop

Every cycle:

1. **Audit** — read recent context (`git log --oneline -10` is canonical for what's shipped). Re-grep current leak counts: `format!`, `String::from_utf8`, `std::thread::spawn`, raw `AtomicXxx`, `Arc::new(...)`, `.to_string()`, `.iter().collect()`, `.unwrap()`/`.expect()`, `Box<dyn …>`, `Arc<dyn …>`, turbofish (`::<`), lifetimes (`'a`/`'static`) in public APIs, `REFERENCES.md` Rust syntax. Sweep `src/lib.rs` prelude for any new Rust-shape leaks.

2. **Plan** — pick the highest-impact bundle that's autonomous-safe (see Decision Framework). Bundles bundle naturally: 1–3 library items + optional tests/examples sweep. Don't over-scope a single iteration.

3. **Execute** — code, build, test. `cargo test` must pass at 1052+/0 failed before commit. `cargo build --tests --examples` must be clean.

4. **Ship** — bump `Cargo.toml` (patch for non-breaking, minor for breaking), commit, `git push origin main`, `git tag v0.X.Y`, `git push origin v0.X.Y`. **Never `cargo publish`.**

5. **Schedule next cycle** — `ScheduleWakeup` with the loop prompt, 1500–1800s delay. If Done Condition (below) is met, omit the wakeup to end the loop.

## Decision framework — make calls without asking

Where prior versions said "stop and ask," apply these instead:

- **Design with multiple valid options**: pick the option that most closely matches Go's surface syntax. Tiebreaker: pick the option that adds *less* surface area to Goish's public API (fewer new public types/macros). Document under a `Design choice:` line in the commit message so future iterations can reverse it cheaply if wrong.

- **Breaking API change**: allowed if (a) the lib change + tests/examples sweep ship in the **same bundle** so `cargo test` stays green, (b) version bump is appropriate (`0.minor` for an open public API, `0.major` once 1.0 lands), (c) commit message has a `Breaking:` section listing affected APIs and migration path.
  Auto-qualifies: comma-ok pass for `Option`-returning APIs, dropping `Option` middle-return from `bufio::Scan*`, return-type reshaping for cleanup.

- **Friction count fails to drop after a sweep**: investigate the skipped sites. If they're genuinely unmigrateable (`AtomicUsize` with no Goish wrapper, `format!` for Debug `{:?}`, `static` const-fn contexts, intentional benchmarks), document a one-line *wontfix rationale* in `REFERENCES.md` and exclude from the Done Condition mental model.

- **Two equally good bundles to ship next**: pick the smaller one. Bigger bundles wait for an iteration with more headroom.

- **Library change would touch >10 test files**: dispatch a background general-purpose subagent for the sweep with a tightly scoped prompt + exclusions; do library work in foreground.

- **Memory says user dislikes X**: respect it; pick alternatives.

- **Sweep migration breaks because the Goish wrapper lacks a trait the std type had** (e.g. `GoString` doesn't `impl ToSocketAddrs`, so `TcpStream::connect(&Sprintf!(...))` won't compile): **fix the library — add the missing impl. Do NOT revert the call site to the Rust idiom.** The library's job is to make Goish types interchangeable with the std types they replace at every reasonable boundary. `impl AsRef<...>`, `impl From<…>`, `impl Display`, `impl PartialEq<&str>`, `impl ToSocketAddrs`, `impl IntoIterator` etc. are all candidates. If the missing impl is genuinely impossible (e.g. orphan rule on a foreign trait + foreign type, or a sealed std trait), document it as a wontfix in REFERENCES.md §26 and *only then* revert the call site — never quietly drop a migration.

## Hard stops (non-negotiable)

- **`cargo publish`** — never run; only `git push` + `git tag`.
- **Destructive git ops on `main`** — no force-push, no `reset --hard`, no branch deletion. Always create new commits.
- **Modifying `.git/config`, hooks, CI workflows, or `~/.claude/`** without explicit instruction.
- **Touching files outside the project root**.
- **Lib build fails after 3 consecutive fix attempts** in one cycle — stop the loop, leave WIP uncommitted, schedule a wakeup with the failure reason in `ScheduleWakeup.reason` so the next iteration (or user) sees it.
- **A `Hard stop` in this list is in conflict with a Decision Framework rule** — Hard stop wins.

## Bundle types — discover applicable ones each cycle

Library bundles fall into these patterns; identify which apply this iteration:

- **Newtype seals** — wrap a public std-type leak (`Arc<dyn Fn>`, `Box<dyn Trait>`, `StdMutexGuard`, `std::io::*Lock`, `std::result::Result`) in an opaque Goish struct with delegating impls.
- **Signature widenings** — generic-ize public function inputs (`impl AsRef<[u8]>`, `impl Into<T>`, `impl Read + Send + 'static`) so callers don't need explicit conversions or boxing.
- **Comma-ok pass** — replace `Option<T>` returns with `(T, bool)` on container / error-recovery / lookup APIs to match Go's `if x, ok := …`.
- **Macro additions** — when turbofish or generic-bound burden has no clean signature fix, ship a Goish-shape macro (`slice!`, `map!`, `list!`, `if_as!`, `ctx_value!`).
- **Prelude pruning** — anything Rust-named in `src/lib.rs` prelude is a leak; mangle to `__Name` or `pub(crate)`.
- **Doc cleanup** — `REFERENCES.md` examples must read like Go (no turbofish, no lifetimes, no `Arc::new`, no `Box::new`).

Tests/examples sweeps fall into these patterns:

- **Pattern substitution** — `format!` → `Sprintf!`, `String::from_utf8` → `bytes::String`, `thread::spawn` → `go!{}`, raw atomics → `sync::atomic::*`, `.lock().unwrap()` → `sync::Mutex::Lock`, `Arc<Mutex<...>>` → `sync::Mutex`, `Box::new(closure)` → drop after lib widening.
- **Comma-ok migration** — after a comma-ok lib change, sweep `match Some/None` / `if let Some` → `if x, ok := …`.
- **Smart-pointer drop** — drop `Arc::clone` / `.clone()` where Goish wrappers handle internal sharing transparently.

## Workflow rules (must follow)

- One bundle per release. Bump `Cargo.toml` once. Two commits per bundle if work splits cleanly: lib + sweep.
- **Commit message format**:
  ```
  vX.Y.Z: <short summary>

  <bullet items by friction ID with one-paragraph rationale each>

  Design choice: <if applicable>
  Breaking: <if applicable, list APIs + migration path>

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  ```
- Push + tag at end of bundle: `git push origin main && git tag vX.Y.Z && git push origin vX.Y.Z`. **Never `cargo publish`** — match the v0.21.0+ pattern.
- Tests: 1052+/0 failed before every commit. `cargo build --tests --examples` clean.
- Backward-compat preferred. Widen via `impl Into<T>` / `impl AsRef<...>` / `Deref` newtype before reaching for breaking changes.
- When breaking change is needed, ship lib + sweep in the same bundle so the world stays buildable.

## Friction IDs

Sequential numbering #1+ across all sessions. Audits append at the next free number. Cite IDs in commit messages so cross-bundle context survives compaction.

To find the next free ID: `git log --all --oneline | grep -oE "#[0-9]+" | sort -V | tail -1`, then increment.

## Done condition

Mission "done" when an audit pass returns:

- `format!` at call sites: < 20 (only `{:?}` Debug or non-trivial padding)
- `String::from_utf8` at call sites: 0
- `std::thread::spawn` (test/example): < 5 (only intentional benchmarks)
- Raw `AtomicXxx` (test/example): only `AtomicUsize` or `static` contexts
- `Box<dyn …>` in public API: only inside `#[doc(hidden)]` fields
- `Arc<dyn …>` in public API: only inside `#[doc(hidden)]` fields
- Public APIs returning `Option<T>` for container/error recovery: 0 (all comma-ok)
- Visible turbofish or lifetimes in public function signatures (rustdoc): 0
- `REFERENCES.md`: zero Rust-only syntax in user-facing examples
- Unmigrateable sites: each documented in `REFERENCES.md` with one-line rationale
- `cargo test`: 1052+/0 failed
- `cargo build --tests --examples`: warning-clean for new code

When all met, on next cycle: post a final report (`mission complete: leak counts X/Y/Z, last release vA.B.C, see git log v0.21.0..HEAD for shipped bundles`) and **omit the `ScheduleWakeup` call** to end the loop.

## When to break out of the loop

- Done condition met (above) → final report + no wakeup.
- Hard stop tripped → leave WIP uncommitted, schedule wakeup with failure in `reason`.
- User intervenes via interrupt → respect their direction; don't re-schedule unless told.

## Working directory + memory

Working dir: `/home/chanwit/Dropbox/projects/goro-workspace/`. Auto-memory at `~/.claude/projects/-home-chanwit-Dropbox-projects-goro-workspace/memory/` — read `MEMORY.md` at cycle start for user-feedback context. Update memory only when learning a *new* user preference, not for routine work logging.
