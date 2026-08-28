# Changelog

## 0.2.0

The test oracle moved from a harvested corpus to **live libonig**, compared
across every encoding, syntax dialect and option we ship. That found 189
differences. Twelve remain, all one documented case.

A minor bump rather than a patch: several of these change what a pattern
matches, and one turns a previously-returned match into an error.

### Fixed -- matching

- **POSIX BRE groups were unimplemented.** `ESC_LPAREN_SUBEXP` was in the flag
  tables but nothing in the parser ever read it, so `\(a\)` compiled to the
  literal text `(a)`. Bare parens are now literals in that dialect, as POSIX
  says.
- **`posix_basic`, `posix_extended` and `grep` had wrong operator flags.**
  Replaced with libonig's own values.
- **`a*?` was read as a literal `?`** in the dialects where `?` is not the
  non-greedy marker. POSIX ERE and Emacs read it as `(a*)?`.
- **CJK prefilters matched inside a character.** ASCII bytes appear as trail
  bytes in the CJK encodings, so a byte scan for `a` matched the `61` in the
  Big5 character `AD 61`. The lead set, ASCII literal and required literal are
  now withheld unless the encoding is self-synchronising.
- **`\w` in a CJK encoding asked Unicode about the transcoded codepoint.**
  Oniguruma does not: a non-ASCII character is word/graph/print only if it is
  genuinely multi-byte. Shift_JIS halfwidth katakana is one byte, and `0xA0`
  was reading as U+00A0 and therefore as whitespace.
- **Undecodable UTF-8 raised instead of scanning.** Oniguruma treats such a
  byte as a one-byte character whose code sits above every real codepoint, so
  it matches `.` and `[^a]` but no character class.
- **`FIND_LONGEST` returned the first alternative, not the longest.** `a|aa`
  on `"aaa"` now matches `aa`.

### Fixed -- API

- **`Regex::find_at` accepted a position past the end of the haystack** and
  returned an empty match there, so `find_at(b"ab", 999)` reported `999..999`.
  It now returns `InvalidArgument`, as `search_range_param` always has. Found
  by the new C ABI fuzzer; a C caller would have sliced with those offsets.
  **This is the behaviour change that makes this a minor bump.**

### Added

- `Encoding::self_sync` -- whether a byte scan can locate character boundaries
  in this encoding.
- `Encoding::code_mbc_len` -- Oniguruma's `ONIGENC_CODE_TO_MBCLEN`.
- Per-encoding character-class and character-length tables, **generated from
  libonig** rather than transcribed, and committed so a build never needs the
  C library.

### Gates

- Oracle swap: 68 450 checks pairing our encodings, syntaxes and options with
  libonig's own -- 12 differences, all one case, documented in the crate docs
  and README.
- Randomized soak: **5 000 000** generated cases against live libonig, **0**
  differences (`ORACLE_SOAK=5000000`).
- C ABI fuzz: 200 000 compiles and 280 000 searches through `onig_new` /
  `onig_search` / `onig_match` with null pointers, reversed spans, offsets
  past the end and regions reused and double-freed -- **0** violations.
- Miri over the whole suite, C ABI included: **no undefined behaviour**.
- wasm32 **executed**, not merely compiled: a no_std cdylib runs a battery
  inside the sandbox under Node, 0 failures, no host imports.
- Cross-builds clean for aarch64/x86_64 Linux (gnu and musl), aarch64 macOS
  and Android.

### Release hardening

- **The declared MSRV was wrong.** `rust-version` said 1.73, but the default
  `rusty-alloc` feature pulls in `rusty_alloc-api`, which is edition 2024 and
  needs 1.85 -- so `cargo add rusty_expressions` on 1.73 failed with a
  dependency parse error rather than a clear MSRV error. Now declares 1.85,
  which is what the default build actually requires. The engine itself still
  builds on 1.73 with `default-features = false`, checked against that
  toolchain.
- `callout::builtin_skip` and `callout::describe` were written as public API
  with doc comments, but `callout` is a private module and only its types were
  re-exported -- nothing outside the crate could reach them. Now exported.
  `builtin_skip`'s doc described `(*COUNT)` while the function returns `Skip`.
- `RegSet` had a public `len` and no `is_empty`.
- Every `unsafe extern "C"` function in `compat` now documents its safety
  contract. A C ABI whose callers must uphold pointer invariants should say
  which ones.
- Zero build warnings in every feature configuration; the remaining dead code
  is annotated with why it is kept, so the next real one is visible.
- The generated character-length tables are now what `mbc_len` actually reads.
  They were generated, committed, and then left unused while hand-written
  ranges did the work -- and those ranges were wrong twice over: Big5
  validated a trail byte libonig does not, and EUC-TW shared EUC-JP's table,
  so its four-byte `0x8E` sequences were read as two.

### Documentation

- **`no_std` was overclaimed.** `default = ["rusty-alloc"]` and `rusty_alloc`
  depends on std, so the default build is not `no_std`. The claim now says it
  holds with `default-features = false`.
- The twelve known differences from libonig are written down rather than
  rounded off.
- `stress_repeat` no longer runs libonig on every case. `[a-z]+x` against a
  megabyte is quadratic with no match to find: we return `RetryLimitSearch` in
  88 ms, libonig ran over forty minutes before being killed. That arm is now
  opt-in, and the gate fails if our own arm exceeds a five-second budget.

## 0.1.4

- Depend on `rusty_alloc-api` 1.1.4 (was 0.4.0). Published so crates.io carries
  the current allocator; 0.1.3 still pins the old one.
- No engine change. Re-verified on the new allocator: 26 tests across three
  feature configurations, wasm32, 50/50 harvested vectors against live libonig,
  25_600 prefilter differential cases, 21_500 constructs cases, a 30_092-check
  context audit and 99_501 API property checks -- all clean.

## 0.1.3

Documentation only; no code change.

- Refreshed the measured performance figures against live libonig: `ours/onig`
  0.32 on search (reproduced 0.32 / 0.32 / 0.31), **23 of 23 benchmark cases
  ours-faster with none tied or lost**, and 0.50 on compile.
- Recorded the standing gate counts, including the API property fuzz over
  `MatchParam`, `RegSet`, `scan` and `search_range_param`.

## 0.1.2

Three more fixes, found by probing what the differential gates structurally
cannot see. Two are denial-of-service defects on untrusted *patterns*.

- **A deeply nested pattern aborted the process.** Parsing, compiling and the
  compile-time analysis are all recursive over pattern structure, so nesting
  depth is native call depth; `((((...))))` died around 400 levels. The parse
  depth limit existed but was set to 4096 -- above the ceiling it was meant to
  protect. Now 200, and nested character classes (which recursed outside the
  guarded path) are bounded too. Any depth now returns `ParseDepthLimit`.
- **A long chain of quantifiers aborted the process.** `a{1,2}a{1,2}...`
  recurses at match time because each repeat's continuation calls into the
  next -- flat in the pattern, deep on the stack. The retry counters could not
  catch it: they are counts, and the stack runs out long before a count limit
  sized for pathological backtracking fires. The VM now carries a real
  recursion-depth guard and returns `MatchStackLimit`.
- **Capture history kept entries from abandoned branches.** `hist` was pushed
  on a closing `Save` and never unwound, so a failed alternative left captures
  in the history tree that the match never made. History is now unwound
  wherever captures are.

## 0.1.1

Three correctness fixes, all found by a new property fuzz over `MatchParam`,
`RegSet`, `scan` and `search_range_param` (99_501 checks, now 0 violations).

- `scan` returned the **same empty match repeatedly**. It advanced past the
  scan cursor rather than past the match, so whenever the search skipped ahead
  to an empty match the cursor stayed behind it and re-found it. Affected
  `find_all` / `find_all_str` too.
- `search_range_param` **missed matches starting at exactly `range`**. The
  candidate-position scans were bounded at `end` when `pos == end` is a legal
  start, so every prefilter stopped one position early.
- `search_range_param` could **return a match starting past `range`**. The
  required-literal filter may legitimately find its literal beyond the last
  legal start; the resulting skip was not bounded by the range.

## 0.1.0

First release. Oniguruma 6.9.10 remade in pure Rust.

- Parse -> compile -> bytecode VM. `no_std` + `alloc`, `wasm32` checked, no C.
- Named groups, look-around, backreferences, subexp calls (`\g<>`), atomic and
  possessive groups, absent expressions, conditionals, capture-history trees,
  `\K` `\G` `\R` `\O` `\N` `\h`, text segments (`\X` `\y` `\Y`).
- Encodings: ASCII, UTF-8, UTF-16BE/LE, UTF-32BE/LE, ISO-8859-1..16, KOI8-R,
  CP1251, EUC-JP/TW/KR/CN, Shift_JIS, Big5, GB18030.
- Syntaxes: Oniguruma, Perl, Perl_NG, Python, Java, POSIX basic/extended,
  GNU regex, Emacs, grep, ASIS, plus user-built syntaxes with variable
  meta-characters.
- `scan` (find-all), `RegSet`, a callout seam with `(*FAIL)` / `(*MISMATCH)` /
  `(*SKIP)` / `(*COUNT)` / `(*ERROR)`, user-defined Unicode properties, and
  Unicode 16.0 property tables generated from the UCD.
- `MatchParam` limits (match-stack depth, retry-in-match, retry-in-search) are
  finite by default; hitting one is an error, never a silent mismatch.
- Optional `compat` feature: an Oniguruma-shaped C ABI in pure Rust.
- Verified match-equivalent to live libonig on a harvested corpus plus ~77_000
  differential cases; ~3x faster than libonig on search, ~2x on compile.
