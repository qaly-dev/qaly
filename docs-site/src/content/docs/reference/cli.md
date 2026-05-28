---
title: qaly CLI reference
description: Complete reference for the qaly and qaly-test command-line tools.
---

## `qaly`

### `qaly perceive`

Print the current screen element tree as JSON.

### `qaly tap <target>`

Tap an element. Accepts label, `e42`, or `@resource_id`.

### `qaly fill <target> <text>`

Clear and type text into an element.

### `qaly assert_visible <target>`

Exit 0 if visible, 1 if not.

### `qaly wait_for <target>`

Wait up to 5s for element to appear.

### `qaly screenshot [--annotated]`

Save screenshot to `screenshot.png`.

### `qaly test <file> [options]`

Run tests from a `.qaly.test` file.

**Options:**
- `--filter <string>` — only run tests whose goal contains this string
- `--workers <N>` — run N tests in parallel
- `--headless` — launch additional emulators headless
- `--avd <name>` — AVD name for additional workers
- `--strict-labels` — fail on duplicate actionable labels

### `qaly migrate`

Renames `.sim/` directories to `.qaly/` in the current tree (for upgrading from sim-mcp).

---

## `qaly-test`

Standalone test runner for CI. Identical to `qaly test` but designed to be called directly.

```bash
qaly-test my.qaly.test
qaly-test my.qaly.test --filter checkout
qaly-test my.qaly.test --workers 4
```

Exit codes: 0 = all passed, 1 = any failed, 2 = config error.
