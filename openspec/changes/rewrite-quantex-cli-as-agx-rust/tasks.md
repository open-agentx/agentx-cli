# Tasks

## 1. OpenSpec

- [x] 1.1 Define the Rust-native AGX rewrite proposal, design, and capability deltas.
- [x] 1.2 Add canonical `agx` CLI surface requirements.
- [x] 1.3 Add agent catalog and inspection requirements.
- [x] 1.4 Add lifecycle command requirements.
- [x] 1.5 Add compatible config/state requirements.
- [x] 1.6 Add self-upgrade requirements.
- [x] 1.7 Add package distribution requirements.
- [x] 1.8 Add development workflow and GitHub Actions requirements.

## 2. Rust CLI Implementation

- [x] 2.1 Initialize Rust workspace and `agx` binary entrypoint.
- [x] 2.2 Implement global context flags and output modes.
- [x] 2.3 Implement structured result envelopes, command catalog, schema catalog, and stable error codes.
- [x] 2.4 Implement supported agent catalog, canonical lookup, aliases, basic PATH detection, executable resolution, and installed version probing.
- [x] 2.5 Implement config loading, config mutation, state mutation, and compatible `~/.quantex` files.
- [x] 2.6 Implement lifecycle locks and self-upgrade locks.
- [x] 2.7 Implement `install`, `ensure`, `uninstall`, `update <agent>`, and `update --all`.
- [x] 2.8 Implement `exec` and shortcut execution through `agx <agent>`.
- [x] 2.9 Implement `doctor` diagnostics and install-source detection.
- [x] 2.10 Implement npm and Bun managed self-upgrade paths.
- [x] 2.11 Complete standalone binary self-upgrade with checksum verification and Windows delayed replacement.
- [x] 2.12 Complete all install-method metadata, platform ordering, self-update metadata, and latest-version cache freshness.

## 3. Packaging and Release

- [x] 3.1 Design npm main package and platform optional dependency packages.
- [x] 3.2 Implement thin npm/Bun launcher for native AGX binaries.
- [x] 3.3 Generate release manifest and SHA256 checksum metadata.
- [x] 3.4 Verify the package launcher starts native `agx`.
- [x] 3.5 Align Rust crate, npm main package, and platform package names on `agxctl`.
- [ ] 3.6 Build full platform binary release assets.

## 4. Fixtures and Documentation

- [x] 4.1 Capture compatibility fixtures for command catalog, schema summary, agent catalog, and representative error envelope.
- [x] 4.2 Define fixture update policy.
- [x] 4.3 Add Rust integration tests under `crates/agx-cli/tests` for CLI contracts, compatible config/state, execution flows, and command-level coverage comparable to `quantex-cli/test/commands`.
- [x] 4.3.1 Close latest `run/exec` parity gaps from `quantex-cli/test/commands/run.test.ts`, including unknown-agent handling, shortcut cancellation/launch failure exit codes, and explicit `--install if-missing` non-interactive execution coverage.
- [x] 4.3.2 Close latest `resolve` parity gaps from `quantex-cli/test/commands/resolve.test.ts`, including missing-agent install guidance details, installed-version reporting, and human-readable launch metadata.
- [x] 4.3.3 Close latest `doctor` parity gaps from `quantex-cli/test/commands/doctor.test.ts`, including manual-update guidance for unmanaged PATH agents without a self-update command.
- [x] 4.3.4 Close latest `doctor` outdated-agent parity gaps from `quantex-cli/test/commands/doctor.test.ts`, including outdated markers for installed agent listings.
- [x] 4.3.5 Close latest `commands` and `schema` human-output parity gaps from `quantex-cli/test/commands/commands.test.ts` and `quantex-cli/test/commands/schema.test.ts`, including command flag / schema reference display in human mode.
- [x] 4.3.6 Close latest `inspect` human-output parity gaps from `quantex-cli/test/commands/inspect.test.ts`, including latest-version display for installed agents.
- [x] 4.3.7 Close latest `ensure` and `uninstall` human-output parity gaps from `quantex-cli/test/commands/ensure.test.ts` and `quantex-cli/test/commands/uninstall.test.ts`, including progress and success messaging.
- [x] 4.4 Add AGX agent workflow bootstrap, OpenSpec README/config, and project-memory spec.
- [x] 4.5 Add task-start and worktree runbooks.
- [x] 4.6 Add Rust workspace architecture ADR.
- [x] 4.7 Add release and package-distribution runbook.

## 5. GitHub Actions

- [x] 5.1 Add Rust and OpenSpec CI validation.
- [x] 5.2 Expand CI to Linux, Windows, and macOS matrix validation.
- [x] 5.3 Add PR governance workflow.
- [x] 5.4 Add lifecycle smoke workflow.
- [x] 5.5 Add release verification and release artifact workflows.

## 6. Validation

- [x] 6.1 Run `cargo fmt --all -- --check`.
- [x] 6.2 Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 6.3 Run `cargo test --workspace`.
- [x] 6.4 Run `cargo build --workspace --release`.
- [x] 6.5 Run `pnpm run openspec:validate`.
- [x] 6.6 Run `pnpm run dist:local` and `pnpm run package:verify` for package-distribution slices.

## 7. Delivery

- [x] 7.1 Commit completed implementation slices with synchronized OpenSpec task updates.
- [ ] 7.2 Push the branch and open an implementation PR.
- [ ] 7.3 Keep this OpenSpec change active until implementation merges and accepted specs are synced.
- [ ] 7.4 Archive this change only after implementation merge and spec sync.
