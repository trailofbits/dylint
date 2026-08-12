// Functions expanded from this macro share an item span. Violations must still be grouped by
// function so that only the latest required position is reported for each one.
fn main() {}

macro_rules! define_function {
    ($name:ident) => {
        fn $name() {}
    };
}

define_function!(first);
define_function!(second);

fn earlier_predecessor() {
    first();
}

fn latest_predecessor() {
    first();
    second();
}
