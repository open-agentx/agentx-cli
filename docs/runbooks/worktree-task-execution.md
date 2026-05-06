# Runbook: Worktree-Backed Implementation

## Purpose

Provide the setup and cleanup flow for implementing AGX changes from dedicated git worktrees.

## When To Use

- the task will create commits or a PR
- the current workspace already has local changes
- multiple branches may be active in parallel
- the user wants the primary workspace to stay on its current branch

## Setup

```bash
git switch main
git pull --ff-only
git worktree add -b <agent>/<task-slug> ../agents-cli-<task-slug> main
git -C ../agents-cli-<task-slug> status --short --branch
```

## Implementation Flow

1. Work from the dedicated worktree path.
2. Keep the OpenSpec change updated as implementation learning becomes durable.
3. Run relevant validation before handoff.
4. Commit, push, and create a PR from the worktree branch when requested or when closure requires it.

## Cleanup

After merge or abandonment:

```bash
git worktree remove ../agents-cli-<task-slug>
git worktree prune
```

Before cleanup, confirm there are no unmerged commits you need to preserve.
