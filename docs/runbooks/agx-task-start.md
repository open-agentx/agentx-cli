# Runbook: AGX Task Start

## Purpose

Provide one repeatable way to start or resume AGX work from a fresh coding-agent conversation.

## Canonical Start Prompt

```text
Use agx-agent-runtime.
Activate Superpowers first if this environment supports it.
Read AGENTS.md, skills/agx-agent-runtime/SKILL.md, and openspec/README.md.
Check git status, the current branch, git worktree list, and active OpenSpec changes.
If this will create commits or a PR, do not work on main; create or switch to a dedicated worktree branch named <agent>/<task-slug>.
Classify the request through the OpenSpec intake gate before editing files.
If OpenSpec is required, create or select the change and use OpenSpec status/instructions to drive implementation.
Continue through validation, commit, push, PR delivery, and archive-closure reporting as far as permissions allow.
```

## Startup Checks

```bash
git status --short --branch
git worktree list
openspec list
```

## What Not To Do

- Do not treat a slash command as project memory.
- Do not implement on `main` when the task will create commits or a PR.
- Do not duplicate full workflow instructions into agent-specific bootstrap files.
- Do not add repo-local workflow wrapper commands when native `git`, `gh`, OpenSpec, Superpowers, and Rust tooling are enough.

## Related Artifacts

- `AGENTS.md`
- `openspec/README.md`
- `skills/agx-agent-runtime/SKILL.md`
- `docs/runbooks/worktree-task-execution.md`
