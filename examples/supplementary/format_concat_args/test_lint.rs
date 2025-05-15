// for testing,
fn main() {
    // Should trigger the lint
    let _s1 = format!("simple string");
    let _s2 = format!("hello {}", "world");
    
    // Should not trigger (not format!)
    let _s3 = "simple string".to_string();
    println!("hello {}", "world");
} 