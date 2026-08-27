# Changelog

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
