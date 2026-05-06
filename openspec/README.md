# OpenSpec

AGX uses OpenSpec for behavior contracts and non-trivial change planning. Superpowers, when available, provides cross-agent session discipline. The AGX runtime skill provides repository-specific intake, validation, artifact routing, and closure rules.

## Structure

| Path | Purpose |
|---|---|
| `openspec/specs/` | Current source-of-truth behavior and process specifications |
| `openspec/config.yaml` | Project context and artifact rules injected into OpenSpec instructions |
| `openspec/changes/` | Proposed non-trivial changes before they are fully merged |
| `openspec/changes/archive/` | Completed changes after accepted specs are synced |

## Working Rule

- classify work through the intake gate before implementation
- activate Superpowers first when available
- use `skills/agx-agent-runtime/SKILL.md` for AGX-specific session startup, validation, artifact routing, and delivery closure
- create or select an OpenSpec change for non-trivial behavior, architecture, release, package-distribution, or durable-process changes
- keep accepted long-lived behavior in `openspec/specs/`
- treat implementation merge and OpenSpec archive closure as separate lifecycle moments
- prefer official OpenSpec, Git, GitHub CLI, and Rust tooling over repo-local workflow orchestration commands

## Useful Commands

Until package scripts are introduced, use the OpenSpec CLI directly:

```bash
openspec list
openspec status --json
openspec status --change <change-id> --json
openspec show --no-interactive <change-or-spec-name>
openspec instructions <artifact> --change <change-id> --json
openspec validate --all --no-interactive
openspec archive <change-id> --yes
```

When repository scripts are added later, they should wrap validation or generation guardrails only. They should not hide OpenSpec state transitions from review.

## Intake Gate

OpenSpec is required for changes to:

- CLI behavior
- structured output or schema
- agent catalog metadata
- install, update, uninstall, exec, or self-upgrade behavior
- config, state, cache, release, or package distribution
- architecture boundaries
- project memory, Superpowers, OpenSpec, or GitHub collaboration rules
- product-facing documentation that changes expectations

Small typo, formatting-only, mechanical maintenance, and test-only cleanup may proceed without OpenSpec after stating the classification.

## Archive Closure

Do not archive an active change before the implementation PR has merged and accepted spec deltas have been synced into `openspec/specs/`. On protected branches, archive closure should be handled by an explicit agent-driven follow-up PR.
