// run-rustfix
#![allow(dead_code)]

fn main() {}

#[derive(Default, serde::Deserialize)]
struct Derived;

#[derive(Default, serde::Deserialize)]
struct DerivedWithParam<T> {
    foo: T,
}

struct Empty;

struct SimpleStruct {
    foo: Derived,
}

enum SimpleEnum {
    Foo(Derived),
}

struct StructWithParam<T> {
    foo: Derived,
    bar: T,
}

enum EnumWithParam<T> {
    Foo(Derived),
    Bar(T),
}

struct TransitiveStruct {
    foo: SimpleStruct,
}

enum TransitiveEnum {
    Foo(SimpleStruct),
}

#[derive(Default)]
struct PartiallyDerivedStruct {
    foo: Derived,
}

#[derive(serde::Deserialize)]
enum PartiallyDerivedEnum {
    Foo(Derived),
}

bitflags::bitflags! {
    struct Flags: u8 {
        const X = 1 << 0;
        const Y = 1 << 1;
        const Z = 1 << 2;
    }
}

struct StructWithFlags {
    flags: Flags,
}
