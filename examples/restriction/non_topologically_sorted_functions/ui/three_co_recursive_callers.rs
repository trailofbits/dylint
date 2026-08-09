// The caller-callee constraints are encountered as `first < second`, `second < third`, and
// `third < first`. The last constraint must be discarded because the three constraints together
// are unsatisfiable.

fn main() {
    first();
}

fn first() {
    second();
}

fn second() {
    third();
}

fn third() {
    first();
}
