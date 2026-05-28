---
title: Framework compatibility
description: How Qaly works with Native Android, Compose, React Native, and Flutter.
---

## Compatibility matrix

| Framework | Hierarchy | Stable selector | Notes |
|---|---|---|---|
| Native Android | ✅ | `android:id` → `@resource_id` | Full support out of the box |
| Jetpack Compose | ✅ | `Modifier.testTag` → `@tag` | Requires `testTag` for stable selectors |
| React Native | ✅ | `testID` prop → `@id` | RN maps `testID` to native `resource-id` automatically |
| Flutter (3.13+) | ✅ | `Semantics(identifier:)` → `@id` | Requires `identifier:` parameter |
| Flutter (custom canvas) | ⚠️ | None | CustomPainter has no a11y nodes |
| Games / no a11y | ❌ | None | No accessibility tree |

## Checking what Qaly sees

```bash
qaly perceive | python3 -m json.tool | grep -E '"label"|"resource_id"|"id"'
```

This shows every element's label, resource_id, and element id.
