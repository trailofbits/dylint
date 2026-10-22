# unused_reexport

A restriction lint that finds direct uses of dependency items when another direct
dependency publicly reexports the same definition.

For example, when `facade` reexports `origin::Thing`:

```rust
use origin::Thing;

fn make() -> origin::Other {
    origin::Other
}
```

The lint recommends `facade::Thing` and, if reexported, `facade::Other`.
A flagged import suppresses path diagnostics for that same definition within its
enclosing module, including paths before the import. Other definitions and nested
modules are still checked.

It compares compiler definition IDs, so similarly named items and different
versions of a dependency are distinct. Dependency aliases, renamed reexports, and
public module reexports are supported. When several paths are available, the lint
chooses the shortest path, breaking ties lexicographically. Suggestions preserve
generic arguments and import bindings. Grouped imports receive help explaining
which separate import to use.

## Usage

Build with the restriction workspace's pinned nightly toolchain:

```sh
cd examples/restriction
cargo build -p unused_reexport
cargo test -p unused_reexport
```

Load the resulting library through Dylint, for example from a target project:

```sh
cargo dylint --path /path/to/dylint/target/examples/debug/libunused_reexport@nightly-2026-08-20-x86_64-unknown-linux-gnu.so
```

The filename and extension depend on the platform and toolchain. Like the other
restriction examples, this library enables its lint at warning level when loaded.
Use `#[allow(unused_reexport)]` to opt out where direct dependencies are intentional.

## Scope and limitations

- Rustc supplies resolved paths, dependency metadata, active `cfg` selection, and
  original source spans; no rustdoc JSON or source parser is needed.
- Only direct dependencies supplied through the extern prelude and loaded by rustc
  are searched. A completely unused dependency may not be loaded.
- Macro-generated paths, macro invocations, glob imports, and associated items are
  not checked. Source code passed through macros may also be skipped.
- Only paths starting with the dependency's extern name are considered; local
  aliases such as `use origin as local_name` are not followed.
- Suggestions need review: a reexport's availability is not a promise of API
  stability, and removing a direct dependency may require other changes.

The UI test uses two local dependency fixtures and checks both positive findings
and cases that must not warn.
