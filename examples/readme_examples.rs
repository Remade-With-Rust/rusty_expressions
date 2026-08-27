use rusty_expressions::{Encoding, Options, Regex, Syntax};
fn main() {
    let re = Regex::new("ca+t", Options::NONE, Encoding::UTF8, Syntax::ONIGURUMA).unwrap();
    let m = re.search(b"one cat two").unwrap().expect("match");
    assert_eq!(m.range(), 4..7);
    let re = Regex::new_str(r"(?<key>\w+)=(?<val>\w+)", Options::NONE, Syntax::ONIGURUMA).unwrap();
    let m = re.search(b"path=api").unwrap().expect("match");
    assert_eq!(m.name("key"), Some(0..4));
    assert_eq!(m.name("val"), Some(5..8));
    println!("README examples OK");
}
