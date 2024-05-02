fn main() {
    let mut x: i64 = 1;

    x *= -11;
    x *= 11;

    // negative tests (with default threshold)

    x *= -10;
    x *= 10;

    // negative tests (with default threshold or otherwise)

    const MILLIS: i64 = 1000;

    const GIGABYTE: u64 = 1024 * 1024 * 1024;

    let a: [&str; 2] = ["x", "y"];

    x *= -1;
    x *= 1;
}

fn revised_heuristic() {
    let mut x: i64 = 1;

    x *= -48;
    x *= 48;

    x *= -80;
    x *= 80;

    // negative tests: one flip

    x *= -15;
    x *= 15;

    // negative tests: single bit

    x *= -16;
    x *= 16;
}
