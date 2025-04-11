// Let's add a comment to ensure the file exists and is accessible
// This file contains a test for false positive scenarios with iterators

#[allow(unnecessary_conversion_for_trait)]
fn main() {
    let xs = vec![[0u8; 16]];
    let mut ys: Vec<[u8; 16]> = Vec::new();
    ys.extend(xs.iter());  // lint incorrectly suggests removing .iter()
    println!("{:?}", xs);  // xs is used here, so .iter() is necessary
} 