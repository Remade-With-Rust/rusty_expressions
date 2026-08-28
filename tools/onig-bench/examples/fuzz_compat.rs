//! Fuzz the Oniguruma-shaped C ABI.
//!
//! `src/compat.rs` is the one place the crate turns off `unsafe_code = deny`,
//! and it had never been fuzzed -- every other gate drives the safe Rust API.
//! A C caller reaches these functions with raw pointers and byte offsets that
//! no type system checked, so this drives them the way a careless caller
//! would: null pointers, reversed spans, offsets past the end, regions reused
//! across searches, and entry points called in the wrong order.
//!
//! Every pointer handed in is a real, live Rust allocation -- passing a
//! genuinely dangling pointer would be undefined behaviour in the caller, not
//! a bug here. What is under test is whether valid pointers carrying hostile
//! VALUES can panic, leak, double-free or read out of bounds.
//!
//! cargo run --release --features compat --manifest-path tools/onig-bench/Cargo.toml --example fuzz_compat
#![allow(unsafe_code)]

use rusty_expressions::compat::*;
use rusty_expressions::Regex;
use std::ptr;

/// Deterministic PRNG: a failure here has to be reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const PATTERNS: &[&str] = &[
    "a",
    "a+",
    "(a)(b)?",
    r"\d+",
    r"(\w+)\s+\1",
    "(?<n>x)y",
    "a|bb|ccc",
    "^a",
    "b$",
    ".",
    "[a-z]+",
    "(?i)ABC",
    "(?>a+)b",
    "a{2,4}",
    "(?=a)a",
    "",
    "()",
    "(((a)))",
    r"\p{Lu}",
    "[^a]+",
];

const HAYS: &[&[u8]] = &[
    b"",
    b"a",
    b"ab",
    b"aaa",
    b"abc abc",
    b"xy",
    b"ccc",
    b"hello world 123",
    &[0xff, 0xfe, 0x41],
    &[0x82, 0xa0, 0x61],
];

fn main() {
    let mut rng = Rng(0x243f_6a88_85a3_08d3);
    let mut compiled = 0u64;
    let mut searched = 0u64;
    let mut matched = 0u64;
    let mut rejected = 0u64;

    for round in 0..200_000u64 {
        let pat = PATTERNS[rng.below(PATTERNS.len())];
        let hay = HAYS[rng.below(HAYS.len())];

        unsafe {
            let mut reg: *mut Regex = ptr::null_mut();
            let pb = pat.as_bytes();
            let rc = onig_new(
                &mut reg,
                pb.as_ptr(),
                pb.as_ptr().add(pb.len()),
                0,
                ONIG_ENCODING_UTF8,
                ONIG_SYNTAX_ONIGURUMA,
                ptr::null_mut(),
            );
            if rc != ONIG_NORMAL {
                rejected += 1;
                continue;
            }
            assert!(!reg.is_null(), "onig_new said OK but left a null regex");
            compiled += 1;

            let region = onig_region_new();
            assert!(!region.is_null(), "onig_region_new returned null");

            // Copy the haystack into a padded buffer so a start or range
            // pointer can legitimately sit PAST `end` while staying inside a
            // live allocation. Pointing past the haystack is what a careless C
            // caller does; computing a pointer past the whole allocation would
            // be undefined behaviour in this harness rather than a finding.
            const PAD: usize = 8;
            let mut backing = vec![0u8; hay.len() + PAD];
            backing[..hay.len()].copy_from_slice(hay);
            let base = backing.as_ptr();
            let end = base.add(hay.len());
            // Offsets that reach past the end of the haystack, and that are
            // often reversed relative to each other.
            let a = rng.below(hay.len() + PAD + 1);
            let b = rng.below(hay.len() + PAD + 1);

            match round % 5 {
                // Ordinary search.
                0 => {
                    let r = onig_search(reg, base, end, base, end, region, 0);
                    searched += 1;
                    if r >= 0 {
                        matched += 1;
                        check_region(region, hay.len());
                    }
                }
                // Start and range chosen adversarially, and often reversed.
                1 => {
                    let r = onig_search(reg, base, end, base.add(a), base.add(b), region, 0);
                    searched += 1;
                    if r >= 0 {
                        matched += 1;
                        check_region(region, hay.len());
                    }
                }
                // Null region and null start/range mean "use the defaults",
                // not "fall over".
                2 => {
                    let r =
                        onig_search(reg, base, end, ptr::null(), ptr::null(), ptr::null_mut(), 0);
                    searched += 1;
                    if r >= 0 {
                        matched += 1;
                    }
                }
                // onig_match at every offset including the one-past-the-end.
                3 => {
                    let r = onig_match(reg, base, end, base.add(a), region, 0);
                    searched += 1;
                    if r >= 0 {
                        matched += 1;
                        check_region(region, hay.len());
                    }
                }
                // Reuse one region across several searches, then clear it by
                // hand: the path where a stale allocation would be freed twice
                // or leaked.
                _ => {
                    for _ in 0..3 {
                        let h = HAYS[rng.below(HAYS.len())];
                        let hb = h.as_ptr();
                        let r =
                            onig_search(reg, hb, hb.add(h.len()), hb, hb.add(h.len()), region, 0);
                        searched += 1;
                        if r >= 0 {
                            matched += 1;
                            check_region(region, h.len());
                        }
                    }
                    onig_region_clear(region);
                    assert!((*region).beg.is_null(), "clear left a live beg pointer");
                    assert_eq!((*region).allocated, 0, "clear left a nonzero allocation");
                    // Clearing twice must be harmless.
                    onig_region_clear(region);
                }
            }

            onig_region_free(region, 1);
            onig_free(reg);
            // Freeing null is a no-op in Oniguruma; it must be one here too.
            onig_free(ptr::null_mut());
            onig_region_free(ptr::null_mut(), 1);
        }
    }

    // Entry points called out of order, or on nothing at all.
    unsafe {
        assert_eq!(onig_initialize(ptr::null_mut(), 0), ONIG_NORMAL);
        assert_eq!(onig_end(), ONIG_NORMAL);

        let pb = b"abc";
        let mut reg: *mut Regex = ptr::null_mut();
        // A reversed pattern span must be rejected, not read backwards.
        assert!(
            onig_new(
                &mut reg,
                pb.as_ptr().add(3),
                pb.as_ptr(),
                0,
                ONIG_ENCODING_UTF8,
                ONIG_SYNTAX_ONIGURUMA,
                ptr::null_mut()
            ) != ONIG_NORMAL,
            "a reversed pattern span was accepted"
        );
        // A null out-pointer must be rejected, not written through.
        assert!(
            onig_new(
                ptr::null_mut(),
                pb.as_ptr(),
                pb.as_ptr().add(3),
                0,
                ONIG_ENCODING_UTF8,
                ONIG_SYNTAX_ONIGURUMA,
                ptr::null_mut()
            ) != ONIG_NORMAL,
            "a null out-pointer was accepted"
        );
        // A null regex must be rejected by both search entry points.
        let h = b"abc";
        assert!(
            onig_search(
                ptr::null(),
                h.as_ptr(),
                h.as_ptr().add(3),
                h.as_ptr(),
                h.as_ptr().add(3),
                ptr::null_mut(),
                0
            ) < 0,
            "a null regex was searched"
        );
        assert!(
            onig_match(
                ptr::null(),
                h.as_ptr(),
                h.as_ptr().add(3),
                h.as_ptr(),
                ptr::null_mut(),
                0
            ) < 0,
            "a null regex was matched"
        );
    }

    println!(
        "fuzz_compat: {compiled} compiles, {searched} searches, {matched} matches, \
         {rejected} patterns rejected"
    );
    println!("  0 violations");
}

/// Every offset a region reports must be -1 or inside the haystack, and the
/// two arrays must agree.
unsafe fn check_region(region: *mut OnigRegion, hay_len: usize) {
    if region.is_null() {
        return;
    }
    let r = &*region;
    assert!(r.num_regs >= 0, "negative num_regs");
    if r.num_regs == 0 {
        return;
    }
    assert!(
        !r.beg.is_null() && !r.end.is_null(),
        "num_regs > 0 with null arrays"
    );
    assert!(
        r.allocated >= r.num_regs,
        "allocated {} smaller than num_regs {}",
        r.allocated,
        r.num_regs
    );
    for i in 0..r.num_regs as isize {
        let b = *r.beg.offset(i);
        let e = *r.end.offset(i);
        if b == -1 || e == -1 {
            assert_eq!(b, e, "half-unset capture {i}");
            continue;
        }
        assert!(b >= 0 && e >= 0, "negative capture bound at {i}: {b}..{e}");
        assert!(b <= e, "reversed capture at {i}: {b}..{e}");
        assert!(
            e as usize <= hay_len,
            "capture {i} ends at {e}, past haystack length {hay_len}"
        );
    }
}
