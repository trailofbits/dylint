# wrong_serialize_struct_arg

### What it does
Checks for `serialize_struct` calls whose `len` argument does not match the number of
subsequent `serialize_field` calls.

### Why is this bad?
The [`serde` documentation] is unclear on whether the `len` argument is meant to be a hint.
Even if it is just a hint, there's no telling what real-world implementations will do with
that argument. Thus, ensuring that the argument is correct helps protect against
`SerializeStruct` implementations that expect it to be correct, even if such implementations
are only hypothetical.

### Example
```rust
let mut state = serializer.serialize_struct("Color", 1)?; // `len` is 1
state.serialize_field("r", &self.r)?;
state.serialize_field("g", &self.g)?;
state.serialize_field("b", &self.b)?;
state.end()
```
Use instead:
```rust
let mut state = serializer.serialize_struct("Color", 3)?; // `len` is 3
state.serialize_field("r", &self.r)?;
state.serialize_field("g", &self.g)?;
state.serialize_field("b", &self.b)?;
state.end()
```

[`serde` documentation]: https://docs.rs/serde/latest/serde/trait.Serializer.html#tymethod.serialize_struct
