use std::fmt;

// ANCHOR: outer
pub fn outer() {
    let a = 1;
    // ANCHOR: inner
    let b = 2;
    // ANCHOR_END: inner
    println!("{a} {b}");
}
// ANCHOR_END: outer

impl fmt::Display for Thing {
    // ANCHOR: indented
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "thing")
    }
    // ANCHOR_END: indented
}

// ANCHOR: duplicated
// ANCHOR_END: duplicated
// ANCHOR: duplicated
// ANCHOR_END: duplicated

// ANCHOR: unterminated
fn never_closed() {}
