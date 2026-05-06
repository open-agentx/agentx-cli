# Rewrite quantex-cli as Rust-native AGX

## Why

The project needs a Rust-native implementation that fully preserves the lifecycle capabilities of `quantex-cli` while establishing `agx` as the single canonical command-line entrypoint.

The current `quantex-cli` reference is a TypeScript/Bun implementation. AGX should use Rust for product runtime code, distribute native binaries, and treat npm and Bun as installation and self-upgrade channels rather than runtime requirements.

## What Changes

- Implement AGX as a Rust workspace with native binaries.
- Replace user-facing `qtx` and `quantex` command paths with `agx`.
- Preserve the observable lifecycle functions from `quantex-cli`: install, ensure, inspect, resolve, update, uninstall, list, info, exec, shortcut execution, capabilities, commands, schema, config, doctor, and upgrade.
- Preserve agent-friendly contracts such as `--json`, `--output ndjson`, `--non-interactive`, `--yes`, `--quiet`, `--dry-run`, `--refresh`, `--no-cache`, `--run-id`, `--idempotency-key`, and `--timeout`.
- Preserve compatible config and state semantics under `~/.quantex/` unless a later migration change intentionally renames them.
- Distribute Rust binaries directly through GitHub Releases and through npm/Bun packages that launch or install the native binary.
- Keep the legacy `quantex-cli` submodule as the behavior reference during migration.

## Capabilities

This change introduces or updates these capability areas:

- CLI surface
- agent catalog
- lifecycle execution
- package distribution
- self-upgrade
- project memory and agent workflow

## Impact

- Product implementation moves to Rust.
- npm/Bun packaging changes from JavaScript runtime delivery to native binary delivery.
- Tests need golden fixtures comparing AGX behavior against the reference implementation.
- Release automation must build, verify, checksum, and publish platform binaries.
- Agent workflow must use OpenSpec before implementation and report closure state explicitly.

## Non-goals

- Do not turn AGX into a workflow orchestration platform.
- Do not keep `qtx` or `quantex` as default user-facing commands.
- Do not require Node.js, Bun, or npm at AGX runtime when a native binary is installed.
- Do not modify the `quantex-cli` reference implementation as part of the Rust rewrite unless separately requested.
