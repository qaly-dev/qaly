---
title: .qaly.test file format
description: Complete reference for the YAML-lite .qaly.test file format.
---

## Full example

```yaml
app: com.example.myapp
clean_state: app_data
duplicate_labels: warn

[fixtures]
logged_in:
  snapshot: logged_in_state
empty_cart:
  app: com.example.myapp

- goal: add item to cart
  fixture: empty_cart

- goal: complete checkout
  fixture: logged_in

- goal: verify order confirmation
```

## Fields

### `app` (required)

Package name of the app under test.

### `clean_state` (optional)

How to reset device state before each test.

| Value | Behavior | Speed |
|---|---|---|
| `app_data` | `pm clear <package>` | ~200ms |
| `snapshot: <name>` | Restore named snapshot | 1–5s |
| `none` | No reset | 0ms |

### `duplicate_labels` (optional, default: warn)

| Value | Behavior |
|---|---|
| `warn` | Print warning; continue |
| `error` | Fail immediately |

### `[fixtures]` section

Named states for individual tests. Each fixture is a snapshot name or app data reset.

### Goals

Each `- goal:` line is one test case.

```yaml
- goal: add item to cart
- goal: complete checkout
  fixture: my_fixture   # override clean_state for this test only
```

## Recording paths

```
.qaly/<test-file-stem>/<NNN>_<goal-slug>.json
```
