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
        #[allow(dead_code)]
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
        #[allow(dead_code)]
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
        #[allow(dead_code)]
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
        #[allow(dead_code)]
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
