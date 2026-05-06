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
