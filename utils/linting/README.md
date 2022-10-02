# dylint_linting

This crate provides the following macros to help in creating [Dylint] libraries:

- [`dylint_library!`]
- [`declare_late_lint!`, `declare_early_lint!`, `declare_pre_expansion_lint!`]
- [`impl_late_lint!`, `impl_early_lint!`, `impl_pre_expansion_lint!`]

## `dylint_library!`

The `dylint_library!` macro expands to the following:

```rust
#[allow(unused_extern_crates)]
extern crate rustc_driver;

#[no_mangle]
pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
    std::ffi::CString::new($crate::DYLINT_VERSION)
        .unwrap()
        .into_raw()
}
```

If your library uses the `dylint_library!` macro and the [`dylint-link`] tool, then all you should have to do is implement the [`register_lints`] function. See the [examples] in this repository.

## `declare_late_lint!`, etc.

If your library contains just one lint, using `declare_late_lint!`, etc. can make your code more concise. Each of these macros requires the same arguments as [`declare_lint!`], and wraps the following:

- a call to `dylint_library!`
- an implementation of the `register_lints` function
- a call to `declare_lint!`
- a call to [`declare_lint_pass!`]

For example, `declare_late_lint!(vis NAME, Level, "description")` expands to the following:

```rust
dylint_linting::dylint_library!();

extern crate rustc_lint;
extern crate rustc_session;

#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[NAME]);
    lint_store.register_late_pass(|| Box::new(Name));
}

rustc_session::declare_lint!(vis NAME, Level, "description");

rustc_session::declare_lint_pass!(Name => [NAME]);
```

`declare_early_lint!` and `declare_pre_expansion_lint!` are defined similarly.

## `impl_late_lint!`, etc.

`impl_late_lint!`, etc. are like `declare_late_lint!`, etc. except:

- each calls [`impl_lint_pass!`] instead of `declare_lint_pass!`;
- each requires an additional argument to specify the value of the lint's [`LintPass`] structure.

That is, `impl_late_lint!`'s additional argument is what goes here:

```rust
    lint_store.register_late_pass(|| Box::new(...));
                                              ^^^
```

An example use of `impl_pre_expansion_lint!` can be found in [env_cargo_path] in this repository.

[`declare_late_lint!`, `declare_early_lint!`, `declare_pre_expansion_lint!`]: #declare_late_lint-etc
[`declare_lint!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.declare_lint.html
[`declare_lint_pass!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.declare_lint_pass.html
[`dylint-link`]: ../../dylint-link
[`dylint_library!`]: #dylint_library
[`impl_late_lint!`, `impl_early_lint!`, `impl_pre_expansion_lint!`]: #impl_late_lint-etc
[`impl_lint_pass!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.impl_lint_pass.html
[`lintpass`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LintPass.html
[`register_lints`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints
[dylint]: https://github.com/trailofbits/dylint
[env_cargo_path]: ../../examples/general/env_cargo_path/src/lib.rs
[examples]: ../../examples
