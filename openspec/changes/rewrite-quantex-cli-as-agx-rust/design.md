# Design: Rust-native AGX Rewrite

## Overview

AGX uses `quantex-cli` as the behavior reference while replacing the runtime with Rust-native binaries. The rewrite prioritizes observable compatibility first: command coverage, structured output shape, config/state compatibility, lifecycle command planning, and release/upgrade semantics.

## Runtime Architecture

The repository is a Rust workspace with the product binary in `crates/agx-cli`.

Current module boundaries:

- `cli.rs`: `clap` command and global flag parsing.
- `commands.rs`: command dispatch, command catalog, schema catalog, and command result assembly.
- `agents.rs`: static agent catalog and alias lookup.
- `inspection.rs`: PATH lookup and installed version probing.
- `package_manager.rs`: managed install, ensure, uninstall, and update behavior.
- `exec.rs`: agent process execution, dry-run planning, install policy handling, and timeout behavior.
- `config.rs` and `state.rs`: compatible `~/.quantex` config and state handling.
- `lock.rs`: lifecycle/state/self-upgrade resource locks.
- `doctor.rs`: runtime and install-source diagnostics.
- `self_upgrade.rs`: channel-specific self-upgrade planning and execution.
- `output.rs`: JSON, NDJSON, and human output rendering.

Additional crates may be extracted later only when module boundaries stabilize and a separate OpenSpec change justifies the split.

## Command and Output Contracts

`agx` is the only canonical binary. Legacy names from the reference implementation are not exposed by default.

Machine-readable command results keep the envelope fields:

- `action`
- `data`
- `error`
- `exitCode`
- `meta`
- `ok`
- `target`
- `warnings`

JSON and NDJSON modes must avoid machine-specific instability in contracts. Fixtures should omit or normalize dynamic fields such as timestamps, run ids, absolute paths, and PATH-dependent availability.

## Packaging and Release

npm and Bun package paths are installation and launch channels for native binaries, not product runtime implementations.

Distribution shape:

- main package exposes the `agx` executable through a thin launcher,
- platform packages carry native binaries as optional dependencies,
- standalone release artifacts remain zero-runtime binaries,
- release metadata includes a manifest and SHA256 checksum files,
- CI verifies the package launcher can start the native binary.

## Migration Compatibility

AGX reads and writes the reference user-visible files during the migration:

- `~/.quantex/config.json`
- `~/.quantex/state.json`

Renaming these paths or changing state compatibility requires a future OpenSpec change with a migration plan.

## Validation Strategy

Required local and CI validation:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
pnpm run openspec:validate
```

Release-package validation additionally runs:

```bash
pnpm run dist:local
pnpm run package:verify
```
