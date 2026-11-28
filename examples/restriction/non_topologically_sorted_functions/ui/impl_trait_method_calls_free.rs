fn main() {}

fn advance_chars() {}

trait Interface {
    fn offset_from_line_column(&self);
}

struct Calculator;

impl Interface for Calculator {
    fn offset_from_line_column(&self) {
        advance_chars();
    }
}
