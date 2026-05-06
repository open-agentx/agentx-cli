# Package Distribution Specification

## Purpose

Define how Rust-native AGX is distributed through binaries, npm, and Bun.

## Requirements

### Requirement: npm and Bun installs MUST launch Rust binaries

AGX npm and Bun installation paths SHALL install or launch Rust-native `agx` binaries instead of requiring the product CLI to run as JavaScript.

#### Scenario: User installs AGX with npm

- WHEN the user installs the AGX package globally with npm
- THEN the installed `agx` command launches a platform-appropriate Rust binary
- AND Node.js is used only as a package launcher or installer when that package strategy requires it

#### Scenario: User installs AGX with Bun

- WHEN the user installs the AGX package globally with Bun
- THEN the installed `agx` command launches a platform-appropriate Rust binary
- AND Bun is an installation channel rather than a runtime requirement for standalone AGX binaries

### Requirement: Standalone binaries MUST remain zero-runtime

AGX SHALL publish standalone release binaries that run without Node.js, Bun, or npm.

#### Scenario: User downloads a standalone binary

- WHEN the user downloads the AGX binary for their platform from a release
- THEN the binary can run `agx --version` without a JavaScript runtime

### Requirement: Release artifacts MUST include verification metadata

AGX release artifacts SHALL include enough metadata to verify platform binaries.

#### Scenario: Release artifacts are generated

- WHEN AGX builds release binaries
- THEN the release includes a manifest
- AND the release includes SHA256 checksums
- AND release verification executes at least the current runner's binary with `agx --version`
