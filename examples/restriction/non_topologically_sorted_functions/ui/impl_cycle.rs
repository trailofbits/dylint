fn main() {}

struct Calculator;

impl Calculator {
    fn first(&self) {
        self.second();
    }

    fn second(&self) {
        self.first();
    }
}
