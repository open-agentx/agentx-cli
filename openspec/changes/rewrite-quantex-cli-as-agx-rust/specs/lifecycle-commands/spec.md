# Lifecycle Commands Specification

## Purpose

Define the lifecycle command compatibility expected from Rust-native AGX.

## ADDED Requirements

### Requirement: AGX MUST preserve managed lifecycle commands

AGX SHALL expose managed lifecycle commands for supported agents.

#### Scenario: User manages an agent

- GIVEN an agent exists in the AGX catalog
- WHEN the user runs `agx install <agent>`, `agx ensure <agent>`, `agx update <agent>`, or `agx uninstall <agent>`
- THEN AGX resolves the canonical agent
- AND returns a structured lifecycle result
- AND uses compatible state records for managed installs

### Requirement: AGX MUST support all-agent update planning

AGX SHALL support updating all tracked managed agents.

#### Scenario: User updates all tracked agents

- GIVEN AGX has tracked installed agents in compatible state
- WHEN the user runs `agx update --all`
- THEN AGX groups or orders update work by recorded install source
- AND returns one structured result per tracked agent

### Requirement: AGX MUST support agent execution

AGX SHALL execute supported agent binaries through explicit and shortcut commands.

#### Scenario: User executes an agent explicitly

- WHEN the user runs `agx exec <agent> -- <args>`
- THEN AGX resolves the agent executable
- AND applies the requested install policy
- AND forwards arguments to the agent process
- AND returns a structured execution result

#### Scenario: User executes an agent shortcut

- WHEN the user runs `agx <agent> <args>`
- THEN AGX treats the invocation as shortcut execution for that agent
- AND uses the same execution behavior as `agx exec`

### Requirement: Lifecycle commands MUST support dry-run behavior

AGX SHALL support dry-run planning for lifecycle and execution commands that can mutate state or invoke external installers.

#### Scenario: User requests dry-run

- WHEN the user runs a supported lifecycle command with `--dry-run`
- THEN AGX reports the planned command or action
- AND does not mutate state
- AND does not run external installers
