# Layout and Density

## Desktop density

Default reference values:

```text
Control height       28–32 px
Toolbar icon         16–18 px
Small icon           14–16 px
Panel padding        8–12 px
Row gap              4–8 px
Section gap          12–16 px
Major gap            20–24 px
Corner radius        3–5 px
```

Do not treat these as mandatory constants. Prefer shared tokens.

## Grid discipline

Use aligned rows for technical properties.

Preferred:

```text
LABEL           VALUE       UNIT       STATUS
Diameter        [ 6.00 ]    mm
Feed            [ 3200 ]    mm/min     Modified
Plunge          [  800 ]    mm/min     Suggested
```

Benefits:

- faster scanning
- less vertical space
- better numeric comparison
- more professional appearance

## Sections

Prefer:

```text
CUTTING PARAMETERS
────────────────────────
Spindle ...
Feed ...
Plunge ...

ENTRY
────────────────────────
Strategy ...
Angle ...
```

over nested cards.

## Inspector width

A technical inspector should remain usable at realistic desktop widths.

When space becomes tight:

1. preserve label/value/unit readability
2. reduce metadata emphasis
3. move status to secondary line only when necessary
4. avoid truncating numeric values or units
5. use tooltips for long labels

## Workspace priority

Do not let side panels unnecessarily consume workspace.

Allow:

- collapsible side panels
- resizable inspectors
- compact section headers
- persistent but narrow toolbars

## Responsive behavior

This design language is desktop-first, not mobile-first.

Responsive behavior should preserve professional panel structure rather than convert everything into giant stacked cards.

## Scrolling

Inspector panels may scroll vertically.

Avoid independent nested scroll areas unless the content truly requires them.

## Dialogs

Use dialogs for:

- focused decisions
- import/export
- destructive confirmations
- multi-step configuration that cannot fit naturally in the inspector

Do not use a modal for routine property editing.
