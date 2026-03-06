const MY_CONST_STR: &str = "const_str";
const MY_CONST_INT: i32 = 123;

fn main() {
    // Cases that SHOULD trigger the lint
    format!("simple string without placeholders");
    format!("hello {}", "world");
    format!("number is {}, string is {}", 123, "test");
    format!("number is {}, string is {}", MY_CONST_INT, MY_CONST_STR);
    format!("{}", "only a string literal");
    format!("{}{}{}", "a", "b", "c");
    format!("hello {}", MY_CONST_STR);
    format!("{}/file.txt", env!("CARGO_MANIFEST_DIR"));
    format!("literal {} literal {} literal", "arg1", "arg2");

    // Cases that SHOULD NOT trigger the lint
    let s_var: String = "string_var".to_string();
    format!("hello {}", s_var);

    #[derive(Debug)]
    struct Foo;
    format!("debug {:?}", Foo);
    format!("debug {:?}", MY_CONST_STR);
    format!("hex: {:x}", MY_CONST_INT);

    format!("hello {name}", name = "world_named");
    format!("hello {name}", name = MY_CONST_STR);

    println!("this is not format!");

    let dynamic = std::env::var("PATH").unwrap_or_default();
    format!("Path is {}", dynamic);

    // Edge case: format! with only a literal, no placeholders, no arguments beyond the format string.
    let _ = format!("just a literal");

    // Empty format string
    format!(""); // Suggests concat!("")

    // Format string with escape sequences
    format!("hello\n{}", "world_newline");
    format!("path: C:\\folder\\{}", "file.txt");
    format!("quotes \"here\" and {}", "there_quotes");
    format!("{{hello}} {}", "braces");

    // Format string with only placeholders
    format!("{}{}", "first", "second");
}

// Test with a function call that is const
const fn get_const_string() -> &'static str {
    "from_const_fn"
}

fn test_const_fn_arg() {
    format!("Const fn says: {}", get_const_string());
}

// Test with different literal types
fn test_literal_types() {
    format!("Int: {}, Float: {}, Bool: {}", 10, 3.14f32, true);
} 