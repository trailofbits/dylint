# dylint-link

`dylint-link` is a wrapper around Rust's default linker (`cc`) to help create [Dylint](https://github.com/trailofbits/dylint) libraries.

When you link a dynamic library with the same name as your package, `dylint-link` creates a copy of your library with a filename that Dylint recognizes, i.e.:

```
DLL_PREFIX LIBRARY_NAME '@' TOOLCHAIN DLL_SUFFIX
```

To use `dylint-link`, install it:

```sh
cargo-install dylint-link --version '>=0.1.0-pre'
```

And set it as the linker in your library's `.cargo/config.toml` file, e.g.:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "dylint-link"
```

If your library uses `dylint-link` and the [`dylint_library!`](../utils/linting) macro, then all you should have to do is implement the [`register_lints`](https://doc.rust-lang.org/stable/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints) function.  See the [examples](../examples) in this repository.
