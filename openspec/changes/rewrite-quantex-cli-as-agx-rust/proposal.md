# Rewrite quantex-cli as Rust-native AGX

## Why

Implementation requested work-intake classification: this change replaces the product runtime, command surface, packaging, release workflow, and agent lifecycle behavior, so it requires OpenSpec before and during implementation.

The reference `quantex-cli` is a TypeScript/Bun lifecycle CLI for AI coding agents. AGX needs the same observable lifecycle coverage while using Rust-native binaries, `agx` as the canonical command, and npm/Bun only as distribution and self-upgrade channels.

## What Changes

- Build AGX as a Rust workspace with the `agx` binary.
- Preserve the reference lifecycle surface under `agx`: discovery, catalog, config, install, ensure, update, uninstall, execution, doctor, and self-upgrade.
- Preserve agent-friendly structured output modes and stable error/exit-code behavior.
- Preserve migration compatibility with `~/.quantex/config.json` and `~/.quantex/state.json`.
- Replace JavaScript-runtime packaging with native binary distribution and thin npm/Bun launchers.
- Establish Rust/pnpm GitHub Actions for validation, release verification, release artifacts, PR governance, and lifecycle smoke checks.
- Keep `quantex-cli` as the read-only behavior reference during migration.

## Capabilities

### New Capabilities

- `cli-surface`: Canonical `agx` command surface and structured output contracts.
- `agent-catalog`: Supported agent metadata, lookup, inspection, and version probing.
- `lifecycle-commands`: Install, ensure, update, uninstall, execution, and shortcut execution behavior.
- `config-surface`: Compatible config/state behavior for the migration period.
- `self-upgrade`: Install-source detection and channel-specific self-upgrade behavior.
- `package-distribution`: Native binary, npm/Bun launcher, release manifest, and checksum distribution.
- `development-workflow`: OpenSpec-led development, validation, and GitHub Actions workflow expectations.

### Modified Capabilities

- `project-memory`: AGX adopts the Quantex OpenSpec-led workflow with Rust/pnpm validation and AGX-specific runbooks.

## Impact

- Affected code: `crates/agx-cli`, package launcher files under `packages/`, release scripts, GitHub Actions, fixtures, and project-memory docs.
- Affected specs: CLI surface, agent catalog, lifecycle commands, config surface, self-upgrade, package distribution, development workflow, and project memory.
- Tests and fixtures must distinguish stable compatibility fields from environment-specific values such as timestamps, run ids, absolute paths, and installed tool availability.
- Release automation must build native binaries, verify launch behavior, and publish checksummed artifacts.

## Non-goals

- Do not expose `qtx` or `quantex` as canonical user-facing entrypoints.
- Do not require Node.js, Bun, or npm when AGX is installed as a standalone native binary.
- Do not turn AGX into a workflow orchestration platform.
- Do not modify the `quantex-cli` submodule as part of this rewrite.
