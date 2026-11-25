macro_rules! define_caller {
    () => {
        impl Calculator {
            fn offset_from_line_column(&self) {
                advance_chars();
            }
        }
    };
}

fn main() {}

fn advance_chars() {}

struct Calculator;

define_caller!();
