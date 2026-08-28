//! Dump libonig's syntax flag tables next to ours, so the differences are
//! visible rather than guessed at.
use rusty_expressions as rx;
fn main() {
    let pairs: Vec<(&str, rx::Syntax, &onig::Syntax)> = vec![
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
    ];
    println!("{:<13} {:>12} {:>12} {:>12} {:>12} {:>10} {:>10}", "syntax", "op_ours", "op_onig", "op2_ours", "op2_onig", "beh_ours", "beh_onig");
    for (n, o, t) in &pairs {
        let ops = t.operators().bits();
        // libonig packs op and op2 into one 64-bit accessor
        let onig_op = (ops & 0xFFFF_FFFF) as u32;
        let onig_op2 = (ops >> 32) as u32;
        println!("{:<13} {:>12x} {:>12x} {:>12x} {:>12x} {:>10x} {:>10x}{}",
            n, o.op, onig_op, o.op2, onig_op2, o.behavior, t.behavior().bits(),
            if o.op == onig_op && o.op2 == onig_op2 { "" } else { "   <-- op differs" });
    }
    println!();
    println!("differing op bits (ours ^ onig):");
    for (n, o, t) in &pairs {
        let ops = t.operators().bits();
        let (oo, o2) = ((ops & 0xFFFF_FFFF) as u32, (ops >> 32) as u32);
        if o.op != oo || o.op2 != o2 {
            println!("  {:<13} op^={:08x}  op2^={:08x}", n, o.op ^ oo, o.op2 ^ o2);
        }
    }
}
