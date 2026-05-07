# Release and Package Distribution Runbook

## Purpose

Use this runbook when preparing AGX release assets and npm/Bun packages.

## Preconditions

- OpenSpec change is valid with `pnpm run openspec:validate`.
- Rust validation passes locally or in CI.
- Version numbers match across `crates/agx-cli/Cargo.toml` and package manifests under `packages/`.

## Local Verification

1. Build the native binary and release metadata:

   ```powershell
   pnpm run dist:local
   ```

2. Verify the package launcher starts the native binary:

   ```powershell
   pnpm run package:verify
   ```

3. Inspect generated files under `release-artifacts/`:

   - `manifest.json`
   - `SHA256SUMS`

`release-artifacts/` is ignored because it is generated output.

## Package Shape

- `agx-cli` is the main package and exposes the `agx` bin.
- `agx-cli-<os>-<arch>` packages are optional platform packages containing native binaries.
- The launcher first honors `AGX_BINARY_PATH`, then tries the platform optional package, then falls back to the workspace release binary for development.

## Release Notes

Document:

- supported platforms,
- SHA256 checksums,
- install commands for npm, Bun, and standalone binaries,
- known upgrade limitations,
- compatibility notes for the `~/.quantex` config and state files.
