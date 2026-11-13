---
name: github-upstream-workflow
description: Set up or maintain the ivaCAM GitHub fork, origin/upstream remotes, feature branches, or deliberate upstream synchronization using gh and Git.
---

# GitHub fork and upstream workflow

Use this skill for repository topology or upstream synchronization, not ordinary local code edits.

## Desired topology

```text
aalarchiv/ivacam  → upstream
user fork         → origin
local worktree
```

Use `gh` for GitHub account/repository operations and Git for local history/remotes.

## Setup workflow

1. Inspect `git status`, current branch, `git remote -v`, and `gh auth status` before changing anything.
2. Determine the authenticated GitHub account rather than guessing it.
3. Reuse an existing suitable fork when present.
4. Otherwise fork upstream while preserving history.
5. Ensure `origin` points to the user's fork and `upstream` points to `aalarchiv/ivacam`.
6. Verify remotes after changes.
7. Never force-push or rewrite upstream history.

Read `references/github-fork-playbook.md` for setup and sync details.

## Upstream synchronization

Do not merge/rebase upstream changes automatically merely because they exist. Inspect incoming changes, assess overlap with fork delta, choose merge/rebase deliberately, and run relevant validation afterward.

## Permission boundary

Follow the current task's explicit authorization for creating repositories, committing, pushing, or opening PRs. A skill does not grant permission to publish changes.
