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

fn main() {}
