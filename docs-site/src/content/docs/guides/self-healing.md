---
title: Automatic self-healing
description: How Qaly recovers from UI changes without manual test maintenance.
---

## The problem

Mobile UIs change frequently — button labels get renamed, elements move, IDs change. Traditional test tools break and require manual updates.

## How Qaly heals

When a replay step fails to find its target, Qaly runs a healing cascade:

1. **Fuzzy label match** — case-insensitive substring match. `"Continue to payment"` heals to `"Continue"`.
2. **Accessibility label fallback** — tries `accessibilityLabel` if the primary label fails.
3. **Role + position match** — if a button moved but kept its role, tries the nth button of that role.

Steps 1–3 run locally in ~10ms with no network access.

## Automatic healing in practice

```
Replaying: tap "Proceed to checkout"
  → Element not found
  → Trying fuzzy match... found "Proceed to Checkout" (case change)
  → Healed automatically. Step passed.
```

## Manual healing via MCP

If automatic healing fails, you can heal a step manually:

```
The test failed at step 3. Please heal it.
```

The agent calls `heal_step(file, goal, step_index)`, which overwrites the step in the recording with the correct action.
