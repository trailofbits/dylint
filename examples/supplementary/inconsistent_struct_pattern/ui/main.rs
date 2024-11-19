#[derive(Default)]
struct Struct {
    a: bool,
    b: bool,
    c: bool,
}

fn main() {
    let strukt = Struct::default();

    // should not lint
    let Struct { a, b, c } = strukt;
    let Struct { a, b, .. } = strukt;
    let Struct { a, c, .. } = strukt;
    let Struct { b, c, .. } = strukt;

    // should lint
    let Struct { a, c, b } = strukt;
    let Struct { b, a, c } = strukt;
    let Struct { c, b, a } = strukt;
}
