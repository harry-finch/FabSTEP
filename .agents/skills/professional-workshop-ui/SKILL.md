---
name: professional-workshop-ui
description: Maintain the compact professional desktop design language and shared UI conventions for FabCAM.
---

# Professional Workshop UI

A reusable UX/UI skill for compact, professional desktop applications used in technical, creative, engineering, fabrication, CAD/CAM, electronics, machine-control, and data-heavy workflows.

This skill defines the shared visual and interaction language. Product-specific skills should add domain rules on top of it.

## Core identity

Professional Workshop UI is:

- professional
- compact
- technical
- quiet
- contextual
- precise
- desktop-first
- keyboard/mouse-first
- dense but readable

Primary visual influence: professional Adobe Creative Cloud desktop applications.

Borrow principles such as:

- compact inspector panels
- restrained dark neutral surfaces
- contextual properties
- subtle dividers
- strong typographic hierarchy
- compact numeric controls
- progressive disclosure
- small consistent icons
- restrained semantic color
- efficient use of vertical space

Do not copy:

- Adobe branding
- Adobe logos
- proprietary icons
- exact component geometry
- exact panel layouts

## Fundamental rule

The user's work is primary.

The interface should support the workspace rather than compete with it.

Prefer:

- canvas/workspace dominance
- compact panels
- contextual inspectors
- lightweight toolbars
- discreet status information

Avoid:

- SaaS-dashboard aesthetics
- mobile-first controls
- oversized cards
- decorative gradients
- large colored surfaces
- excessive rounding
- duplicated information
- large blocks of explanatory text in normal workflows

## Application anatomy

Use this structure where appropriate:

```text
┌──────────────────────────────────────────────┐
│ Toolbar / document tabs                      │
├───────┬──────────────────────────┬───────────┤
│       │                          │           │
│ Tools │       Workspace          │ Inspector │
│       │                          │           │
├───────┴──────────────────────────┴───────────┤
│ Status / contextual information              │
└──────────────────────────────────────────────┘
```

Not every application needs every region, but use the same vocabulary consistently.

## Contextual inspector principle

Prefer object/context-sensitive properties.

Examples:

```text
Object selected
→ object properties

Operation selected
→ operation properties

Machine selected
→ machine properties

Nothing selected
→ document/project properties
```

Do not expose every possible setting at once.

## Progressive disclosure

Frequently used controls stay visible.

Advanced, diagnostic, provenance, and uncommon controls should be available through:

- collapsible sections
- disclosure rows
- tooltips
- advanced panels
- secondary dialogs when genuinely necessary

Power should not make the default interface look complicated.

## Density

Design for desktop mouse/keyboard use.

Default guidance:

- control height: 28–32 px
- toolbar icons: 16–18 px
- small icons: 14–16 px
- panel padding: 8–12 px
- row gaps: 4–8 px
- section gaps: 12–16 px
- major gaps: 20–24 px
- corner radius: 3–5 px

These are reference values, not mandatory pixel constants. Prefer shared design tokens over local magic numbers.

One property should usually occupy one row.

Prefer:

```text
Diameter      [ 6.00 ] mm
Feed          [ 3200 ] mm/min
Depth         [ 4.00 ] mm
```

over multi-line form layouts.

## Color philosophy

Color should communicate state, hierarchy, or action.

Do not use color merely to decorate.

Typical semantic roles:

- blue: primary action / selection / focus
- green: success / validated / ready
- amber: warning / adaptation / caution
- red: blocking error / dangerous state
- cyan: optional technical information
- purple: optional automation/special feature

Use small touches of semantic color rather than large colored containers.

Example:

```text
✓ Validated
⚠ Adapted for machine
⛔ Cannot generate safely
```

The amount of visual emphasis should match severity.

## App identity color

Applications may use a restrained secondary identity accent.

Example:

```text
FabCAM          orange
KiPartManager   violet
FabSBP          cyan
```

Identity accents must not replace semantic colors.

Red still means error.
Amber still means warning.
Green still means success/validated.

## Icons

Use one consistent icon library across the application family.

Preferred default:

- Lucide

Tabler Icons is an acceptable alternative if a project already uses it consistently.

Rules:

- use SVG/icon components, not emoji
- keep stroke weight visually consistent
- use 14–16 px for secondary actions
- use 16–18 px for toolbars
- use icon-only controls only for familiar actions
- icon-only controls require tooltips
- domain-specific actions should usually use icon + text

Avoid mixing icon libraries.

## Typography

Keep typography hierarchy limited and predictable.

Suggested levels:

1. panel / object title
2. section title
3. field label
4. field value
5. secondary text
6. metadata/status

Section labels may use:

- small uppercase
- semibold
- muted light gray
- restrained letter spacing

Metadata should never compete visually with primary values.

## Layout

Prefer:

- spacing
- alignment
- thin dividers
- subtle surface contrast

over:

- nested cards
- heavy borders
- boxes inside boxes
- decorative containers

Use bordered surfaces only where they communicate something real:

- focus
- selection
- validation state
- error
- active editing
- meaningful grouping

## Numeric controls

Technical applications should align numeric properties consistently.

Preferred conceptual columns:

```text
LABEL        VALUE        UNIT        STATUS
```

Example:

```text
Spindle      [ 18000 ]    rpm         Suggested
Feed         [  3600 ]    mm/min      Modified
Plunge       [   900 ]    mm/min      Suggested
Depth        [     4 ]    mm
```

Keep value widths, units, and status alignment consistent.

## Button hierarchy

Use at most three visual levels:

### Primary
One dominant action in a local context.

### Secondary
Normal supporting actions.

### Quiet / toolbar
Low-emphasis actions, icon buttons, or tertiary controls.

Avoid multiple competing primary buttons.

## Selection

Selection should be clear but restrained.

Prefer:

- thin accent border
- subtle background tint
- small accent indicator

Avoid turning large areas fully blue unless the platform convention strongly requires it.

## Warnings and errors

Information:
small icon + restrained text

Warning:
amber icon/text, compact

Blocking error:
stronger treatment, explicit explanation, clear next action

Do not use large warning cards for expected, safe adaptations.

## Tooltips

Use tooltips heavily for:

- icon-only controls
- technical terms
- calculated values
- machine states
- shorthand labels
- rarely used actions

Tooltips should educate without cluttering the default layout.

## Copy style

Prefer concise professional UI copy.

Example:

Prefer:

```text
✓ Available on this machine
```

over:

```text
The selected tool is currently available in the active machine.
```

Long explanations belong in help, tooltips, or secondary detail.

## Theme

Dark theme is the primary visual reference.

Prefer:

- very dark neutral application background
- slightly lighter panels
- slightly contrasted inputs
- subtle separators
- high-but-not-harsh text contrast
- muted secondary text

Avoid pure black everywhere and avoid high-glare white-on-black treatment.

A light theme may exist later, but should preserve the same hierarchy and density.

## Anti-patterns

Do not introduce:

- SaaS dashboard aesthetics
- mobile/touch-first spacing
- giant cards
- excessive border radii
- decorative gradients
- large colored surfaces
- random icon libraries
- emoji as UI icons
- duplicated information
- modal dialogs for routine actions
- hidden critical machine state
- badges for ordinary metadata
- oversized explanatory text in inspectors

## Accessibility

Compact does not mean inaccessible.

Maintain:

- readable contrast
- visible keyboard focus
- proper labels
- predictable tab order
- non-color-only state communication
- reasonable hit targets
- tooltips/accessible names for icon-only buttons

## Reuse across applications

Use this skill for the shared UI language.

Pair it with product-specific UX skills.

Examples:

```text
FabCAM
$professional-workshop-ui
$cam-ux-ui

KiPartManager
$professional-workshop-ui
$kipart-ux

FabSBP
$professional-workshop-ui
$machine-control-ux
```

The shared skill defines the visual language.

The product-specific skill defines domain behavior, safety, workflows, and specialist interaction patterns.

## Implementation rule

Before adding a new local style:

1. inspect existing shared tokens/components
2. reuse or extend them where appropriate
3. avoid one-off CSS when a shared pattern exists
4. preserve density and visual hierarchy
5. verify the change against the anti-pattern list

## Reference files

Read as needed:

- `references/visual-language.md`
- `references/layout-and-density.md`
- `references/color-system.md`
- `references/icons.md`
- `references/components.md`
- `references/interaction-patterns.md`
- `references/anti-patterns.md`
