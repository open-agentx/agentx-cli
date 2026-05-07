# Self Upgrade Specification

## Purpose

Define the self-upgrade behavior expected from Rust-native AGX.

## ADDED Requirements

### Requirement: AGX MUST inspect its install source

AGX SHALL identify how the current `agx` executable was installed before selecting a self-upgrade path.

#### Scenario: Self-upgrade inspects runtime source

- WHEN the user runs `agx upgrade` or `agx doctor`
- THEN AGX classifies its install source as npm, Bun, standalone binary, source build, or unknown
- AND that classification drives upgrade behavior and recovery messaging

### Requirement: Managed self-upgrade MUST use the matching installation channel

AGX SHALL use the matching managed installation channel when self-upgrading npm-installed or Bun-installed Rust binaries.

#### Scenario: npm-installed AGX upgrades itself

- GIVEN AGX was installed through npm
- WHEN the user runs `agx upgrade`
- THEN AGX upgrades through an npm package installation path
- AND verifies that the resulting `agx` reports the expected version

#### Scenario: Bun-installed AGX upgrades itself

- GIVEN AGX was installed through Bun
- WHEN the user runs `agx upgrade`
- THEN AGX upgrades through a Bun package installation path
- AND verifies that the resulting `agx` reports the expected version

### Requirement: Managed self-upgrade MUST surface registry freshness warnings

AGX SHALL warn when the selected managed registry lags behind upstream npm metadata.

#### Scenario: Mirror registry is behind upstream npm

- GIVEN AGX is using npm or Bun as its self-upgrade channel
- AND AGX resolves a latest version from the selected registry
- AND AGX also observes a newer upstream npm version
- WHEN the user runs `agx upgrade` or `agx upgrade --check`
- THEN AGX returns a `MIRROR_LAG` warning in structured output
- AND human output explains that the selected registry currently installs an older version than upstream npm

### Requirement: Managed self-upgrade dry-run MUST preserve update intent

AGX SHALL report whether a managed self-upgrade is available even when `--dry-run` prevents execution.

#### Scenario: Dry-run managed self-upgrade with a newer version available

- GIVEN AGX is using npm or Bun as its self-upgrade channel
- AND AGX resolves a latest version newer than the current AGX version
- WHEN the user runs `agx upgrade --dry-run`
- THEN AGX returns status `update-available`
- AND includes the planned managed upgrade command
- AND includes a `DRY_RUN` warning instead of executing the upgrade

### Requirement: Binary self-upgrade MUST verify checksum

AGX standalone binary self-upgrade SHALL verify the downloaded binary checksum before replacement.

#### Scenario: Standalone binary upgrade

- GIVEN AGX is running as a standalone binary
- WHEN the user runs `agx upgrade`
- THEN AGX downloads the platform release asset
- AND verifies its SHA256 checksum
- AND replaces the current executable using platform-safe semantics
