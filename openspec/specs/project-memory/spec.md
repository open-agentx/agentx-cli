# Project Memory Specification

## Purpose

Define how AGX stores durable behavior contracts, workflow rules, and collaboration knowledge inside the repository.

## Requirements

### Requirement: Repo-native canonical memory

AGX SHALL store long-lived project memory in versioned repository artifacts instead of relying on session memory alone.

#### Scenario: Choosing where to write durable knowledge

- GIVEN a contributor or agent needs to record durable knowledge
- WHEN the information is a behavior contract, decision, runbook, postmortem, or session summary
- THEN the contributor writes it into `openspec/` or `docs/`
- AND does not create a new ad hoc root-level markdown file

### Requirement: Non-trivial changes MUST use OpenSpec

AGX SHALL use OpenSpec change folders as the default proposal and task contract for non-trivial behavior, architecture, release, package-distribution, or durable-process changes.

#### Scenario: Planning a non-trivial change

- GIVEN a requested change alters observable CLI behavior, package distribution, architecture, release policy, project memory policy, or durable workflow
- WHEN implementation is prepared
- THEN the project records it under `openspec/changes/<change-id>/`
- AND the change includes a proposal, task list, and relevant spec delta before or alongside implementation

#### Scenario: Handling small fixes without OpenSpec overhead

- GIVEN a change is a typo fix, formatting-only cleanup, mechanical maintenance update, or test-only cleanup
- WHEN it does not alter a behavior contract or durable process
- THEN the change MAY proceed without creating an OpenSpec change
- AND the agent briefly states that classification before editing

### Requirement: Superpowers SHALL provide preferred cross-agent session discipline

AGX SHALL use Superpowers as the preferred cross-agent runtime for coding-agent session startup, planning, implementation discipline, verification, and delivery closure when it is available.

#### Scenario: Agent starts with Superpowers available

- WHEN a coding agent starts AGX repository work
- AND Superpowers is available
- THEN the agent MUST activate Superpowers before planning or editing
- AND it MUST use `skills/agx-agent-runtime/SKILL.md` for repository-specific intake, validation, artifact routing, and closure rules

#### Scenario: Agent starts without Superpowers available

- WHEN a coding agent starts AGX repository work
- AND Superpowers is not available
- THEN the agent MUST follow `AGENTS.md`
- AND it MUST still use OpenSpec and repository validation commands as the source-of-truth workflow

### Requirement: AGENTS.md must stay a thin execution handbook

AGX SHALL keep `AGENTS.md` as a thin but self-contained execution handbook for coding agents.

#### Scenario: Agent reads AGENTS.md

- WHEN a coding agent reads `AGENTS.md` at session start
- THEN the file exposes mission, non-goals, quickstart, hard constraints, validation triggers, intake gates, closure gates, and trigger-based pointers
- AND it does not copy volatile source trees, full command catalogs, or large architecture dumps

### Requirement: Work Intake Gate

Agents and contributors SHALL classify requested work before implementation or file edits begin.

#### Scenario: OpenSpec trigger is present

- GIVEN requested work changes observable CLI behavior, structured output, schema, agent catalog fields, configuration, state, release policy, package distribution, architecture boundaries, or durable workflow
- WHEN implementation is about to begin
- THEN the work MUST have an OpenSpec change with proposal, tasks, relevant spec delta, and design when useful

### Requirement: Delivery Closure Gate

Agents and contributors SHALL perform delivery closure checks before reporting implementation work as complete.

#### Scenario: Agent prepares final answer after implementation

- GIVEN an agent has implemented, documented, or otherwise changed repository files
- WHEN the agent is ready to report completion
- THEN the agent MUST check validation status, OpenSpec status, git status, commit status, push status, PR status, release status, and archive closure status as applicable
- AND the final answer MUST distinguish completed work from pending merge, release, or archive closure steps

### Requirement: Worktree-backed implementation

AGX SHALL default to dedicated git worktrees for implementation work expected to create commits or a PR.

#### Scenario: Starting PR-bound implementation

- GIVEN requested work is expected to create commits or a PR
- WHEN an agent starts implementation
- THEN it SHOULD create or reuse a dedicated worktree branch
- AND it SHOULD avoid switching or dirtying the user's primary workspace unless explicitly requested

### Requirement: Rust-native product implementation

AGX product code SHALL be implemented as Rust-native binaries.

#### Scenario: Contributor adds product runtime code

- WHEN a contributor adds AGX product implementation
- THEN the code is added to Rust crates
- AND JavaScript, TypeScript, npm, or Bun code is limited to distribution wrappers, compatibility checks, or repository guardrails when justified by OpenSpec
