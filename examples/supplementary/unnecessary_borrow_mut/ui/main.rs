#![allow(unused_assignments, unused_variables)]

use std::cell::RefCell;

fn main() {
    let mut x = 0;
    let cell = RefCell::new(1);

    x = *cell.borrow_mut();

    let ref_mut = cell.borrow_mut();
    x = *ref_mut;

    // negative tests

    x = *cell.borrow();

    *cell.borrow_mut() = 2;

    require_mut_ref(&mut cell.borrow_mut());

    let mut ref_mut = cell.borrow_mut();
    require_mut_ref(&mut ref_mut);

    let _: &mut u32 = &mut cell.borrow_mut();
}

fn require_mut_ref(_: &mut u32) {}
