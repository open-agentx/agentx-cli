# Config Surface Specification

## Purpose

Define the configuration compatibility expectations for Rust-native AGX.

## ADDED Requirements

### Requirement: AGX MUST read compatible Quantex config

AGX SHALL load user configuration from `~/.quantex/config.json` during the Rust rewrite migration.

#### Scenario: User lists configuration

- WHEN the user runs `agx config --json`
- THEN AGX returns the effective configuration after merging any stored values with built-in defaults
- AND the command does not require a config file to already exist

### Requirement: AGX MUST expose get set and reset actions

AGX SHALL expose `config get`, `config set`, and `config reset` actions compatible with the reference lifecycle surface.

#### Scenario: User sets a supported key

- WHEN the user runs `agx config set <key> <value>`
- THEN AGX validates the key and value
- AND writes `~/.quantex/config.json`
- AND returns the stored value in the structured result

#### Scenario: User resets configuration

- WHEN the user runs `agx config reset`
- THEN AGX writes the built-in default configuration
- AND returns the default configuration in the structured result

### Requirement: AGX MUST apply npm and Bun update strategy configuration

AGX SHALL use `npmBunUpdateStrategy` to choose the managed update command shape for npm and Bun installs.

#### Scenario: User updates a managed npm install with respect-semver

- GIVEN `~/.quantex/config.json` sets `npmBunUpdateStrategy` to `respect-semver`
- AND AGX tracks an npm-managed install for an agent
- WHEN the user runs `agx update <agent>`
- THEN AGX runs `npm update -g <package>`

#### Scenario: User updates a managed Bun install with latest-major

- GIVEN `~/.quantex/config.json` leaves `npmBunUpdateStrategy` at `latest-major`
- AND AGX tracks a Bun-managed install for an agent
- WHEN the user runs `agx update <agent>`
- THEN AGX runs `bun update -g --latest <package>`
