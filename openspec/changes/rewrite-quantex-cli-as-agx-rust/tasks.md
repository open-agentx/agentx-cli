# Tasks

## 1. Project Bootstrap

- [x] Initialize Rust workspace and crate ownership boundaries.
- [x] Add `agx` binary entrypoint.
- [x] Add baseline Rust formatting, lint, test, and build validation.
- [x] Add CI workflow for Rust validation.

## 2. Compatibility Fixtures

- [ ] Capture reference command catalog fixtures from `quantex-cli`.
- [ ] Capture reference schema fixtures from `quantex-cli`.
- [ ] Capture reference agent catalog fixtures.
- [ ] Capture representative error envelope and exit-code fixtures.
- [ ] Define fixture update policy in docs or tests.

## 3. CLI Surface

- [x] Implement global context flags.
- [x] Implement `human`, `json`, and `ndjson` output modes.
- [x] Implement structured result and event envelopes.
- [x] Implement stable error codes and exit-code mapping.
- [x] Implement command catalog and schema catalog.

## 4. Agent Catalog and Inspection

- [x] Add read-only catalog fields for all supported agents.
- [x] Implement `list` and `info` read-only catalog surfaces.
- [x] Implement canonical name and alias lookup.
- [ ] Migrate all supported install methods, self-update commands, and version probes.
- [ ] Implement platform install method ordering.
- [x] Implement basic PATH detection.
- [x] Implement executable resolution.
- [x] Implement basic installed version probing.
- [ ] Implement latest version probing with cache freshness metadata.

## 5. Lifecycle Commands

- [x] Implement `install`.
- [x] Implement `ensure`.
- [x] Implement `uninstall`.
- [x] Implement `update <agent>`.
- [x] Implement `update --all` grouped by recorded install source.
- [x] Implement `exec` with install policy, preflight guidance, dry-run, timeout, and cancellation.
- [x] Implement shortcut agent execution through `agx <agent>`.

## 6. Config, State, and Locks

- [x] Implement config loading and normalization.
- [x] Implement state loading and mutation.
- [x] Preserve initial compatibility with `~/.quantex/config.json` and `~/.quantex/state.json`.
- [x] Implement lifecycle and self-upgrade resource locks.

## 7. Self Upgrade

- [x] Implement self install-source detection.
- [ ] Implement npm-installed binary self-upgrade.
- [ ] Implement Bun-installed binary self-upgrade.
- [ ] Implement standalone binary self-upgrade with checksum verification.
- [ ] Implement Windows delayed binary replacement.
- [x] Implement recovery hints and doctor integration.

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
