# wrong_serialize_struct_arg

### What it does

Checks for Serde serialization method calls whose `len` argument does not match the number of
subsequent `serialize_field` or `serialize_element` calls. This includes:

- `serialize_struct` (expects `serialize_field`)
- `serialize_struct_variant` (expects `serialize_field`)
- `serialize_tuple_struct` (expects `serialize_field`)
- `serialize_tuple_variant` (expects `serialize_field`)
- `serialize_tuple` (expects `serialize_element`)

### Why is this bad?

The [`serde` documentation] is unclear on whether the `len` argument is meant to be a hint.
Even if it is just a hint, there's no telling what real-world implementations will do with
that argument. Thus, ensuring that the argument is correct helps protect against
implementations that expect it to be correct, even if such implementations are only hypothetical.

### Examples

```rust
let mut state = serializer.serialize_struct("Color", 1)?; // `len` is 1, but 3 fields follow
state.serialize_field("r", &self.r)?;
state.serialize_field("g", &self.g)?;
state.serialize_field("b", &self.b)?;
state.end()

let mut tup = serializer.serialize_tuple(1)?; // `len` is 1, but 2 elements follow
tup.serialize_element(&self.0)?;
tup.serialize_element(&self.1)?;
tup.end()
```

Use instead:

```rust
let mut state = serializer.serialize_struct("Color", 3)?;
state.serialize_field("r", &self.r)?;
state.serialize_field("g", &self.g)?;
state.serialize_field("b", &self.b)?;
state.end()

let mut tup = serializer.serialize_tuple(2)?;
tup.serialize_element(&self.0)?;
tup.serialize_element(&self.1)?;
tup.end()
```

The same principle applies to other serialization methods like `serialize_struct_variant`,
`serialize_tuple_struct`, and `serialize_tuple_variant`.

[`serde` documentation]: https://docs.rs/serde/latest/serde/trait.Serializer.html#tymethod.serialize_struct
