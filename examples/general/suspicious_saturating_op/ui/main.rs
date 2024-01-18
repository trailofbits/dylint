fn main() {
    let mut x: u64 = 1;
    let y: u64 = 1;
    let z: u64 = 1;
    x = x.saturating_add(y).saturating_sub(z);
    println!("{}", x);
}
