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

#### Scenario: Another process already holds the agent lifecycle lock

- GIVEN another AGX process is already mutating agent lifecycle state
- WHEN the user runs `agx install`, `agx ensure`, `agx uninstall`, or `agx update`
- THEN AGX fails with a resource-locked result instead of running the lifecycle operation concurrently

#### Scenario: User installs multiple agents in one command

- GIVEN multiple requested agents exist in the AGX catalog
- WHEN the user runs `agx install <agent-a> <agent-b> ...`
- THEN AGX installs them sequentially
- AND returns one structured result per requested input
- AND includes an aggregate batch summary in structured output
- AND continues processing later inputs even when an earlier input fails

### Requirement: AGX MUST support all-agent update planning

AGX SHALL support updating all tracked managed agents.

#### Scenario: User updates all tracked agents

- GIVEN AGX has tracked installed agents in compatible state
- WHEN the user runs `agx update --all`
- THEN AGX groups or orders update work by recorded install source
- AND returns one structured result per tracked agent

#### Scenario: Managed install version metadata is more precise than binary probing

- GIVEN AGX is tracking a managed npm or Bun install for an agent
- AND the package manager can report the installed package version directly
- WHEN AGX inspects the agent or evaluates `agx update --all`
- THEN AGX prefers the managed package version over binary version probing
- AND up-to-date detection uses that managed package version when available

### Requirement: AGX MUST support agent execution

AGX SHALL execute supported agent binaries through explicit and shortcut commands.

#### Scenario: User executes an agent explicitly

- WHEN the user runs `agx exec <agent> -- <args>`
- THEN AGX resolves the agent executable
- AND applies the requested install policy
- AND forwards arguments to the agent process
- AND returns a structured execution result

#### Scenario: Human execution preserves interactive agent stdio

- WHEN the user runs `agx exec <agent> -- <args>` or `agx <agent> <args>` in human output mode
- THEN AGX connects the agent stdin, stdout, and stderr to the current terminal
- AND interactive agent prompts remain usable during execution

#### Scenario: Explicit execution targets an unknown agent

- WHEN the user runs `agx exec <agent> -- <args>` for an agent outside the catalog
- THEN AGX returns an `AGENT_NOT_FOUND` result
- AND uses the stable unknown-agent exit code

#### Scenario: Explicit execution auto-installs in non-interactive mode when requested

- GIVEN the requested agent is not installed
- WHEN the user runs `agx exec <agent> --install if-missing -- <args>` in non-interactive mode
- THEN AGX installs the managed agent without prompting
- AND executes the requested agent command after installation
- AND returns a successful execution result

#### Scenario: User executes an agent shortcut

- WHEN the user runs `agx <agent> <args>`
- THEN AGX treats the invocation as shortcut execution for that agent
- AND uses the same execution behavior as `agx exec`

#### Scenario: Shortcut execution needs an install confirmation

- GIVEN the requested agent is not installed
- WHEN the user runs `agx <agent> <args>` in an interactive session
- THEN AGX asks whether it should install the agent first
- AND installs before execution when the user confirms
- AND returns a cancelled result when the user declines

#### Scenario: Shortcut execution targets an unknown agent

- WHEN the user runs `agx <agent> <args>` for an agent outside the catalog
- THEN AGX returns an `AGENT_NOT_FOUND` result
- AND uses the stable unknown-agent exit code

#### Scenario: Resolve output reports install guidance and launch details

- WHEN the user runs `agx resolve <agent>` or `agx resolve <agent> --json`
- THEN AGX reports whether the agent is installed
- AND includes installed version and launch path data when available
- AND provides install guidance when the agent is missing

#### Scenario: Inspect output reports latest version details for installed agents

- WHEN the user runs `agx inspect <agent>`
- THEN AGX shows the installed version when available
- AND shows the latest version when available
- AND includes the agent install methods in human-readable form

#### Scenario: Doctor reports manual update guidance for unmanaged PATH agents

- GIVEN an agent is visible in PATH but not tracked as a managed install
- WHEN the user runs `agx doctor`
- THEN AGX reports an agent remediation issue
- AND includes manual update guidance even when the agent has no self-update command

#### Scenario: Doctor marks outdated installed agents

- GIVEN AGX knows both the installed and latest versions for a tracked agent
- WHEN the user runs `agx doctor`
- THEN AGX marks that agent as outdated when the versions differ
- AND reports the latest version in the installed agents list

#### Scenario: Human lifecycle commands report install progress

- WHEN the user runs `agx ensure <agent>` or `agx uninstall <agent>`
- THEN AGX renders a human-readable progress message for the lifecycle action
- AND renders a human-readable completion message when the action succeeds

### Requirement: Lifecycle commands MUST support dry-run behavior

AGX SHALL support dry-run planning for lifecycle and execution commands that can mutate state or invoke external installers.

#### Scenario: User requests dry-run

- WHEN the user runs a supported lifecycle command with `--dry-run`
- THEN AGX reports the planned command or action
- AND does not mutate state
- AND does not run external installers

### Requirement: Lifecycle commands MUST safely adopt detectable existing installs

AGX SHALL begin tracking an already-installed agent when the install source can be inferred without guessing.

#### Scenario: Managed install is inferred from npm or Bun layout

- GIVEN an agent binary already exists on PATH
- AND AGX has no recorded state for that agent
- WHEN the user runs `agx install <agent>` or `agx ensure <agent>`
- THEN AGX detects npm or Bun layout from the resolved binary path when available
- AND records compatible managed install state
- AND reports that AGX is now tracking the existing install

#### Scenario: Script-driven install is inferred from self-update metadata

- GIVEN an agent binary already exists on PATH
- AND the agent exposes a stable self-update command but no managed npm or Bun package
- AND AGX has no recorded state for that agent
- WHEN the user runs `agx install <agent>` or `agx ensure <agent>`
- THEN AGX records compatible script install state using the known self-update command
- AND reports that AGX is now tracking the existing install
