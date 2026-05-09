# Agent Catalog Specification

## Purpose

Define the agent catalog compatibility expectations for the Rust-native AGX rewrite.

## ADDED Requirements

### Requirement: AGX MUST preserve supported agent coverage

AGX SHALL preserve the supported agent coverage from the `quantex-cli` reference implementation unless a future OpenSpec change intentionally adds or removes an agent.

#### Scenario: User lists supported agents

- WHEN the user runs `agx list --json`
- THEN the result includes the agents supported by the reference catalog
- AND each agent exposes lifecycle-focused metadata needed for install, inspect, resolve, update, uninstall, and execution

#### Scenario: Reference catalog adds a supported agent

- GIVEN the `quantex-cli` reference catalog adds a new supported agent entry
- WHEN AGX syncs catalog parity
- THEN AGX adds the same lifecycle-facing agent metadata unless an OpenSpec change intentionally diverges
- AND canonical names, aliases, install package metadata, and self-update commands stay aligned with the reference entry

### Requirement: Catalog entries MUST stay lifecycle-focused

AGX catalog metadata SHALL stay scoped to lifecycle operations and stable identification.

#### Scenario: Agent entry is defined

- WHEN AGX defines an agent entry
- THEN the entry may include canonical name, display name, lookup aliases, homepage, package metadata, install methods, binary name, version probe data, and self-update commands
- AND it does not require free-form marketing description fields

### Requirement: Catalog lookup MUST support canonical names and aliases

AGX SHALL resolve agents by canonical names and documented aliases.

#### Scenario: User invokes an alias

- GIVEN an agent has a documented alias
- WHEN the user runs an AGX lifecycle command with that alias
- THEN AGX resolves the same canonical agent entry
- AND lifecycle behavior uses the canonical agent state key

### Requirement: Doctor diagnostics MUST surface self-update remediation for untracked agents

AGX SHALL report machine-actionable self-update guidance when an installed agent can update itself but AGX is not tracking the install source.

#### Scenario: Doctor inspects an untracked self-updating agent

- GIVEN an agent binary is available on PATH
- AND AGX is not tracking that install in compatible state
- AND the agent definition exposes one or more self-update commands
- WHEN the user runs `agx doctor --json`
- THEN AGX returns an `AGENT_MANUAL_UPDATE_REQUIRED` issue
- AND the issue includes the canonical agent name
- AND the issue includes the suggested self-update command list
