fn main() {}

struct Calculator;

impl Calculator {
    fn caller(&self) {
        self.first();
        self.second();
    }

    fn second(&self) {}

    fn first(&self) {}
}
