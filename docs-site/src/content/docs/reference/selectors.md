---
title: Selector syntax
description: How to reference UI elements in tap, fill, wait_for, and assert_visible.
---

## Selector types

### Element id: `e<N>`

The `id` field returned by `perceive()`. Valid for the current screen only.

```bash
qaly tap e42
```

### Resource ID suffix: `@<suffix>`

Matches any element whose `resource_id` ends with `suffix`. Most stable selector type.

```bash
qaly tap @btn_checkout
qaly fill @field_email "user@example.com"
```

### Full resource ID

Exact match on the full `resource_id` field.

```bash
qaly tap "com.example.shop:id/btn_add_to_cart"
```

### Label (default)

If the selector doesn't match above patterns, it's treated as a label. Tries:

1. Exact match on `label` (case-insensitive)
2. Exact match on `accessibility_label`
3. Substring match on `label`
4. Substring match on `accessibility_label`

```bash
qaly tap "Checkout"    # exact label match
qaly tap "check"       # substring — matches "Checkout"
```

## Ambiguity

If multiple elements match, Qaly returns an error with candidates:

```
Error: "Checkout" is ambiguous (3 matches):
  e12  button  "Checkout" (tappable)
  e34  button  "Checkout now" (tappable)
Retry with element id: tap e12
```
