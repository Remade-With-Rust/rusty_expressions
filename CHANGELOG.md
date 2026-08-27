# Changelog

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
