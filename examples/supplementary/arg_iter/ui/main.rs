fn good<I: IntoIterator<Item = u32>>(iter: I) {
    // This is fine - using IntoIterator
    for item in iter {
        println!("{}", item);
    }
}

fn bad<I: Iterator<Item = u32>>(iter: I) {
    // This should trigger the lint - could use IntoIterator instead
    for item in iter {
        println!("{}", item);
    }
}

fn bad_with_type_parameter<T: std::fmt::Debug, I: Iterator<Item = T>>(iter: I) {
    // This should also trigger the lint
    for item in iter {
        println!("{:?}", item);
    }
}

// This should NOT trigger the lint because Iterator is used in another trait bound
fn with_other_bound<I: Iterator<Item = u32> + Clone>(iter: I) {
    let cloned = iter.clone();
    for item in cloned {
        println!("{}", item);
    }
}

fn main() {
    let v = vec![1, 2, 3];

    // With IntoIterator we can pass the vec directly
    good(v.clone());

    // With Iterator we need to call into_iter()
    bad(v.into_iter());
}
