// This test demonstrates a false positive where the lint incorrectly
// suggests removing `.iter()`, which would consume `xs`, making it unavailable
// for the subsequent println!
#[allow(unknown_lints)]
#[allow(unnecessary_conversion_for_trait)]
fn main() {
    let xs = vec![[0u8; 16]];
    let mut ys: Vec<[u8; 16]> = Vec::new();
    ys.extend(xs.iter());  // Using .iter() is necessary here to preserve xs
    println!("{:?}", xs);  // `xs` is still accessible here because we used .iter() above
}
