//! The greedy-repeat abort: was a process kill at ~2 KB. Now must either match
//! or return a graceful `Err`, in bounded time, at every size.
//!
//! This gate used to run libonig alongside us on every case, and its header
//! claimed these were "sizes libonig handles". They are not. `[a-z]+x` against
//! a megabyte of `abab...` is quadratic backtracking with no match to find:
//! libonig has no retry limit by default and ran for over forty minutes on it
//! before being killed, while we return `RetryLimitSearch` in 88 ms.
//!
//! That is the whole point of the retry limit, so the oracle arm is now opt-in
//! (`ORACLE_STRESS=1`) rather than something that silently wedges a test run.
//! Our arm always runs, and the gate fails if any case exceeds `BUDGET` -- a
//! hang is a production bug, and a comparison that never returns cannot tell
//! you whether you have one.
//!
//! cargo run --release --manifest-path tools/onig-bench/Cargo.toml --example stress_repeat
use std::io::Write;
use std::time::{Duration, Instant};

use rusty_expressions::{Encoding, MatchParam, Options, Regex, Syntax};

/// What libonig answers, when it is linked at all.
///
/// Behind `cfg` so the gate builds and runs without a C toolchain -- the
/// point of this example is our own bounded behaviour, and requiring the
/// oracle to observe it would defeat that.
#[cfg(feature = "oracle")]
fn onig_find(pat: &str, hay: &str) -> String {
    let ore = onig::Regex::with_options(
        pat,
        onig::RegexOptions::REGEX_OPTION_NONE,
        onig::Syntax::oniguruma(),
    )
    .unwrap();
    match ore.find(hay) {
        Some((s, e)) => format!("{s}..{e}"),
        None => "miss".into(),
    }
}

/// Per-case ceiling for our arm. The slowest case measures ~150 ms; this is
/// wide enough not to be flaky on a loaded machine, tight enough that a real
/// regression in the repeat path trips it.
const BUDGET: Duration = Duration::from_secs(5);

fn main() {
    let pats = ["(?:ab)+", "[a-z]+?x", "[a-z]+x", r"(\w)+", r"\w+", "(a|b)+"];
    let sizes = [1_000usize, 10_000, 100_000, 1_000_000];

    // Opt-in, because on the quadratic cases this arm does not finish in any
    // time worth waiting for. See the module comment.
    let oracle = cfg!(feature = "oracle") && std::env::var("ORACLE_STRESS").is_ok();
    if !oracle {
        println!(
            "(libonig arm off; build --features oracle and set ORACLE_STRESS=1 to \
             enable -- it does not terminate promptly)"
        );
    }

    println!("{:<12} {:>9} {:>22} {:>11}", "pattern", "bytes", "ours", "time");
    let mut over_budget = 0;
    let mut slowest = Duration::ZERO;

    for pat in pats {
        for n in sizes {
            let hay = "ab".repeat(n / 2);
            let re = Regex::new(
                pat.as_bytes(),
                Options::NONE,
                Encoding::UTF8,
                Syntax::ONIGURUMA,
            )
            .unwrap();

            let t = Instant::now();
            let ours = match re.search_param(hay.as_bytes(), &MatchParam::default()) {
                Ok(Some(m)) => format!("{:?}", m.range()),
                Ok(None) => "miss".into(),
                // A limit reached is a pass: bounded work, reported, no hang.
                Err(e) => format!("Err {:?}", e.kind),
            };
            let took = t.elapsed();
            slowest = slowest.max(took);

            let flag = if took > BUDGET {
                over_budget += 1;
                "  <-- OVER BUDGET"
            } else {
                ""
            };
            println!(
                "{:<12} {:>9} {:>22} {:>8.1}ms{}",
                pat,
                hay.len(),
                ours,
                took.as_secs_f64() * 1000.0,
                flag
            );
            std::io::stdout().flush().unwrap();

            #[cfg(feature = "oracle")]
            if oracle {
                let t = Instant::now();
                let theirs = onig_find(pat, &hay);
                // Only a real answer is comparable; where we stopped early on
                // purpose there is nothing to disagree with.
                let differs = !ours.starts_with("Err") && ours != theirs;
                println!(
                    "{:<12} {:>9} {:>22} {:>8.1}ms{}",
                    "  onig",
                    hay.len(),
                    theirs,
                    t.elapsed().as_secs_f64() * 1000.0,
                    if differs { "  <-- DIFFERS" } else { "" }
                );
                std::io::stdout().flush().unwrap();
            }
        }
    }

    println!(
        "\nslowest case {:.1}ms, budget {:.0}ms, {over_budget} over budget",
        slowest.as_secs_f64() * 1000.0,
        BUDGET.as_secs_f64() * 1000.0
    );
    if over_budget > 0 {
        std::process::exit(1);
    }
}
