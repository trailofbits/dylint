#![expect(dead_code)]

use std::{
    env::{VarError, var},
    fs::File,
    io::{Error, ErrorKind, Read},
};

fn main() {}

pub fn deref_assign_before_ok_return(flag: &mut bool) -> Result<(), VarError> {
    *flag = true;
    Ok(())
}

pub fn call_with_mut_ref_before_ok_return(xs: &mut Vec<u32>) -> Result<(), VarError> {
    xs.push(0);
    Ok(())
}

pub fn deref_assign_before_err_return(flag: &mut bool) -> Result<(), VarError> {
    *flag = true;
    Err(VarError::NotPresent)
}

pub fn call_with_mut_ref_before_err_return(xs: &mut Vec<u32>) -> Result<(), VarError> {
    xs.push(0);
    Err(VarError::NotPresent)
}

pub fn deref_assign_before_error_switch(flag: &mut bool) -> Result<(), VarError> {
    *flag = true;
    let _ = var("X")?;
    Ok(())
}

pub fn call_with_mut_ref_before_error_switch(xs: &mut Vec<u32>) -> Result<(), VarError> {
    xs.push(0);
    let _ = var("X")?;
    Ok(())
}

pub fn deref_assign_after_ok_assign(flag: &mut bool) -> Result<(), VarError> {
    let result = Ok(());
    *flag = true;
    result
}

pub fn call_with_mut_ref_after_ok_assign(xs: &mut Vec<u32>) -> Result<(), VarError> {
    let result = Ok(());
    xs.push(0);
    result
}

pub fn deref_assign_after_err_assign(flag: &mut bool) -> Result<(), VarError> {
    let result = Err(VarError::NotPresent);
    *flag = true;
    result
}

pub fn call_with_mut_ref_after_err_assign(xs: &mut Vec<u32>) -> Result<(), VarError> {
    let result = Err(VarError::NotPresent);
    xs.push(0);
    result
}

pub fn deref_assign_in_ok_arm(flag: &mut bool) -> Result<(), VarError> {
    let result = var("X").map(|_| {});
    match result {
        Ok(_) => {
            *flag = true;
        }
        Err(_) => {}
    }
    result
}

pub fn call_with_mut_ref_in_ok_arm(xs: &mut Vec<u32>) -> Result<(), VarError> {
    let result = var("X").map(|_| {});
    match result {
        Ok(_) => {
            xs.push(0);
        }
        Err(_) => {}
    }
    result
}

pub fn deref_assign_in_err_arm(flag: &mut bool) -> Result<(), VarError> {
    let result = var("X").map(|_| {});
    match result {
        Ok(_) => {}
        Err(_) => {
            *flag = true;
        }
    }
    result
}

pub fn call_with_mut_ref_in_err_arm(xs: &mut Vec<u32>) -> Result<(), VarError> {
    let result = var("X").map(|_| {});
    match result {
        Ok(_) => {}
        Err(_) => {
            xs.push(0);
        }
    }
    result
}

pub fn contributing_call(file: &mut File) -> Result<bool, Error> {
    let mut buf = [0];
    file.read(&mut buf).and_then(|size| {
        if size == 0 {
            Err(Error::from(ErrorKind::UnexpectedEof))
        } else {
            Ok(buf[0] != 0)
        }
    })
}

pub mod bank {
    pub struct Account {
        balance: i64,
    }

    pub struct InsufficientBalance;

    impl Account {
        pub fn withdraw(&mut self, amount: i64) -> Result<i64, InsufficientBalance> {
            self.balance -= amount;
            if self.balance < 0 {
                return Err(InsufficientBalance);
            }
            Ok(self.balance)
        }

        pub fn safe_withdraw(&mut self, amount: i64) -> Result<i64, InsufficientBalance> {
            let new_balance = self.balance - amount;
            if new_balance < 0 {
                return Err(InsufficientBalance);
            }
            self.balance = new_balance;
            Ok(self.balance)
        }
    }
}

pub mod more_than_two_variants {
    pub enum Error {
        Zero,
        One,
        Two,
    }

    pub fn deref_assign_before_err_return(flag: &mut bool) -> Result<(), Error> {
        *flag = true;
        Err(Error::Two)
    }
}

pub mod bitflags {
    bitflags::bitflags! {
        #[derive(Clone, Copy)]
        pub struct Flags: u8 {
            const FOO = 1 << 0;
            const BAR = 1 << 1;
        }
    }

    static FLAGS: std::sync::Mutex<Flags> = std::sync::Mutex::new(Flags::empty());

    pub fn double_check(flag: Flags) -> Result<bool, ()> {
        let flags = FLAGS.lock().unwrap();
        let prev = flags.contains(flag);
        if prev && !flags.contains(flag) {
            return Err(());
        }
        Ok(prev)
    }

    pub fn write_and_check(flag: Flags) -> Result<(), ()> {
        let mut flags = FLAGS.lock().unwrap();
        flags.insert(flag);
        if !flags.contains(flag) {
            return Err(());
        }
        Ok(())
    }
}

pub mod mut_ref_arg {
    // smoelius: Should not lint
    pub fn foo(mut s: String) -> Result<(), ()> {
        s.push('x');
        Err(())
    }

    // smoelius: Should lint
    pub fn bar(s: &mut String) -> Result<(), ()> {
        s.push('x');
        Err(())
    }
}

// smoelius: Currently, a warning is generated for the call to `env` because it modifies `command`.
// Notably, the call is not considered to "contribute" to the error because `Command` does not
// implement the `Try` trait. We may want to revisit this decision.
pub fn debug(command: &mut std::process::Command) -> Result<bool, Error> {
    command
        .env("RUST_LOG", "debug")
        .status()
        .map(|status| status.success())
}

// smoelius: I don't yet understand what is going on with async functions. But this is the smallest
// example I have produced that exhibits the false positive.
pub mod async_false_positive {
    use std::{convert::Infallible, sync::Arc};

    pub async fn deref_assign_before_noop_and_async_arc_consume() -> Result<(), Infallible> {
        let arc = Arc::new(());
        noop();
        async_arc_consume(arc).await?;
        Ok(())
    }

    pub fn noop() {}

    pub async fn async_arc_consume(_: Arc<()>) -> Result<(), Infallible> {
        Ok(())
    }
}

pub mod downcast {
    pub enum Error {
        Zero,
        One,
        Two,
    }

    pub fn deref_assign_before_downcast(flag: &mut bool) -> Result<(), Error> {
        *flag = true;
        let result = foo();
        match result {
            Err(Error::Two) => Ok(()),
            _ => result,
        }
    }

    pub fn foo() -> Result<(), Error> {
        Ok(())
    }
}

use derivative::Derivative;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Foo {
    foo: u8,
    #[derivative(Debug = "ignore")]
    bar: u8,
}

pub mod public_only {
    use std::env::VarError;

    pub fn call_with_mut_ref_before_err_return(xs: &mut Vec<u32>) -> Result<(), VarError> {
        xs.push(0);
        Err(VarError::NotPresent)
    }

    fn private_call_with_mut_ref_before_err_return(xs: &mut Vec<u32>) -> Result<(), VarError> {
        xs.push(0);
        Err(VarError::NotPresent)
    }
}

// Test to check that functions returning std::fmt::Result should not trigger the lint
pub mod fmt_result_test {
    use std::fmt::{self, Write};

    pub fn fmt_result_with_write_before_err_return(buffer: &mut String) -> fmt::Result {
        // This non-local effect (write) should not trigger a lint warning
        // because the return type is std::fmt::Result
        buffer.write_str("Hello, world!")?;
        Err(fmt::Error)
    }
}

pub mod consolidate_warnings {
    pub fn foo(x: &mut u32, y: bool, z: bool) -> std::io::Result<()> {
        *x = 0;

        if y {
            return Err(std::io::Error::other("y"));
        }

        if z {
            return Err(std::io::Error::other("z"));
        }

        Ok(())
    }
}
