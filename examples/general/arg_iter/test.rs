// This should trigger our lint
fn bad_function<I: Iterator<Item = u32>>(iter: I) {
    for item in iter {
        println!("{}", item);
    }
}

// This is the better way
fn good_function<I: IntoIterator<Item = u32>>(iter: I) {
    for item in iter {
        println!("{}", item);
    }
}

// This should not trigger our lint because Iterator is used in other bounds
fn with_other_bound<I: Iterator<Item = u32> + Clone>(iter: I) {
    let cloned = iter.clone();
    for item in cloned {
        println!("{}", item);
    }
}

fn main() {
    let v = vec![1, 2, 3];

    // With IntoIterator we can pass the vec directly
    good_function(v.clone());

    // With Iterator we need to call into_iter()
    bad_function(v.into_iter());
}
