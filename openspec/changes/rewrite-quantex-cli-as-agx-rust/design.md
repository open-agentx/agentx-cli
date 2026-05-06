# Design: Rust-native AGX Rewrite

## Overview

AGX will be implemented as a Rust workspace with a thin CLI crate and focused domain crates. The `quantex-cli` submodule remains the behavior reference for command coverage, structured output, lifecycle behavior, and release/process lessons.

## Workspace Shape

```text
crates/
  agx-cli/
  agx-core/
  agx-agent-catalog/
  agx-inspection/
  agx-lifecycle/
  agx-package-manager/
  agx-config-state/
  agx-self-upgrade/
  agx-release/
  agx-devflow/
```

Initial implementation may use fewer crates, but ownership boundaries should follow this shape so later extraction is straightforward.

## Command Surface

The only canonical binary is `agx`.

The Rust CLI should use `clap` and expose:

- `agx capabilities`
- `agx commands`
- `agx schema [command]`
- `agx inspect <agent>`
- `agx resolve <agent>`
- `agx install <agent>`
- `agx ensure <agent>`
- `agx exec <agent> --install <policy> -- [args...]`
- `agx update [agent] --all`
- `agx uninstall <agent>`
- `agx list`
- `agx info <agent>`
- `agx config [get|set|reset] [key] [value]`
- `agx upgrade [--check] [--channel stable|beta]`
- `agx doctor`
- `agx <agent> [args...]`

## Structured Output

AGX should keep a stable output envelope with:

- `action`
- `data`
- `error`
- `exitCode`
- `meta`
- `ok`
- `target`
- `warnings`

`stdout` remains reserved for structured output in JSON/NDJSON modes. Human logs and underlying installer output should avoid corrupting machine-readable output.

## Package Distribution

npm and Bun installation should install or launch Rust binaries.

Preferred distribution model:

- main npm package exposes `bin/agx` as a thin launcher
- platform-specific packages carry native binaries through optional dependencies
- Bun can consume the same npm package shape
- standalone GitHub Release assets remain canonical zero-runtime binaries

Self-upgrade must distinguish:

- npm-installed Rust binary
- Bun-installed Rust binary
- standalone binary
- source build
- unknown

## State and Config

To preserve migration safety, initial AGX should read and write the same user-visible configuration and state paths used by the reference implementation:

- `~/.quantex/config.json`
- `~/.quantex/state.json`

A future OpenSpec change may rename these paths after a compatibility and migration plan exists.

## Testing Strategy

Use golden tests captured from the reference implementation for:

- command catalog
- schema catalog
- capabilities shape
- error envelopes and exit codes
- agent catalog snapshots
- dry-run lifecycle behavior

Use isolated temporary homes for config and state tests.

Use integration tests for process spawning, timeout, cancellation, and installer command planning.

## Validation

Required Rust validation:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

OpenSpec validation:

```bash
openspec validate --all --no-interactive
```
