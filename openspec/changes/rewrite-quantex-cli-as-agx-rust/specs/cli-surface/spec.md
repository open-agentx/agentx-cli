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

### Requirement: AGX MUST expose machine-readable command and schema contracts

AGX SHALL publish stable command-catalog and schema-catalog metadata for automation and agent tooling.

#### Scenario: Machine consumer inspects the command catalog

- WHEN the user runs `agx commands --json`
- THEN each command entry includes its name, summary, supported flags, stability, and output schema reference

#### Scenario: Command catalog flags match the stable lifecycle surface

- WHEN the user runs `agx commands --json`
- THEN `exec` advertises `--install` and `--non-interactive` without the alias-style `--install-policy`
- AND `install` and `ensure` do not advertise self-upgrade-only flags such as `--channel` or `--check`
- AND `upgrade` advertises `--check`, `--channel`, `--refresh`, `--no-cache`, and `--idempotency-key`

#### Scenario: Machine consumer inspects a specific structured schema

- WHEN the user runs `agx schema <command> --json`
- THEN AGX returns the requested command schema only
- AND the schema includes nested fields needed to automate capabilities, config, doctor, exec, info, inspect, install batch results, resolve, upgrade, and update results

#### Scenario: Structured schemas expose strict required fields

- WHEN the user runs `agx schema --json`
- THEN envelope schemas and nested data schemas set `additionalProperties: false`
- AND object schemas mark their required fields explicitly
- AND the command metadata schema keeps `source`, `fetchedAt`, and `staleAfter` as optional freshness fields

#### Scenario: Structured result metadata uses stable ISO-8601 timestamps

- WHEN AGX emits a JSON or NDJSON result envelope
- THEN `meta.timestamp` uses an ISO-8601 UTC timestamp
- AND commands that consult cached version metadata may include `meta.source`, `meta.fetchedAt`, and `meta.staleAfter`
- AND cached fallback metadata reports cache freshness instead of pretending a new network refresh happened

#### Scenario: Machine-readable errors expose actionable details

- WHEN AGX emits a structured error for unknown agents, unknown schema targets, invalid config actions, or missing install guidance
- THEN `error.details` includes the machine-readable input or remediation fields associated with that failure
- AND execution and resolve preflight failures duplicate install guidance into `error.details` for automation clients

#### Scenario: Human consumer inspects command and schema catalogs

- WHEN the user runs `agx commands` or `agx schema`
- THEN AGX renders a readable catalog that includes the command flags and schema references for commands
- AND the schema catalog shows each schema name with its description
