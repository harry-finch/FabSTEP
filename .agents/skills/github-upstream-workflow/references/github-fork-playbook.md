# GitHub fork playbook

Commands are examples; inspect CLI help when syntax/version matters.

## Baseline inspection

```bash
gh --version
git --version
gh auth status
git status
git branch --show-current
git remote -v
```

Determine the authenticated login with `gh` rather than inferring it from local paths.

## Fork setup

Preferred state:

```text
origin   git@github.com:<user>/ivacam.git
upstream https://github.com/aalarchiv/ivacam.git
```

SSH/HTTPS may differ according to authenticated setup. Do not create a disconnected repository with fresh history when a normal GitHub fork is suitable.

## Feature work

Use a focused branch such as `feature/shopbot-mvp`. Keep default/upstream branches clean.

## Pulling upstream

Before integrating upstream: fetch, review commits/diff, identify conflicts with documented fork delta, select merge/rebase according to project policy, run build/tests/validation, and update project status/docs if architecture or behavior changed.

Never resolve a CAM semantic conflict by choosing a side mechanically.
