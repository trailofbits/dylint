// Calls in closures must be attributed to the function containing the closure. The constraints
// below are encountered in module order as
//
// `first < second < third`.
//
// The final `third < first` constraint would create a cycle and is therefore discarded.
//
// If the closure body is not visited, the middle constraint is missed and the lint spuriously warns
// that `first` should be defined after `third`.

fn main() {
    first();
}

fn first() {
    second();
}

fn second() {
    let call_third = || third();
    call_third();
}

fn third() {
    first();
}
