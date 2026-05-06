# Self Upgrade Specification

## Purpose

Define the self-upgrade behavior expected from Rust-native AGX.

## Requirements

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

### Requirement: Binary self-upgrade MUST verify checksum

AGX standalone binary self-upgrade SHALL verify the downloaded binary checksum before replacement.

#### Scenario: Standalone binary upgrade

- GIVEN AGX is running as a standalone binary
- WHEN the user runs `agx upgrade`
- THEN AGX downloads the platform release asset
- AND verifies its SHA256 checksum
- AND replaces the current executable using platform-safe semantics
