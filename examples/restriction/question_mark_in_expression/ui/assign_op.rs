#![allow(unused)]

fn main() -> Result<(), ()> {
    let mut x = 0;
    x += foo()?;
    Ok(())
}

fn foo() -> Result<u32, ()> {
    Ok(1)
}
