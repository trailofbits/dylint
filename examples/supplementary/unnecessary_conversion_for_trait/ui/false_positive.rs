#[allow(unnecessary_conversion_for_trait)]
fn main() {
    let xs = vec![[0u8; 16]];
    let mut ys: Vec<[u8; 16]> = Vec::new();
    ys.extend(xs.iter());
    //          ^^^^^^^
    println!("{:?}", xs);
} 