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

## Overview

Repair upgrade-related Rust and Clippy API uses using diagnostics, upstream history, and tests. A successful upgrade command does not establish that the lints build.

## Scope

Named lint: that lint. Named library: all its lints. No name: all lints recursively under the current directory, including nested workspaces. Inventory manifests, lint declarations, and Dylint metadata; root `cargo metadata` may miss nested workspaces. Preserve existing edits and the upgraded toolchain; separate unrelated failures.

## Quick reference and common mistakes

| Observation or shortcut               | Action                                                                                                                                               |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Upgrade succeeds or root check passes | Check nested workspaces; run lint/UI tests and configured CI lint checks, including `cargo clippy`; confirm tests ran and inspect warnings.          |
| Clippy fixture or dependency fails    | Find a later fix; change the selected `clippy_utils` revision only if it uses the intended nightly and passes the affected test; explain the choice. |
| UI snapshot differs                   | Compare warning counts, messages, and primary and note spans; identify upstream versus lint cause before updating.                                   |

## Diagnose

Inspect each workspace's manifests, toolchain, and upgrade diff. Record the old nightly and old/new `clippy_utils` revisions, including `[workspace.dependencies]`. If the old nightly is unavailable, broaden the history search using diagnostics.

Run `cargo check --message-format=json` for every lint, grouping by workspace when useful, and compile test targets (`cargo test --no-run -p <package>`). Check exit statuses, Cargo manifest warnings, primary and secondary spans, and related notes. Even failures without usable spans count; continue across lints.

## Find and apply migrations

Search Clippy commits between old and upgraded `clippy_utils` revisions; if the old revision is unknown, search from the upgraded revision through the old nightly's date. Use explicit midnight and timezone (`--since='2026-08-20 00:00:00 +0000'`); bare dates may omit same-day commits. Narrow with `git log -S` or `-G`; inspect candidates, parents, and uses at the upgraded revision. Read an existing clone without moving its checkout, or use a temporary clone.

If Clippy history is insufficient, inspect Rust source and commit history for the target toolchain, plus open and closed issues and PRs in `rust-lang/rust` and `rust-lang/rust-clippy`. Confirm proposed migrations apply to the upgraded toolchain.

Make small edits backed by evidence and pre-upgrade comparison. Preserve diagnostics; check surrounding types, imports, signatures, and call sites. For structural HIR changes, inspect lowering and visitor behavior and preserve cases and spans unless the API necessitates change. Extract old logic verbatim first when that clarifies adaptation. Resolve conflicting migrations with further evidence, not symbol matches.

For example, `inconsistent_qualification` encountered a change from `ItemKind::Use(path, kind)` to `ItemKind::Use(tree)` with `UseKind::Nested`. Extracting its old matching logic into `match_use_tree` made adaptation reviewable; recursive matching then carried prefixes into grouped imports. A UI case with `use std::env::{split_paths, var};` checked that the warning remained and its note pointed to the matching `split_paths` leaf.

When API terminology changes, audit affected names and comments. Fix new mismatches with the migration; keep preexisting cleanup separate. After each coherent edit, rerun the check and test-target compilation. Mark a lint unresolved only after applicable Clippy, Rust, and discussion leads are exhausted; continue with other lints.

## Verify and report

Run affected lint and UI tests; zero selected tests provide no verification. Check formatting and the final diff. If commits are requested, group fixes by API or warning, separating verbatim extraction and preexisting cleanup. Report every lint checked, source changes and supporting commits or discussions, verification results, and remaining errors.
