# Anti-patterns

Avoid the following unless a product-specific requirement explicitly justifies them.

## SaaS dashboard styling

Avoid:

- giant cards
- marketing-style metrics
- oversized headings
- large whitespace
- rounded rectangles around every section

## Mobile-first controls

Avoid:

- 48–56 px control heights everywhere
- huge touch targets at the expense of density
- mobile navigation patterns on desktop

## Nested boxes

Avoid:

```text
card
  card
    bordered section
      bordered input group
```

Prefer hierarchy through spacing and dividers.

## Decoration-only color

Do not use strong color just because a section exists.

Color should encode state, action, identity accent, or visualization meaning.

## Badge explosion

Do not turn every metadata value into a pill.

Prefer plain secondary text.

## Icon inconsistency

Do not mix unrelated icon styles or emoji without a deliberate reason.

## Redundant context

Do not repeatedly show machine/tool/material/operation if the user already knows the context and does not need to verify it.

Move secondary context into collapsed detail when needed.

## Modal overload

Do not open a modal for every edit.

Use the inspector for normal property editing.

## Hidden critical state

Never hide safety-critical information.

## Excessive explanation

Avoid paragraphs inside dense inspectors.

Use concise status, tooltip, info disclosure, or help.

## Silent fallback

Do not silently convert the user's requested technical behavior into something materially different.

If adaptation is necessary:

- resolve deterministically
- surface the adaptation
- preserve intent when possible

## Local CSS drift

Avoid one-off styles for each screen.

Prefer shared tokens, components, patterns, spacing, and semantic states.
