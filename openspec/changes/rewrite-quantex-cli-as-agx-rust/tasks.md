# Tasks

## 1. Project Bootstrap

- [ ] Initialize Rust workspace and crate ownership boundaries.
- [ ] Add `agx` binary entrypoint.
- [ ] Add baseline Rust formatting, lint, test, and build validation.
- [ ] Add CI workflow for Rust validation.

## 2. Compatibility Fixtures

- [ ] Capture reference command catalog fixtures from `quantex-cli`.
- [ ] Capture reference schema fixtures from `quantex-cli`.
- [ ] Capture reference agent catalog fixtures.
- [ ] Capture representative error envelope and exit-code fixtures.
- [ ] Define fixture update policy in docs or tests.

## 3. CLI Surface

- [ ] Implement global context flags.
- [ ] Implement `human`, `json`, and `ndjson` output modes.
- [ ] Implement structured result and event envelopes.
- [ ] Implement stable error codes and exit-code mapping.
- [ ] Implement command catalog and schema catalog.

## 4. Agent Catalog and Inspection

- [ ] Migrate all supported agent definitions.
- [ ] Implement canonical name and alias lookup.
- [ ] Implement platform install method ordering.
- [ ] Implement PATH detection and executable resolution.
- [ ] Implement installed and latest version probing with cache freshness metadata.

## 5. Lifecycle Commands

- [ ] Implement `install`.
- [ ] Implement `ensure`.
- [ ] Implement `uninstall`.
- [ ] Implement `update <agent>`.
- [ ] Implement `update --all` grouped by recorded install source.
- [ ] Implement `exec` with install policy, preflight guidance, dry-run, timeout, and cancellation.
- [ ] Implement shortcut agent execution through `agx <agent>`.

## 6. Config, State, and Locks

- [ ] Implement config loading and normalization.
- [ ] Implement state loading and mutation.
- [ ] Preserve initial compatibility with `~/.quantex/config.json` and `~/.quantex/state.json`.
- [ ] Implement lifecycle and self-upgrade resource locks.

## 7. Self Upgrade

- [ ] Implement self install-source detection.
- [ ] Implement npm-installed binary self-upgrade.
- [ ] Implement Bun-installed binary self-upgrade.
- [ ] Implement standalone binary self-upgrade with checksum verification.
- [ ] Implement Windows delayed binary replacement.
- [ ] Implement recovery hints and doctor integration.

## 8. Package Distribution

- [ ] Design npm main package and platform optional dependency packages.
- [ ] Implement thin launcher for npm/Bun installation.
- [ ] Build platform binaries for release assets.
- [ ] Generate manifest and SHA256 checksums.
- [ ] Verify installed package launches native `agx`.

## 9. Documentation and Project Memory

- [x] Add AGX agent workflow bootstrap.
- [x] Add OpenSpec README and config.
- [x] Add project-memory spec.
- [x] Add task-start and worktree runbooks.
- [ ] Add Rust workspace architecture ADR after crate boundaries settle.
- [ ] Add release and package-distribution runbook.

## 10. Validation and Closure

- [ ] Run Rust validation.
- [ ] Run OpenSpec validation.
- [ ] Confirm git status.
- [ ] Deliver implementation PR when code begins.
- [ ] Keep this OpenSpec change active until implementation merges and accepted specs are synced.
- [ ] Archive this change only after implementation merge and spec sync.
