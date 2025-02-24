// This path does not exist
// See ../nonexistent/path/file.rs

// This path should exist
// See ../src/lib.rs

/* This is a block comment with a nonexistent path
   See ../another/nonexistent/path/file.go
*/

// Single dot path that does exist
// ./main.rs

// Single dot path that does not exist
// The span ./ido/not/exist.rs only points to the path

// Workspace root path that does exist
// nonexistent_path_in_comment/Cargo.toml

// (https://github.com/trailofbits/dylint).

// /bin/rustc.

// like `$ORIGIN/../../a.rs`... (see https://github.com/trailofbits/dylint/issues/54

// /tmp  $DIR/very/nice/nonexistent.rs


// https://doc-rust-lang.org/std/cell/struct.html


// [`RefCell`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html

/// [`RefCell::borrow_mut`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.borrow_mut 
/// [`RefCell::borrow`]: https://doc.rust-lang.org/std/cell/struct.RefCell.html#method.borrow 

fn main() {}
