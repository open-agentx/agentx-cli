# Development Workflow Specification

## Purpose

Define the OpenSpec-led AGX development and validation workflow.

## ADDED Requirements

### Requirement: AGX MUST use OpenSpec for durable behavior changes

AGX SHALL use OpenSpec for observable CLI behavior, structured output/schema, agent catalog metadata, configuration/state, release/upgrade, architecture-boundary, durable-process, and product-facing documentation changes.

#### Scenario: Agent starts implementation work

- GIVEN a requested change affects durable behavior or process
- WHEN an agent begins implementation
- THEN the agent updates or creates an OpenSpec change first
- AND keeps tasks synchronized with completed implementation slices

### Requirement: AGX MUST keep validation reproducible

AGX SHALL define local and CI validation commands for Rust, pnpm, package launcher, and OpenSpec checks.

#### Scenario: A functional slice is completed

- WHEN an implementation slice is ready to commit
- THEN Rust formatting, linting, tests, release build, and OpenSpec validation pass
- AND package distribution slices also verify the package launcher

### Requirement: AGX GitHub Actions MUST cover core delivery gates

AGX SHALL provide GitHub Actions for CI validation, release verification, release artifact publication, PR governance, and lifecycle smoke checks.

#### Scenario: Pull request validation runs

- WHEN a pull request targets a protected branch
- THEN CI validates Rust code and OpenSpec
- AND platform tests build and verify the AGX package launcher

#### Scenario: Release verification runs

- WHEN release verification is manually dispatched or a release tag is pushed
- THEN AGX builds platform artifacts
- AND generates verification metadata
- AND uploads release artifacts for inspection or publication
