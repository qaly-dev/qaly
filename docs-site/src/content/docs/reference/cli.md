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

### `qaly perceive --out <path>`

Write an annotated screenshot (labeled bounding boxes) to `<path>`. The CLI has
no standalone `screenshot` command — use `perceive --out`. (The MCP server does
expose a `screenshot` tool.)

### `qaly test <file> [options]`

Run tests from a `.qaly.test` file. Exits non-zero if any test fails.

**Options:**
- `--filter <string>` — only run tests whose goal contains this string
- `--workers <N>` — run N tests in parallel (qaly spawns extra emulators automatically)
- `--headless` — launch additional emulators headless
- `--avd <name>` — AVD name for additional workers
- `--strict-labels` — fail on duplicate actionable labels
- `--auto-heal` — fuzzy-match failing selectors and patch the recording (dev only — never in CI)
- `--debug` — write screenshot artifacts to `.qaly/<stem>/last-failure/` on failure
- `--debug-dir <path>` — same as `--debug` with a custom output path

### `qaly init`

Interactive setup wizard: detect prerequisites, install missing platform-tools,
write config, register `qaly-mcp` with your AI agent, and create a sample test.

### `qaly doctor`

Report environment status (adb, emulator, AVDs) without modifying anything.
Exits non-zero if a required tool is missing.

---

## `qaly-test`

Standalone test runner for CI. Identical to `qaly test` but designed to be called directly.

```bash
qaly-test my.qaly.test
qaly-test my.qaly.test --filter checkout
qaly-test my.qaly.test --workers 4
```

Exit codes: 0 = all passed, 1 = any failed, 2 = config error.
