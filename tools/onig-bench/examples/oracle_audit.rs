//! The audit, re-pointed at **libonig** instead of our own engine.
//!
//! cargo run --release --features oracle --manifest-path tools/onig-bench/Cargo.toml --example oracle_audit
//!
//! `audit.rs` uses our own `find_at` as ground truth, which means a bug in
//! `find_at` would be agreed with rather than caught. This runs the same
//! contexts -- encodings, syntaxes, options -- against the C engine, so the
//! evidence is independent of us.
use rusty_expressions as rx;

// ---------------------------------------------------------------------------
// Encoding pairing: ours <-> libonig's
// ---------------------------------------------------------------------------

struct Enc {
    name: &'static str,
    ours: rx::Encoding,
    theirs: onig_sys::OnigEncoding,
}

fn encodings() -> Vec<Enc> {
    macro_rules! e {
        ($n:expr, $o:expr, $t:ident) => {
            Enc {
                name: $n,
                ours: $o,
                theirs: unsafe { &mut onig_sys::$t },
            }
        };
    }
    vec![
        e!("UTF-8", rx::Encoding::UTF8, OnigEncodingUTF8),
        e!("ASCII", rx::Encoding::ASCII, OnigEncodingASCII),
        e!("ISO-8859-1", rx::Encoding::ISO_8859_1, OnigEncodingISO_8859_1),
        e!("ISO-8859-2", rx::Encoding::ISO_8859_2, OnigEncodingISO_8859_2),
        e!("ISO-8859-5", rx::Encoding::ISO_8859_5, OnigEncodingISO_8859_5),
        e!("ISO-8859-15", rx::Encoding::ISO_8859_15, OnigEncodingISO_8859_15),
        e!("KOI8-R", rx::Encoding::KOI8_R, OnigEncodingKOI8_R),
        e!("CP1251", rx::Encoding::CP1251, OnigEncodingCP1251),
        e!("Shift_JIS", rx::Encoding::SJIS, OnigEncodingSJIS),
        e!("Big5", rx::Encoding::BIG5, OnigEncodingBIG5),
        e!("EUC-JP", rx::Encoding::EUC_JP, OnigEncodingEUC_JP),
        e!("EUC-KR", rx::Encoding::EUC_KR, OnigEncodingEUC_KR),
        e!("EUC-TW", rx::Encoding::EUC_TW, OnigEncodingEUC_TW),
        e!("EUC-CN", rx::Encoding::EUC_CN, OnigEncodingEUC_CN),
        e!("GB18030", rx::Encoding::GB18030, OnigEncodingGB18030),
        e!("UTF-16BE", rx::Encoding::UTF16_BE, OnigEncodingUTF16_BE),
        e!("UTF-16LE", rx::Encoding::UTF16_LE, OnigEncodingUTF16_LE),
    ]
}

fn syntaxes() -> Vec<(&'static str, rx::Syntax, &'static onig::Syntax)> {
    vec![
        ("oniguruma", rx::Syntax::ONIGURUMA, onig::Syntax::oniguruma()),
        ("perl", rx::Syntax::perl(), onig::Syntax::perl()),
        ("perl_ng", rx::Syntax::perl_ng(), onig::Syntax::perl_ng()),
        ("python", rx::Syntax::python(), onig::Syntax::python()),
        ("java", rx::Syntax::java(), onig::Syntax::java()),
        ("posix_basic", rx::Syntax::posix_basic(), onig::Syntax::posix_basic()),
        ("posix_ext", rx::Syntax::posix_extended(), onig::Syntax::posix_extended()),
        ("gnu", rx::Syntax::gnu_regex(), onig::Syntax::gnu_regex()),
        ("emacs", rx::Syntax::emacs(), onig::Syntax::emacs()),
        ("grep", rx::Syntax::grep(), onig::Syntax::grep()),
        ("asis", rx::Syntax::ASIS, onig::Syntax::asis()),
    ]
}

/// Option pairs that both engines understand identically.
fn option_sets() -> Vec<(&'static str, rx::Options, onig::RegexOptions)> {
    use onig::RegexOptions as O;
    vec![
        ("none", rx::Options::NONE, O::REGEX_OPTION_NONE),
        ("icase", rx::Options::IGNORECASE, O::REGEX_OPTION_IGNORECASE),
        ("extend", rx::Options::EXTEND, O::REGEX_OPTION_EXTEND),
        ("multiline", rx::Options::MULTILINE, O::REGEX_OPTION_MULTILINE),
        ("singleline", rx::Options::SINGLELINE, O::REGEX_OPTION_SINGLELINE),
        (
            "find_longest",
            rx::Options::FIND_LONGEST,
            O::REGEX_OPTION_FIND_LONGEST,
        ),
        (
            "find_not_empty",
            rx::Options::FIND_NOT_EMPTY,
            O::REGEX_OPTION_FIND_NOT_EMPTY,
        ),
        (
            "dont_capture",
            rx::Options::DONT_CAPTURE_GROUP,
            O::REGEX_OPTION_DONT_CAPTURE_GROUP,
        ),
    ]
}

fn ours_search(re: &rx::Regex, hay: &[u8]) -> Option<(usize, usize)> {
    match re.search_param(hay, &rx::MatchParam::default()) {
        Ok(Some(m)) => {
            let r = m.range();
            Some((r.start, r.end))
        }
        _ => None,
    }
}

fn theirs_search(re: &onig::Regex, hay: &[u8], enc: onig_sys::OnigEncoding) -> Option<(usize, usize)> {
    let buf = onig::EncodedBytes::from_parts(hay, enc);
    let mut region = onig::Region::new();
    re.search_with_encoding(
        buf,
        0,
        hay.len(),
        onig::SearchOptions::SEARCH_OPTION_NONE,
        Some(&mut region),
    )
    .and_then(|_| region.pos(0))
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

fn main() {
    let mut rng = Rng(0x5EED_1234_ABCD_9876);
    let mut checks = 0u64;
    let mut diffs: Vec<String> = Vec::new();
    let mut per_bucket: std::collections::BTreeMap<String, (u64, u64)> =
        std::collections::BTreeMap::new();

    // ---------------------------------------------------------------
    // 1. ENCODINGS -- the gap the old audit could not see.
    // ---------------------------------------------------------------
    // Patterns that stay valid across single- and multi-byte encodings.
    let enc_pats: &[&str] = &[
        r"a", r"ab", r"a.b", r"a+", r"[a-z]+", r"\w+", r"\d+", r"\s+", r"[^a]+",
        r"^a", r"a$", r"(a)(b)", r"(a)\1", r"a|b", r"a{2,3}", r"(?i)A",
        r"\bcat\b", r"[a-z]+=", r"a(?=b)", r"(?<=a)b", r"(?>a+)b", r"a++b",
        r".", r"..", r"[[:alpha:]]+", r"[[:digit:]]", r"\A a \z", r"x*",
    ];
    // Byte corpora exercising both ASCII and high bytes / multi-byte lead bytes.
    let enc_bodies: &[&[u8]] = &[
        b"abc",
        b"a b c",
        b"aabbcc=dd",
        b"cat dog cat",
        b"123 456",
        &[0x61, 0xA1, 0xA2, 0x62],
        &[0x82, 0xA0, 0x61, 0x82, 0xA1],
        &[0xB0, 0xA1, 0x41, 0xB0, 0xA2],
        &[0x61, 0x0A, 0x62, 0x0A],
        &[0x00, 0x61, 0x00, 0x62],
        &[0xE4, 0xB8, 0xAD, 0x61],
        b"",
        b"a",
    ];
    for enc in encodings() {
        for pat in enc_pats {
            // Both engines must accept the pattern in this encoding, or we skip.
            let ours = match rx::Regex::new(
                pat.as_bytes(),
                rx::Options::NONE,
                enc.ours,
                rx::Syntax::ONIGURUMA,
            ) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let pbuf = onig::EncodedBytes::from_parts(pat.as_bytes(), enc.theirs);
            let theirs = match onig::Regex::with_options_and_encoding(
                pbuf,
                onig::RegexOptions::REGEX_OPTION_NONE,
                onig::Syntax::oniguruma(),
            ) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for hay in enc_bodies {
                let a = ours_search(&ours, hay);
                let b = theirs_search(&theirs, hay, enc.theirs);
                checks += 1;
                let e = per_bucket.entry(format!("enc:{}", enc.name)).or_insert((0, 0));
                e.0 += 1;
                if a != b {
                    e.1 += 1;
                    if diffs.len() < 400 {
                        diffs.push(format!(
                            "ENC {:<11} {:<14} hay={:02x?} ours={:?} onig={:?}",
                            enc.name, pat, hay, a, b
                        ));
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // 2. SYNTAXES
    // ---------------------------------------------------------------
    let syn_pats: &[&str] = &[
        r"a+", r"a|b", r"(a)b", r"[a-z]+", r"\d", r"\w+", r"a{2,3}", r"^ab$",
        r"a\|b", r"\(a\)", r"a\{2\}", r"(?P<n>a)", r"(?<n>a)", r"\<cat\>",
        r"A", r"a.b", r"[[:alpha:]]", r"\babc\b", r"a*?b",
    ];
    let syn_hays: &[&str] = &["aab", "a|b", "(a)", "ab", "cat dog", "A", "abc", "aaab", "a{2}"];
    for (sname, sours, stheirs) in syntaxes() {
        for pat in syn_pats {
            let ours = match rx::Regex::new(pat.as_bytes(), rx::Options::NONE, rx::Encoding::UTF8, sours) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let theirs = match onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, stheirs) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for hay in syn_hays {
                let a = ours_search(&ours, hay.as_bytes());
                let b = theirs
                    .find(hay)
                    .map(|(s, e)| (s, e));
                checks += 1;
                let e = per_bucket.entry(format!("syn:{sname}")).or_insert((0, 0));
                e.0 += 1;
                if a != b {
                    e.1 += 1;
                    if diffs.len() < 400 {
                        diffs.push(format!("SYN {sname:<12} {pat:<14} hay={hay:?} ours={a:?} onig={b:?}"));
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // 3. OPTIONS
    // ---------------------------------------------------------------
    let opt_pats: &[&str] = &[
        r"a+", r"A+", r"[a-z]+", r"^a", r"a$", r"a b", r"(a)(b)", r"a|aa",
        r"\w+", r"a*", r".", r"(?i)a", r"x?", r"a.c",
    ];
    let opt_hays: &[&str] = &["aaa", "AAA", "a\nb", "ab", "a b", "", "abc", "  a  "];
    for (oname, oours, otheirs) in option_sets() {
        for pat in opt_pats {
            let ours = match rx::Regex::new(pat.as_bytes(), oours, rx::Encoding::UTF8, rx::Syntax::ONIGURUMA) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let theirs = match onig::Regex::with_options(pat, otheirs, onig::Syntax::oniguruma()) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for hay in opt_hays {
                let a = ours_search(&ours, hay.as_bytes());
                let b = theirs.find(hay);
                checks += 1;
                let e = per_bucket.entry(format!("opt:{oname}")).or_insert((0, 0));
                e.0 += 1;
                if a != b {
                    e.1 += 1;
                    if diffs.len() < 400 {
                        diffs.push(format!("OPT {oname:<14} {pat:<10} hay={hay:?} ours={a:?} onig={b:?}"));
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // 4. RANDOMIZED UTF-8 sweep, still oracle-backed
    // ---------------------------------------------------------------
    let rnd_pats: &[&str] = &[
        r"\w+", r"[0-9]+", r"(\w+)=(\w+)", r"[\w.]+@\w+", r"a+", r"(a+)+b",
        r"\d+\.\d+", r"(?m)^\w+", r"^\d+", r"(?>\w+)=", r"\d++ms", r"[a-z]+ing",
        r"a.*b", r"\bcat\b", r"(a|b)+c", r"x(?=\d)", r"(?<=a)b", r"(\w+) \1",
        r"(?<n>\w+)-(?<m>\w+)", r"\w*", r"(a)\g<1>", r"[^=]+=", r"a{2,4}",
        r"(?i)ABC", r"\p{L}+", r"(?m)^$", r"\s+", r"[a-z]+?x", r"(?~ab)",
    ];
    let alphabets = ["ab", "ab=c", "a\nb=", "aeiou=z", "0.9 x", "A@b.c", "ing t", "\nab\n", "cat cat"];
    for _ in 0..60_000 {
        let pat = rnd_pats[rng.below(rnd_pats.len())];
        let alpha: Vec<char> = alphabets[rng.below(alphabets.len())].chars().collect();
        let n = rng.below(30);
        let hay: String = (0..n).map(|_| alpha[rng.below(alpha.len())]).collect();
        let ours = match rx::Regex::new(pat.as_bytes(), rx::Options::NONE, rx::Encoding::UTF8, rx::Syntax::ONIGURUMA) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let theirs = match onig::Regex::with_options(pat, onig::RegexOptions::REGEX_OPTION_NONE, onig::Syntax::oniguruma()) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let a = ours_search(&ours, hay.as_bytes());
        let b = theirs.find(&hay);
        checks += 1;
        let e = per_bucket.entry("random:utf8".into()).or_insert((0, 0));
        e.0 += 1;
        if a != b {
            e.1 += 1;
            if diffs.len() < 400 {
                diffs.push(format!("RND {pat:<20} hay={hay:?} ours={a:?} onig={b:?}"));
            }
        }
    }

    println!("{:<20} {:>10} {:>8}", "bucket", "checks", "diffs");
    println!("{}", "-".repeat(42));
    let mut total_d = 0u64;
    for (k, (c, d)) in &per_bucket {
        total_d += d;
        println!("{:<20} {:>10} {:>8}{}", k, c, d, if *d > 0 { "  <-- " } else { "" });
    }
    println!("{}", "-".repeat(42));
    println!("{:<20} {:>10} {:>8}", "TOTAL", checks, total_d);
    if !diffs.is_empty() {
        println!("\nfirst {} differences:", diffs.len());
        for d in &diffs {
            println!("  {d}");
        }
    }
    if total_d > 0 {
        std::process::exit(1);
    }
}
