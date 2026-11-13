# Visual Language

## Purpose

The interface should feel like a professional desktop instrument rather than a website.

The user is manipulating meaningful work:

- geometry
- machine state
- toolpaths
- components
- projects
- datasets
- parameters

The UI should stay visually subordinate to that work.

## Design characteristics

### Professional
No playful ornamentation by default. The interface should communicate reliability and control.

### Compact
Use space efficiently. Do not equate readability with large spacing.

### Technical
Units, states, numeric values, and hierarchy must be unambiguous.

### Quiet
Neutral surfaces dominate. Strong emphasis is reserved for real importance.

### Contextual
Show the controls relevant to the current selection or task.

### Precise
Alignment, spacing, units, labels, icons, and state indicators should feel deliberate.

### Desktop-first
Design primarily for keyboard/mouse workflows and persistent multi-panel layouts.

## Influence

Use Adobe Creative Cloud desktop applications as a reference for:

- density
- panel hierarchy
- compact controls
- contextual properties
- subdued dark theme
- progressive disclosure

Do not replicate Adobe-specific branding or proprietary UI.

## Visual priority

Typical priority order:

1. workspace content
2. selected object / operation
3. current editable properties
4. primary action
5. warnings/errors
6. metadata/provenance
7. rarely used advanced controls

If metadata visually competes with the current editable value, the hierarchy is wrong.

## Surface hierarchy

Use subtle tonal differences rather than thick borders.

Conceptually:

```text
application background
  ↓
panel surface
  ↓
input/control surface
  ↓
hover/focus/selection
```

Keep the number of simultaneous surface levels low.

## Ornamentation

Avoid visual effects that do not encode meaning.

Do not use:

- decorative gradients
- large drop shadows
- glossy controls
- excessive translucency
- large rounded cards

Small shadows may be used for floating menus, tooltips, and modal elevation where needed.
