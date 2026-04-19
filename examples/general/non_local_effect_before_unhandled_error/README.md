# non_local_effect_before_unhandled_error

### What it does

Checks for calls whose errors may be unhandled and whose callees perform non-local effects
(e.g., assignments to mutable references) before returning an error.

### Why is this bad?

Functions that make changes to the program state before returning an error are difficult
to reason about: generally speaking, if a function returns an error, it should be as
though the function was never called. Failing to handle an error returned by such a
function compounds the problem, because the caller silently leaves the program in a
partially-modified state.

This lint is interprocedural: it identifies functions that may perform non-local effects
before returning an error, then flags call sites that do not handle the errors returned
by those functions.

### Known problems

- The search strategy for detecting non-local effects is exponential in the number of
  blocks in a function body. To help deal with complex bodies, the lint includes a "work
  limit" (see "Configuration" below).
- Errors in loops are not handled properly.
- Interprocedural tracking is limited to functions whose MIR is available (i.e., functions
  defined in the current crate).

### Example

```rust
impl Account {
    fn withdraw(&mut self, amount: i64) -> Result<i64, InsufficientBalance> {
        self.balance -= amount;
        if self.balance < 0 {
            return Err(InsufficientBalance);
        }
        Ok(self.balance)
    }
}

fn caller(account: &mut Account) {
    let _ = account.withdraw(100);
}
```

Use instead:

```rust
impl Account {
    fn withdraw(&mut self, amount: i64) -> Result<i64, InsufficientBalance> {
        let new_balance = self.balance - amount;
        if new_balance < 0 {
            return Err(InsufficientBalance);
        }
        self.balance = new_balance;
        Ok(self.balance)
    }
}

fn caller(account: &mut Account) -> Result<(), InsufficientBalance> {
    account.withdraw(100)?;
    Ok(())
}
```

### Configuration

- `work_limit: u64` (default 500000): When exploring a function body for non-local
  effects, the maximum number of times the search path is extended. Setting this to a
  higher number allows more bodies to be explored exhaustively, but at the expense of
  greater runtime.
