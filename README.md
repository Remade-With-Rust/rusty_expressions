# rusty_expressions

[![crates.io](https://img.shields.io/crates/v/rusty_expressions?logo=rust)](https://crates.io/crates/rusty_expressions)
[![docs.rs](https://img.shields.io/docsrs/rusty_expressions?logo=docsdotrs)](https://docs.rs/rusty_expressions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Remade With Rust](https://img.shields.io/badge/Remade%20With-Rust-000?logo=rust&logoColor=fff)](https://github.com/Remade-With-Rust)
[![By Mata Network](https://img.shields.io/badge/by-Mata%20Network-5b2be0)](https://www.mata.network)

> **rusty_expressions** is **Oniguruma, remade in pure Rust** — the engine
> behind Ruby, PHP `mb_ereg` and `jq`, rebuilt from its documented behaviour.
> Named groups, look-around, backreferences, subexp calls, atomic and
> possessive groups, absent expressions, conditionals, callouts, per-regex
> encodings and pluggable syntax dialects. **No C, no FFI, no `onig-sys`**,
> `no_std` + `alloc`, and it builds for `wasm32-unknown-unknown`.
>
> It is **match-equivalent to Oniguruma 6.9.10** — verified on a harvested
> corpus and on tens of thousands of differential cases run against live
> `libonig` — and about **3x faster than libonig on search**.

Part of **[Remade With Rust](https://github.com/Remade-With-Rust)** by
**[Mata Network](https://www.mata.network/)**.

---

## ⚡ The headline

**Oniguruma upstream is finished.** The C project was archived 2025-04-24. Its
last release carries Unicode 16.0 and years of OSS-Fuzz and Coverity fixes —
and there will be no further security patches, ever. A C copy in your tree is a
CVE surface you cannot close, and a `*-sys` crate is a C toolchain on every
machine you deploy to.

`rusty_expressions` is the replacement: the same documented behaviour, in safe
Rust, with the C deleted.

| | `onig` (FFI to libonig) | rust-lang `regex` | **rusty_expressions** |
|---|---|---|---|
| C in the dependency tree | all of it | none | **none** |
| Builds for `wasm32` | ✗ | ✓ | **✓** |
| Needs a C toolchain | ✓ | ✗ | **✗** |
| Backreferences | ✓ | ✗ *(by design)* | **✓** |
| Look-around | ✓ | ✗ *(by design)* | **✓** |
| Atomic / possessive | ✓ | ✗ | **✓** |
| Subexp call `\g<>` | ✓ | ✗ | **✓** |
| Absent expressions `(?~…)` | ✓ | ✗ | **✓** |
| Per-regex encodings | ✓ | ✗ (UTF-8 only) | **✓** |
| Syntax dialects | ✓ | ✗ | **✓** |
| Upstream still maintained | ✗ **archived** | ✓ | **✓** |

## 🏎 Performance — ~3x faster than the C engine

Measured against **live `libonig`** in the same process, 64 KB corpora,
ABBA-interleaved, medians of 11 rounds, with a null arm establishing the noise
floor (**0.0 % arm skew, 1–4 % p10–p90 spread**). Lower is better.

**`ours/onig = 0.32` overall — and there is no case where libonig wins.**

| category | case | ours | libonig | ratio |
|---|---|---:|---:|---:|
| structure | `[\w.]+@[\w.]+\.\w+` | 54 µs | 2430 µs | **0.02** |
| onig-only | `(?>\w+)=` atomic | 262 µs | 2859 µs | **0.09** |
| onig-only | `\d++ms` possessive | 114 µs | 1311 µs | **0.09** |
| onig-only | `\d+(?= ms)` look-ahead | 96 µs | 936 µs | **0.10** |
| class | `[#@%^&]+` (no match) | 26 µs | 231 µs | **0.11** |
| unicode | `\p{Greek}+` | 134 µs | 811 µs | **0.16** |
| structure | `\d+\.\d+\.\d+\.\d+` | 157 µs | 989 µs | **0.16** |
| capture | `(\w+)=(\w+)` | 398 µs | 2172 µs | **0.18** |
| capture | `(?<k>\w+)=(?<v>\w+)` | 502 µs | 2302 µs | **0.22** |
| unicode | `\p{Lu}+` | 236 µs | 993 µs | **0.24** |
| capture | `(\d{4})-(\d{2})-(\d{2})` | 127 µs | 461 µs | **0.28** |
| class | `[0-9]+` | 230 µs | 591 µs | **0.39** |
| class | `\w+` | 1201 µs | 2386 µs | **0.50** |
| onig-only | `(\w+) \1` backref | 1677 µs | 3132 µs | **0.54** |
| anchor | `(?m)^2026` | 78 µs | 135 µs | **0.58** |
| onig-only | `(?<=status=)\d+` | 1234 µs | 1898 µs | **0.65** |
| icase | `(?i)[a-z]+ing` | 1728 µs | 1905 µs | **0.91** |

**23 of 23 cases ours-faster or tied. Compile is ~2x faster too** (`0.48`).

### How

Nothing here changes a match — every optimization is a skip or a
precomputation that is *provably* unable to alter the result, which is why the
differential suite stays clean through all of them.

- **First-byte filter** — a 256-bit set built by walking the whole program
  (through `Save`, anchors, option pushes and look-around; unioned across
  alternation), not just the leading instruction.
- **Required-literal filter** — a byte sequence *every* match must contain,
  with its distance range. When it follows an unbounded class run that cannot
  match its first byte, the match must begin at or after the start of that run
  — so we find the literal and work backwards. `[\w.]+@[\w.]+\.\w+` went from
  **48 201 start positions to 242**.
- **Line-start filter** — `^`-anchored patterns only consider positions after
  a newline, fused with the literal check.
- **One engine per search**, not per start position, with inline capture
  storage: the hot path allocates nothing.
- **Compile-time tables** — class ASCII bitmaps (built under the options live
  at that point), repeat shapes, literal bytes, group spans.
- **Follow-literal-guided backtracking** — a greedy run only tries lengths
  that leave the required next byte in place; when the class cannot match that
  byte at all, only the maximal length is viable.
- **`rusty_alloc`** as the global allocator (default on) is worth a further
  ~21 %.

## ✅ Correctness — gated against the C engine, not against ourselves

The oracle is **live `libonig`**, never our own output. A remake gated against
itself gates in its own bugs.

| Gate | Coverage | Result |
|---|---|---|
| Harvested Oniguruma corpus | 50 vectors, match **and capture** offsets | **50/50**, and it flags any fixture that disagrees with libonig |
| Prefilter differential | 25 600 generated pattern × haystack pairs; every skip cross-checked against an unfiltered scan | **0** |
| Constructs differential | 21 500 pairs over atomic / look-around / absent / conditional / subexp-call | **0** |
| Context audit | 30 092 checks over 6 encodings, 6 option sets, 7 syntax dialects, user properties, capture spill, non-zero search starts | **0** |
| Callout verbs, subexp captures, class escapes, line anchors | targeted vs libonig | **0** |

Bugs these gates caught during development — each one invisible to a suite
that only checks whole-match ranges:

- `\x{...}` and `\xHH` silently dropped inside a character class
- `(*FAIL)` unimplemented and silently *succeeding* (a wrong match)
- `(*SKIP)` firing on the wrong pass
- `\g<n>` not writing its capture
- `(?m)^$` matching at end-of-string after a trailing newline
- a required-literal filter skipping past matches once a user property was
  registered

## 🔒 Safety and limits

- The engine is **`unsafe_code = "deny"`**. The only `unsafe` in the crate is
  the optional `compat` C ABI, which cannot be expressed safely.
- Oniguruma is a backtracking NFA and so is this — **we do not claim
  linear-time matching**. Limits are the mitigation, and they are **on by
  default**: `MatchParam` carries a match-stack depth limit and
  retry-in-match / retry-in-search limits. Hitting one is an `Err`, never a
  silent mismatch.
- A greedy repeat runs on the heap, not the call stack: `.+` over **1 MB**
  matches in ~13 ms rather than aborting.

## 📦 Install

```toml
[dependencies]
rusty_expressions = "0.1"

# A library that installs its own #[global_allocator]:
# rusty_expressions = { version = "0.1", default-features = false }
```

```rust
use rusty_expressions::{Encoding, Options, Regex, Syntax};

let re = Regex::new("ca+t", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA)?;
let m = re.search(b"one cat two")?.expect("match");
assert_eq!(m.range(), 4..7);          // byte offsets, as OnigRegion reports
# Ok::<(), rusty_expressions::Error>(())
```

Named groups, look-around and the rest work as Oniguruma documents them:

```rust
use rusty_expressions::{Options, Regex, Syntax};

let re = Regex::new_str(r"(?<key>\w+)=(?<val>\w+)", Options::NONE, Syntax::ONIGURUMA)?;
let m = re.search(b"path=api")?.expect("match");
assert_eq!(m.name("key"), Some(0..4));
# Ok::<(), rusty_expressions::Error>(())
```

### Features

| Feature | Default | What it does |
|---|---|---|
| `rusty-alloc` | **on** | installs `rusty_alloc` as the global allocator |
| `secure` | off | guard pages + encrypted free lists in the allocator |
| `compat` | off | Oniguruma-shaped C ABI (`onig_new` / `onig_search` / `OnigRegion`) — **pure Rust, no libonig** |
| `expr-count` | off | deterministic VM work counters for the benchmark harness |

## 🧭 When to use this, and when not to

**This is not a drop-in for [`regex`](https://crates.io/crates/regex).** They
are different complexity classes; swapping changes semantics *and* worst-case
runtime at once. Choose per call site:

| Situation | Use |
|---|---|
| Untrusted pattern **or** untrusted haystack on a hot path | **`regex`** — linear time, cannot be made to hang |
| Backreferences, look-around, `\g<>`, atomic / possessive, absent expressions | **`rusty_expressions`** |
| The match must agree with **Ruby / PHP / jq / Oniguruma** | **`rusty_expressions`** |
| Non-UTF-8 haystack — Shift_JIS, Big5, GB18030, EUC-*, UTF-16/32, ISO-8859-* | **`rusty_expressions`** |
| A non-default dialect — Perl, Python, Java, POSIX, GNU, Emacs, grep, ASIS | **`rusty_expressions`** |
| Anything already working on `regex` | **leave it** |

## 🗂 What is implemented

Encodings: ASCII, UTF-8, UTF-16BE/LE, UTF-32BE/LE, ISO-8859-1…16, KOI8-R,
CP1251, EUC-JP, EUC-TW, EUC-KR, EUC-CN, Shift_JIS, Big5, GB18030.

Syntaxes: Oniguruma (default), Perl, Perl_NG, Python, Java, POSIX basic /
extended, GNU regex, Emacs, grep, ASIS — plus user-built `Syntax` values with
variable meta-characters (SQL `%` / `_`).

Also: capture-history trees (`(?@…)`), `RegSet`, `scan` (find-all), a callout
seam with `(*FAIL)` / `(*MISMATCH)` / `(*SKIP)` / `(*COUNT)` / `(*ERROR)`,
user-defined Unicode properties, and Unicode 16.0 property tables generated
from the UCD.

## 🔬 Reproducing the numbers

The benchmark harness links crates.io `onig` (real libonig) purely as a test
oracle. It is **not** a dependency of this crate.

```sh
cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example suite
cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example prefilter_diff
cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example constructs_diff
cargo run --release --manifest-path tools/onig-bench/Cargo.toml --example audit
cargo run --release --manifest-path tools/onig-bench/Cargo.toml --example reqlit   # what the analyzer found
```

## 📄 License

MIT. Oniguruma itself is BSD-2; no Oniguruma source was copied — this is a
reimplementation from `doc/RE`, `doc/API`, `doc/SYNTAX.md` and harvested test
output.

## The Remade With Rust ecosystem

Pure-Rust replacements for the C libraries the world runs on, by
[Mata Network](https://www.mata.network/):
[remade_ffmpeg_rs](https://github.com/Remade-With-Rust/remade_ffmpeg_rs),
[rusty_h264](https://github.com/Remade-With-Rust/rusty_h264),
[FFAI](https://github.com/Remade-With-Rust/FFAI),
[rusty_alloc](https://github.com/Remade-With-Rust/rusty_alloc),
[spacedb](https://github.com/Remade-With-Rust/spacedb),
[mid](https://github.com/Remade-With-Rust/mid).
