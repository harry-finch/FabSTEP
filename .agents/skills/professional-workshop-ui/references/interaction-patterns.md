# Interaction Patterns

## Context first

Do not ask the user to re-enter information the application already knows.

Reuse current context:

- selected object
- active machine
- active material
- current tool
- current operation
- document state

Only ask for information requiring a decision or confirmation.

## Progressive disclosure

Default view: frequent controls only.

Advanced view: rare, expert, diagnostic, provenance, or tuning controls.

## Explicit adaptation

When software adapts a requested operation to a machine or environment:

- do not hide material changes
- show a compact state
- allow detail on demand

Example:

```text
Helix
⚠ Linearized for ShopBot
```

## Preserve intent

Where possible distinguish:

- user intent
- derived/resolved representation

Do not destroy the user's original choice merely because the active machine requires a different representation.

## Confirmation

Require confirmation for:

- destructive operations
- unsafe machine actions
- irreversible data replacement
- promotion to high-trust states where appropriate

Do not require confirmation for routine reversible editing.

## Keyboard support

Professional desktop workflows should support:

- tab navigation
- enter/escape in dialogs
- standard copy/paste
- common platform shortcuts
- predictable focus

## Tooltips

Use tooltips as secondary teaching.

Do not force novice explanations into every panel.

## Error recovery

Errors should answer:

1. what failed?
2. why?
3. what can the user do next?

Prefer concise actionable copy.

## Persistent state

Preserve useful UI state when appropriate:

- expanded sections
- inspector width
- last selected tab
- panel visibility

## Beginner/expert tension

Prefer:

- sensible defaults
- progressive disclosure
- advanced sections
- contextual help

Avoid creating two entirely different interaction models unless truly necessary.
