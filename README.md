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
> `no_std` + `alloc` (with `default-features = false`), and it **runs** on
> `wasm32-unknown-unknown` — executed there, not merely compiled.
>
> It is **match-equivalent to Oniguruma 6.9.10** — verified on a harvested
> corpus and on **over 5 million differential cases** run against live
> `libonig`, plus **~380 000 property and C-ABI fuzz checks** — and **~3x
> faster than libonig on search**, winning all 23 cases in the benchmark
> suite. The twelve known differences are
> [written down](#known-differences-from-libonig), not rounded off.

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
| Runs on `wasm32` | ✗ | ✓ | **✓** |
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
floor (**0.0 % arm skew, 1 % p10-p90 spread**). Lower is better. Reproduced at
0.32 / 0.32 / 0.31 across three runs.

**`ours/onig = 0.32` overall — 23 of 23 cases ours-faster. libonig does not win
a single one.**

| category | case | ours | libonig | ratio |
|---|---|---:|---:|---:|
| structure | `[\w.]+@[\w.]+\.\w+` | 53 µs | 2644 µs | **0.02** |
| onig-only | `\d++ms` possessive | 100 µs | 1274 µs | **0.08** |
| onig-only | `\d+(?= ms)` look-ahead | 75 µs | 927 µs | **0.08** |
| onig-only | `(?>\w+)=` atomic | 242 µs | 2685 µs | **0.09** |
| class | `[#@%^&]+` (no match) | 29 µs | 235 µs | **0.12** |
| structure | `\d+\.\d+\.\d+\.\d+` | 150 µs | 976 µs | **0.15** |
| unicode | `\p{Greek}+` | 125 µs | 802 µs | **0.16** |
| capture | `(\w+)=(\w+)` | 398 µs | 2395 µs | **0.17** |
| unicode | `\p{Lu}+` | 190 µs | 925 µs | **0.21** |
| capture | `(?<k>\w+)=(?<v>\w+)` | 488 µs | 2345 µs | **0.21** |
| alt | `INFO\|WARN\|ERROR\|DEBUG` | 85 µs | 346 µs | **0.24** |
| capture | `(\d{4})-(\d{2})-(\d{2})` | 134 µs | 532 µs | **0.25** |
| literal | `fox` (find-all) | 28 µs | 77 µs | **0.36** |
| class | `[0-9]+` | 216 µs | 589 µs | **0.37** |
| structure | `https?://[\w./?=&-]+` | 47 µs | 106 µs | **0.45** |
| class | `\w+` | 1186 µs | 2518 µs | **0.47** |
| onig-only | `(\w+) ` backref | 1841 µs | 3429 µs | **0.54** |
| anchor | `(?m)^2026` | 81 µs | 143 µs | **0.56** |
| alt | `fox\|dog\|cat` | 172 µs | 300 µs | **0.57** |
| onig-only | `(?<=status=)\d+` | 1266 µs | 2070 µs | **0.61** |
| literal | `zzzqqq` (absent) | 19 µs | 26 µs | **0.75** |
| icase | `(?i)[a-z]+ing` | 1639 µs | 2126 µs | **0.77** |
| icase | `(?i)THE QUICK` | 350 µs | 405 µs | **0.87** |

Throughput peaks at **1.2 GB/s** (email extraction) and **2.2 GB/s** (a class
that cannot match). **Compile is ~2x faster too** (`0.50`).

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
| API property fuzz | ~100 000 checks over `MatchParam` limits, `RegSet`, `scan`, `search_range_param` | **0** |
| C ABI fuzz | 200 000 compiles and 280 000 searches through `onig_new`/`onig_search`/`onig_match` with null pointers, reversed spans, offsets past the end, and regions reused and double-freed | **0** |
| Oracle swap | 68 450 checks pairing our encodings, syntaxes and options with libonig'''s own | **12**, all one case, below |
| Randomized soak | 5 000 000 generated cases against live `libonig` (`ORACLE_SOAK=5000000`) | **0** |
| Miri | whole test suite, C ABI included | **no undefined behaviour** |
| wasm32 execution | battery run inside the sandbox under Node, not merely compiled | **0** |
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
- `scan` returning the same empty match repeatedly
- two denial-of-service aborts on untrusted *patterns*: deep nesting, and a
  long chain of quantifiers
- POSIX BRE groups `\(...\)` unimplemented, so they compiled to literal text
- `a*?` read as a literal `?` in the dialects where `?` is not the lazy marker
- CJK prefilters matching the ASCII byte inside a two-byte character
- `\w` in a CJK encoding asking Unicode about the transcoded codepoint
- undecodable UTF-8 raising instead of scanning, as libonig does
- `FIND_LONGEST` returning the first alternative rather than the longest
- `find_at` past the end of the haystack returning an empty match there

### Known differences from libonig

Twelve of the 68 450 oracle-swap checks differ, all the same case. In the EUC
encodings, on input that is **not valid** in that encoding, libonig can report
a match starting inside a character: its character walk treats `AD 61` in
EUC-JP as one two-byte character, but its literal byte-scan finds the `61` and
its `left_adjust_char_head` accepts that offset as a character head, so a bare
`a` matches there. We do not reproduce it — which patterns take that path
depends on libonig'''s internal optimiser, and reporting a match that begins
mid-character is the worse of the two answers. On well-formed input the two
agree everywhere.

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
- Pattern **nesting depth** and **VM recursion depth** are bounded too, so a
  hostile pattern gets an `Err`, never a process abort.

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

# Pair our encodings, syntaxes and options with libonig's own and diff everything:
cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example oracle_audit
# ...and turn the same generator into a soak:
ORACLE_SOAK=5000000 cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example oracle_audit

# The C ABI, driven as a careless C caller would:
cargo run --release --features compat --manifest-path tools/onig-bench/Cargo.toml --example fuzz_compat

# Run the engine inside a wasm32 sandbox, rather than trusting that it compiled:
cargo build --release --target wasm32-unknown-unknown --manifest-path tools/wasm-smoke/Cargo.toml
node tools/wasm-smoke/run.js

# No undefined behaviour, C ABI included:
cargo +nightly miri test --no-default-features --features compat --test expressions
```

The per-encoding character-class and character-length tables are generated
from libonig rather than transcribed by hand, and committed so an ordinary
build never needs the C library:

```sh
cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example gen_ctype  > src/enc_ctype.rs
cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example gen_mbclen > src/enc_mbclen.rs
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
