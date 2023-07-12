fn main() {
    let mut x: i64 = 1;

    x *= -1000;
    x *= 1000;

    // negative tests (with default threshold)

    x *= -999;
    x *= 999;

    // negative tests (with default threshold or otherwise)

    const MILLIS: i64 = 1000;

    let a: [&str; 2] = ["x", "y"];

    x *= -1;
    x *= 1;
}
