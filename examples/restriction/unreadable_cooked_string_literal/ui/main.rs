// run-rustfix

fn main() {
    println!("fn main() {{\n    println!(\"Hello, world!\");\n}}");

    let _ = "newline \n, quote \", and backslash \\";

    let _ = "zero quote \" hashes";
    let _ = "one quote #\" hash to the left";
    let _ = "one quote \"# hash to the right";
    let _ = "two quote ##\" hashes to the left";
    let _ = "two quote \"## hashes to the right";

    // negative tests

    let _ = "\n newline at beginning";
    let _ = "newline at end \n";

    let _ = "\" quote at beginning";
    let _ = "quote at end \"";

    let _ = "escaped char (\x41) that is not \n, \", or \\";
}
