---
name: auto-correct
description: Use when Dylint lints or lint libraries have been upgraded with `cargo dylint upgrade` and may have broken Rust or Clippy API uses.
metadata:
  version: "0.1.0"
  license: "MIT OR Apache-2.0"
  compatibility:
    requires:
      - shell access
      - cargo and git
---

# Correct lints after upgrading

Repair API uses broken by the upgraded Rust toolchain or `clippy_utils`. Use diagnostics to locate failures and historical changes to establish the right migration. Compilation and tests determine whether a repair succeeded.

## Scope

- If the user names a lint, work on that lint; if they name a library, cover every lint it defines. Otherwise, cover every lint in the current directory and its subdirectories, including nested workspaces. Inventory manifests, lint declarations, and Dylint workspace metadata; one `cargo metadata` call may miss nested workspaces.
- Preserve existing edits and the completed upgrade. Repair upgrade-related failures in the lints in scope; identify unrelated failures separately.

## Establish the upgrade and failures

1. Inspect each package or workspace's manifest, toolchain file, and upgrade diff. Find the old nightly channel and upgraded `clippy_utils` revision, including revisions in `[workspace.dependencies]`. If the old channel is unavailable, use the errors to guide a broader history search.
2. Check every lint in scope with `cargo check --message-format=json`, grouping packages by workspace when useful. Compile test targets too, such as with `cargo test --no-run -p <package>`. Check exit statuses and read primary spans, related notes, and secondary spans. A failed build with no usable spans is still a failure.
3. If compilation succeeds, run the relevant lint and UI tests before concluding that no repair is needed. Continue checking other lints when one fails.

## Find and apply migrations

1. Search Clippy's Rust source history from the upgraded `clippy_utils` revision back through the old nightly's date. Narrow by changed symbols with `git log -S` or `-G`; inspect candidate commits, their parents, and uses at the upgraded revision. Read an existing Clippy clone without changing its checkout, or use a temporary clone.
2. If Clippy history is insufficient, inspect Rust source and commit history for the target toolchain. Search relevant open and closed issues and PRs in `rust-lang/rust` and `rust-lang/rust-clippy`. Check that any proposed migration applies to the upgraded toolchain.
3. Make a small source edit supported by that evidence. Preserve the lint's intended diagnostics, checking surrounding types, imports, signatures, and related call sites. Resolve conflicting candidate migrations through more investigation rather than a token match alone.
4. Rerun the affected check and test-target compilation after each coherent edit. Use new errors to guide the next pass. Mark a lint unresolved only after the applicable Clippy, Rust, and discussion leads have been exhausted; continue with the remaining lints.

## Verify and report

Run the affected lint and UI tests, check formatting, and inspect the final diff. Report every lint checked, source changes and supporting commits or discussions, verification results, and remaining errors. A successful upgrade command does not establish that the lints build.
