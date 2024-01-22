fn dead_store() {
    let sum = |a: u64, b: u64| a + b;

    let mut arr = [0u64; 4];

    let v = sum(1, 2);
    // dead store here
    arr[0] = v;

    let v = sum(3, 4);
    // rewrites the previous store
    arr[0] = v;
}

fn no_dead_store() {
    let sum = |a: u64, b: u64| a + b;

    let mut arr = [0u64; 4];

    let v = sum(1, 2);
    arr[0] = v;

    let v = sum(3, 4);
    arr[1] = v;
}

fn dead_store_vec() {
    let sum = |a: u64, b: u64| a + b;

    let mut arr = vec![0u64; 4];

    let v = sum(1, 2);
    arr[0] = v;

    let v = sum(3, 4);
    // rewriting the previous store
    arr[0] = v;
}

fn dead_store_variant() {
    let mut arr = [0u64; 4];
    arr[0] = 1;
    arr[1] = 2;
    // rewriting the previous store
    arr[0] = 3;
}

fn no_dead_store_read() {
    let mut arr = [0u64; 4];
    arr[0] = 1;
    arr[1] = arr[0];
    arr[0] = 3;
}

fn no_dead_store_mutated() {
    let mut arr = [0u64; 4];
    arr[0] = 1;
    // we can't make any assumptions about arr after this call
    f(arr);
    arr[0] = 3;
}

fn f(mut arr: [u64; 4]) {
    unimplemented!();
}

fn main() {}
