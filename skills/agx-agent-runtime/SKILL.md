---
name: agx-agent-runtime
description: Use when starting or resuming any AGX repository task; routes agent sessions through AGX intake, OpenSpec, validation, and delivery closure.
license: MIT
---

# AGX Agent Runtime

Use this skill at the start of every AGX repository session and again before final handoff.

## Runtime Stack

- Superpowers is the preferred cross-agent workflow runtime when available.
- OpenSpec is the source of truth for non-trivial behavior and durable process contracts.
- Rust tooling is the source of truth for product validation.
- GitHub Actions are the remote enforcement layer once workflows exist.
- `AGENTS.md` is the thin fallback when Superpowers is unavailable.

## Session Start

1. Activate Superpowers first when the environment provides it.
2. Read `AGENTS.md` for mission, scope, validation, and red-line constraints.
3. Check the current branch and working tree.
4. Run `openspec list` to identify active changes when OpenSpec is available.
5. Classify the user request before editing files.

## Intake

Create or select an OpenSpec change before implementation when the work affects:

- observable CLI behavior
- stable structured output, schema, command catalog, or machine-readable contracts
- agent catalog fields, install methods, update strategies, or version probing
- configuration, state, cache, release, publishing, or self-upgrade behavior
- npm, Bun, binary, or platform package distribution
- architecture boundaries or crate ownership
- project memory policy, durable workflow, OpenSpec/Superpowers rules, ADR/runbook process, or GitHub collaboration flow
- product-facing documentation that changes user expectations

Small typo, formatting-only, mechanical no-behavior, and test-only cleanup may proceed without OpenSpec after stating that classification.

## Validation Routing

After modifying Rust product code, run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For release or binary-distribution changes, also run relevant build and artifact checks once the repository provides them.

For OpenSpec or project-memory changes, run:

```bash
openspec validate --all --no-interactive
```

If a validation command is not yet available in the bootstrap repository, report it as not available and identify the intended future command.

## Artifact Routing

- Behavior or durable process contract: `openspec/`
- Long-lived decision: `docs/adr/`
- Repeatable operation or recovery: `docs/runbooks/`
- Session summary: `docs/sessions/`
- Product runtime: Rust crates
- npm/Bun wrapper behavior: package-distribution specs plus distribution files
- Never create ad hoc root-level Markdown

## Delivery Closure

Before final handoff, report:

- validation state
- OpenSpec state
- git state
- commit state
- remote state
- PR state
- release state
- archive closure state

Use explicit closure labels: local implementation, repository delivery, PR delivery, merge delivery, OpenSpec archive closure, and release closure.
