# non_local_effect_before_error_return

### What it does
Checks for non-local effects (e.g., assignments to mutable references) before return of an
error.

### Why is this bad?
Functions that make changes to the program state before returning an error are difficult to
reason about. Generally speaking, if a function returns an error, it should be as though the
function was never called.

### Known problems
- The search strategy is exponential in the number of blocks in a function body. To help
  deal with complex bodies, the lint includes a "work limit" (see "Configuration" below).
- Errors in loops are not handled properly.

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
```

### Configuration
- `work_limit: u64` (default 500000): When exploring a function body, the maximum number of
  times the search path is extended. Setting this to a higher number allows more bodies to
  be explored exhaustively, but at the expense of greater runtime.
