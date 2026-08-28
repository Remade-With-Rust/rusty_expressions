//! Battery run inside the wasm sandbox. See `Cargo.toml` for why.
#![no_std]
#![allow(unsafe_code)]

extern crate alloc;
use alloc::vec::Vec;
use rusty_expressions::{Encoding, MatchParam, Options, Regex, Syntax};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

/// A bump allocator over a static arena.
///
/// The crate under test is `no_std` and brings no allocator of its own in this
/// configuration, so the harness supplies one. Deliberately the dumbest thing
/// that works -- freeing is a no-op -- because what is being tested is the
/// regex engine, not this.
struct Bump;

const ARENA: usize = 16 << 20;
static mut HEAP: [u8; ARENA] = [0; ARENA];
static mut NEXT: usize = 0;

unsafe impl core::alloc::GlobalAlloc for Bump {
    unsafe fn alloc(&self, l: core::alloc::Layout) -> *mut u8 {
        let base = &raw mut HEAP as *mut u8;
        let start = (NEXT + l.align() - 1) & !(l.align() - 1);
        if start + l.size() > ARENA {
            return core::ptr::null_mut();
        }
        NEXT = start + l.size();
        base.add(start)
    }
    unsafe fn dealloc(&self, _p: *mut u8, _l: core::alloc::Layout) {}
}

#[global_allocator]
static A: Bump = Bump;

/// Did this case behave? `Some((start, end))` or `None` for no match.
fn check(pat: &str, hay: &[u8], want: Option<(usize, usize)>) -> bool {
    let re = match Regex::new(pat.as_bytes(), Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA) {
        Ok(r) => r,
        Err(_) => return want.is_none(),
    };
    match re.search_param(hay, &MatchParam::default()) {
        Ok(Some(m)) => {
            let r = m.range();
            want == Some((r.start, r.end))
        }
        Ok(None) => want.is_none(),
        Err(_) => false,
    }
}

/// Number of failing cases. Zero means the engine works under wasm32.
#[no_mangle]
pub extern "C" fn run() -> i32 {
    let mut bad = 0i32;

    // Literals, classes, quantifiers, anchors, alternation.
    if !check("ca+t", b"one cat two", Some((4, 7))) { bad += 1; }
    if !check(r"\d+", b"abc 12345 def", Some((4, 9))) { bad += 1; }
    if !check(r"\w+@\w+", b"mail me@here now", Some((5, 12))) { bad += 1; }
    if !check("^start", b"start here", Some((0, 5))) { bad += 1; }
    if !check("end$", b"the end", Some((4, 7))) { bad += 1; }
    if !check("a|bb|ccc", b"xxcccyy", Some((2, 5))) { bad += 1; }
    if !check("nope", b"haystack", None) { bad += 1; }

    // Unicode -- the tables are the biggest thing we ship.
    if !check(r"\p{Lu}+", "über ÄÖÜ ok".as_bytes(), Some((6, 12))) { bad += 1; }
    if !check(r"\p{Hiragana}+", "kana ひらがな end".as_bytes(), Some((5, 17))) { bad += 1; }

    // Captures, backreferences, look-around, atomic groups.
    if !check(r"(\w+)\s+\1", b"the cat cat sat", Some((4, 11))) { bad += 1; }
    if !check(r"foo(?=bar)", b"foobar", Some((0, 3))) { bad += 1; }
    if !check(r"(?<!x)y", b"xy ay", Some((4, 5))) { bad += 1; }
    if !check(r"(?>a+)b", b"aaab", Some((0, 4))) { bad += 1; }

    // Named groups resolve through the capture machinery.
    if let Ok(re) = Regex::new_str(r"(?<num>\d+)", Options::NONE, Syntax::ONIGURUMA) {
        match re.search(b"id 4711 x") {
            Ok(Some(m)) => {
                if m.name("num") != Some(3..7) {
                    bad += 1;
                }
            }
            _ => bad += 1,
        }
    } else {
        bad += 1;
    }

    // Allocation under load: the heap path, not just the inline capture slots.
    let hay: Vec<u8> = (0..4000u32).map(|i| if i % 97 == 0 { b'z' } else { b'a' }).collect();
    if let Ok(re) = Regex::new_str("a+z", Options::NONE, Syntax::ONIGURUMA) {
        match re.search(&hay) {
            Ok(Some(m)) => {
                if m.range() != (1..98) {
                    bad += 1;
                }
            }
            _ => bad += 1,
        }
    } else {
        bad += 1;
    }

    // An engine limit must come back as an error, not as a trap.
    if let Ok(re) = Regex::new_str("(a*)*b", Options::NONE, Syntax::ONIGURUMA) {
        let long: Vec<u8> = core::iter::repeat(b'a').take(40).collect();
        // Either answer is fine; trapping the sandbox is not.
        let _ = re.search_param(&long, &MatchParam::default());
    } else {
        bad += 1;
    }

    bad
}
