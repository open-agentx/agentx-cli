# AGENTS.md

## Mission

- AGX is a Rust-native agent lifecycle CLI.
- The product scope is installing, inspecting, ensuring, updating, uninstalling, discovering, launching, and self-upgrading AI coding assistant CLIs.
- AGX keeps a human-friendly and agent-friendly command surface centered on `agx`.
- AGX is not a workflow orchestration platform.
- The legacy `quantex-cli` submodule is a reference implementation, not the runtime target.

## Agent Quickstart

1. Activate Superpowers first when the current agent environment supports it.
2. Read `skills/agx-agent-runtime/SKILL.md` and `openspec/README.md`.
3. Check `git status --short --branch`, `git worktree list`, and active OpenSpec changes before editing.
4. Classify the request through the Work Intake Gate.
5. For non-trivial behavior, architecture, release, package distribution, or durable-process changes, create or select an OpenSpec change before implementation.
6. Prefer a dedicated worktree for work that will create commits or a PR.
7. Before final handoff, report validation, OpenSpec, git, commit, push, PR, release, and archive-closure state.

## Must

- Use Rust as the implementation language for AGX product code.
- Keep `agx` as the canonical command-line entrypoint.
- Preserve the observable lifecycle capabilities of `quantex-cli` unless an OpenSpec change explicitly changes the contract.
- Treat npm and Bun as installation and self-upgrade channels for Rust binaries, not as a JavaScript runtime requirement.
- Keep durable workflow and behavior contracts in `openspec/`.
- Keep long-lived decisions in `docs/adr/`, repeatable operations in `docs/runbooks/`, and session summaries in `docs/sessions/`.
- Keep `AGENTS.md` short and execution-focused; route details to source files, OpenSpec, docs, or skills.

## Must Not

- Do not reintroduce `qtx` or `quantex` as user-facing entrypoints unless a future OpenSpec change intentionally adds compatibility aliases.
- Do not grow AGX into a workflow orchestration platform.
- Do not add ad hoc root-level markdown for durable knowledge.
- Do not duplicate full workflow instructions into agent-specific directories.
- Do not report OpenSpec-backed work as fully closed while implementation, merge, archive, or release closure remains pending.
- Do not modify the `quantex-cli` submodule unless the user explicitly asks to change the reference implementation.

## Work Intake Gate

Classify requested work before implementation or file edits.

Create or select an OpenSpec change first when the work affects:

- observable CLI behavior
- stable structured output, schema, command catalog, or machine-readable contracts
- agent catalog fields, install methods, update strategy, version probing, or execution semantics
- configuration, state, cache, release, publishing, or self-upgrade behavior
- npm, Bun, binary, or platform package distribution
- architecture boundaries or crate ownership
- project memory policy, durable workflow, OpenSpec rules, Superpowers rules, ADR/runbook process, or GitHub collaboration flow
- product-facing documentation that changes user expectations

OpenSpec may be skipped only for typo fixes, formatting-only cleanup, small wording cleanup with no product or process meaning change, mechanical no-behavior maintenance, or test-only cleanup that does not redefine expected behavior.

## Validation

Rust product validation:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Project-memory validation:

```bash
openspec validate --all --no-interactive
```

Until repository scripts exist, use the OpenSpec CLI directly or record the missing command as follow-up work in the relevant OpenSpec change.

## Delivery Closure Gate

Before reporting implementation work as complete, check and report:

- validation state
- OpenSpec state
- git state
- commit state
- remote/push state
- PR state
- release state
- archive closure state

Use explicit closure labels:

- local implementation
- repository delivery
- PR delivery
- merge delivery
- OpenSpec archive closure
- release closure

## Trigger-Based Pointers

- Rust rewrite plan: `openspec/changes/rewrite-quantex-cli-as-agx-rust/`
- Current process contract: `openspec/specs/project-memory/spec.md`
- Agent runtime: `skills/agx-agent-runtime/SKILL.md`
- Task start runbook: `docs/runbooks/agx-task-start.md`
- Worktree runbook: `docs/runbooks/worktree-task-execution.md`
- Reference implementation: `quantex-cli/`
