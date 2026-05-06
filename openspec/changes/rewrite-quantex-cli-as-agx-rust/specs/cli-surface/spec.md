# CLI Surface Specification

## Purpose

Define the intended AGX command surface and compatibility expectations for the Rust rewrite.

## ADDED Requirements

### Requirement: AGX MUST use agx as the canonical command

AGX SHALL expose `agx` as the canonical command-line entrypoint.

#### Scenario: User invokes AGX

- GIVEN AGX is installed
- WHEN the user runs `agx --help`
- THEN the command displays the AGX CLI help
- AND the product documentation uses `agx` examples

#### Scenario: Legacy entrypoints are considered

- GIVEN the reference implementation used `qtx` and `quantex`
- WHEN the Rust rewrite defines its user-facing command surface
- THEN it does not expose those names as canonical entrypoints
- AND any future compatibility alias requires a separate OpenSpec change

### Requirement: AGX MUST preserve lifecycle command coverage

AGX SHALL preserve the lifecycle command coverage of the reference implementation while using `agx` command examples.

#### Scenario: User discovers commands

- WHEN the user runs `agx commands --json`
- THEN the command catalog includes lifecycle, discovery, config, doctor, and self-upgrade commands
- AND each command has stable metadata for automation

### Requirement: AGX MUST preserve agent-friendly output modes

AGX SHALL support human, JSON, and NDJSON output modes.

#### Scenario: Machine consumer requests JSON

- WHEN the user runs a supported command with `--json`
- THEN AGX writes a structured result envelope to stdout
- AND warnings, errors, target, and metadata are machine-readable

#### Scenario: Machine consumer requests NDJSON

- WHEN the user runs a supported command with `--output ndjson`
- THEN AGX emits newline-delimited event envelopes
- AND the final event contains the command result
