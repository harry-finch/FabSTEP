# Color System

## Principle

Color carries meaning. Neutral surfaces should dominate.

## Semantic colors

### Primary accent

Usually blue.

Use for:
- primary action
- selection
- focus
- active mode
- links where appropriate

Do not color every interactive element strongly.

### Success / validated

Green.

Use for:
- validated state
- successful completion
- ready/healthy state

Prefer small indicators rather than large green containers.

### Warning / adaptation

Amber.

Use for:
- caution
- machine adaptation
- stale recommendation
- non-blocking mismatch
- attention required

### Error / unsafe

Red.

Use for:
- blocking error
- unsafe state
- destructive action emphasis where appropriate

### Informational technical accent

Cyan may be used sparingly for technical information.

### Automation / special feature

Purple may be used sparingly for automation/AI/special workflows if the product needs a distinct semantic channel.

## Application identity color

An application may define a secondary identity accent.

Example:

```text
FabCAM          orange
KiPartManager   violet
FabSBP          cyan
```

Rules:

- use sparingly
- keep semantic colors universal
- do not make large panels app-colored
- suitable places: logo, tiny accent line, active document marker, visualization overlays

## Dark surfaces

Use several neutral dark levels.

Conceptually:

```text
background
panel
control
hover
selected
```

Avoid pure black as the default application background.

## Text

Use at least:
- primary text
- secondary text
- disabled text
- inverse text if needed

Metadata should be visibly secondary.

## Color accessibility

Do not rely on color alone.

Pair semantic color with icon, text, shape, or state label.
