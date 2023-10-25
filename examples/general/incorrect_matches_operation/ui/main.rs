fn main() {
    let x = 1;

    if matches!(x, 123) | matches!(x, 256) {
        println!("Matches");
    }

    if matches!(x, 123) || matches!(x, 256) {
        println!("Matches");
    }

    if matches!(x, 123) && matches!(x, 256) {
        println!("That is an unreachable state!");
    }

    if matches!(x, 123) & matches!(x, 256) {
        println!("That is an unreachable state!");
    }

    let _a = matches!(x, 1);
    let _b = matches!(x, 1) | matches!(x, 2);

    // This one will not error out
    let _c = matches!(x, 2) | matches!(_b, true);

    false_negative();
    false_positive();
}

fn false_negative() {
    let x = 2;
    assert!(matches!(x, 1) | matches!(x, 2));
}

fn false_positive() {
    let x = 2;
    let a = matches!(x, 1) | matches!(x, 2 if false);
    assert!(!a);
}
