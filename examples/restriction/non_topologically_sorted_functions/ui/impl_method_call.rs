fn main() {}

struct Calculator;

impl Calculator {
    fn helper(&self) {}
}

impl Calculator {
    fn caller(&self) {
        self.helper();
    }
}
