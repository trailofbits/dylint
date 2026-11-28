// The call order would place `free` between two methods, but the `impl` moves as one item.
fn main() {}

struct Calculator;

impl Calculator {
    fn first(&self) {
        free();
        self.second();
    }

    fn second(&self) {}
}

fn free() {}
