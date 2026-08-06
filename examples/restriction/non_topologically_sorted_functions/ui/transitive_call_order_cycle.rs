// Regression test for issue #2012.
//
// Because `main` calls `fourth` before `second`, call ordering prefers `fourth < second`.
// However, the caller-callee constraints take precedence and require
// `second < third < fourth`. The call-order constraint must therefore be discarded to
// avoid a transitive cycle.
//
// Four functions are minimal for this bug. A three-function example would produce only an
// exact reverse pair: `second < third` from a caller-callee constraint and `third < second`
// from call ordering. The old implementation already detected and discarded such a pair.
// Reproducing an undetected cycle therefore requires three non-root functions:
// `fourth < second < third < fourth`.

fn main() {
    fourth();
    second();
}

fn second() {
    third();
}

fn third() {
    fourth();
}

fn fourth() {}
