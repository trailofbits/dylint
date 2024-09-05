#![expect(unused_variables)]

use std::cell::RefCell;

fn main() {
    let x: RefCell<usize>;
    let y = RefCell::new(0);
}
