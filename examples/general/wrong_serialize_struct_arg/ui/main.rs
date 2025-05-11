#![allow(dead_code)]

struct Color {
    r: u8,
    g: u8,
    b: u8,
}

use serde::ser::{Serialize, SerializeStruct, Serializer};

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        // The bug is that serialize_struct is called with 1 instead of 3
        let mut state = serializer.serialize_struct("Color", 1)?;
        state.serialize_field("r", &self.r)?;
        state.serialize_field("g", &self.g)?;
        state.serialize_field("b", &self.b)?;
        state.end()
    }
}

fn main() {
    let color = Color { r: 1, g: 2, b: 3 };
    let serialized = serde_json::to_string(&color).unwrap();
    println!("serialized = {}", serialized);
}

mod negative_test {
    use super::*;
    struct Struct {
        field: u8,
    }
    impl Serialize for Struct {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = serializer.serialize_struct("S", 1)?;
            state.serialize_field("field", &self.field)?;
            state.end()
        }
    }
}

mod serialize_struct_let_else_instead_of_try {
    use super::*;
    use serde::ser::Error;
    struct Struct {
        field: u8,
    }
    impl Serialize for Struct {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let Ok(mut state) = serializer.serialize_struct("S", 0) else {
                return Err(S::Error::custom("`serialize_struct` failed"));
            };
            state.serialize_field("field", &self.field)?;
            state.end()
        }
    }
}

mod serialize_field_let_else_instead_of_try {
    use super::*;
    use serde::ser::Error;
    struct Struct {
        field: u8,
    }
    impl Serialize for Struct {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = serializer.serialize_struct("S", 0)?;
            let Ok(()) = state.serialize_field("field", &self.field) else {
                return Err(S::Error::custom("`serialize_struct` failed"));
            };
            state.end()
        }
    }
}

mod multiple_serialize_struct_calls {
    use super::*;
    struct Struct {
        field: u8,
    }
    impl Struct {
        #[expect(dead_code)]
        fn foo<S>(&self, first: S, second: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = first.serialize_struct("S", 0)?;
            state.serialize_field("field", &self.field)?;
            state.end()?;

            let mut state = second.serialize_struct("S", 0)?;
            state.serialize_field("field", &self.field)?;
            state.end()
        }
    }
}

mod nested_blocks {
    use super::*;
    struct Struct {
        field: u8,
    }
    impl Struct {
        #[expect(dead_code)]
        fn foo<S>(&self, outer: S, inner: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = outer.serialize_struct("S", 0)?;

            let _ = {
                let mut state = inner.serialize_struct("S", 0)?;
                state.serialize_field("field", &self.field)?;
                state.end()
            }?;

            state.serialize_field("field", &self.field)?;
            state.end()
        }
    }
}

mod serialize_field_with_no_preceding_serialize_struct {
    use super::*;
    struct Struct {
        field: u8,
    }
    impl Struct {
        #[expect(dead_code)]
        fn foo<S>(&self, mut state: S::SerializeStruct) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            state.serialize_field("field", &self.field)?;
            state.end()
        }
    }
}

mod wrong_serialize_struct {
    use super::*;
    struct T<S> {
        serializer: S,
    }
    impl<S> T<S>
    where
        S: Serializer,
    {
        fn serialize_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<S::SerializeStruct, S::Error> {
            self.serializer.serialize_struct(name, len)
        }
    }
    struct Struct {
        field: u8,
    }
    impl Struct {
        #[expect(dead_code)]
        // smoelius: Changing `T<S>` to `S` in the next line should cause a warning to be emitted.
        fn foo<S>(&self, serializer: T<S>) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = serializer.serialize_struct("S", 0)?;
            state.serialize_field("field", &self.field)?;
            state.end()
        }
    }
}

mod test_struct_variant {
    use super::*;
    use serde::ser::SerializeStructVariant;

    enum ColorEnum {
        RGB { r: u8, g: u8, b: u8 },
    }

    impl Serialize for ColorEnum {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                ColorEnum::RGB { r, g, b } => {
                    let mut state = serializer.serialize_struct_variant(
                        "ColorEnum",
                        0,
                        "RGB",
                        1, // Wrong length, should be 3
                    )?;
                    state.serialize_field("r", r)?;
                    state.serialize_field("g", g)?;
                    state.serialize_field("b", b)?;
                    state.end()
                }
            }
        }
    }
}

mod test_tuple_struct {
    use super::*;
    use serde::ser::SerializeTupleStruct;

    struct RGB(u8, u8, u8);

    impl Serialize for RGB {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = serializer.serialize_tuple_struct("RGB", 1)?; // Wrong length, should be 3
            state.serialize_field(&self.0)?;
            state.serialize_field(&self.1)?;
            state.serialize_field(&self.2)?;
            state.end()
        }
    }
}

mod test_tuple_variant {
    use super::*;
    use serde::ser::SerializeTupleVariant;

    enum ColorEnum {
        RGB(u8, u8, u8),
    }

    impl Serialize for ColorEnum {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                ColorEnum::RGB(r, g, b) => {
                    let mut state = serializer.serialize_tuple_variant(
                        "ColorEnum",
                        0,
                        "RGB",
                        1, // Wrong length, should be 3
                    )?;
                    state.serialize_field(r)?;
                    state.serialize_field(g)?;
                    state.serialize_field(b)?;
                    state.end()
                }
            }
        }
    }
}

mod test_serialize_tuple {
    use super::*;
    use serde::ser::SerializeTuple;

    // Example: A simple 2-tuple
    struct MyPair(u8, String);

    impl Serialize for MyPair {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            // Incorrect len: 1, but there are 2 elements
            let mut tup = serializer.serialize_tuple(1)?;
            tup.serialize_element(&self.0)?;
            tup.serialize_element(&self.1)?;
            tup.end()
        }
    }

    // Example: Tuple with no elements, but len is specified as 1
    struct EmptyTupleMarker;
    impl Serialize for EmptyTupleMarker {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let tup = serializer.serialize_tuple(1)?; // Incorrect len: 1, but 0 elements
            tup.end()
        }
    }

    // Example: Correct usage for a 3-tuple (should not warn)
    struct MyTriplet(u8, u8, u8);
    impl Serialize for MyTriplet {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut tup = serializer.serialize_tuple(3)?;
            tup.serialize_element(&self.0)?;
            tup.serialize_element(&self.1)?;
            tup.serialize_element(&self.2)?;
            tup.end()
        }
    }
}
