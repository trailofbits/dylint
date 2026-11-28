fn main() {}

struct Calculator;

impl Calculator {
    fn caller(&self) {
        let call_method = || self.helper();
        call_method();
        free_helper();
    }

    fn helper(&self) {}
}

fn free_helper() {}
