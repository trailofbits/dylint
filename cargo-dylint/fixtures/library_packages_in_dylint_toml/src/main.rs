fn main() {
    let result: Result<_, _> = std::fs::read_dir(".").unwrap().next().unwrap();
    println!("{:?}", result.unwrap().path());
}
