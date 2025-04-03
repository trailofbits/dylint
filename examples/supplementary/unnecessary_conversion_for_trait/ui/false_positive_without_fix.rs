// This version doesn't have the #[allow] attribute and would fail without our fix
fn main() {
    let xs = vec![[0u8; 16]];
    let mut ys: Vec<[u8; 16]> = Vec::new();
    ys.extend(xs.iter());
    //          ^^^^^^^
    println!("{:?}", xs);
} 