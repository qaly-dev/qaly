---
title: Getting Started with the CLI
description: Use the qaly CLI to record, replay, and debug Android tests from the terminal.
---

## Prerequisites

- macOS or Linux
- Android SDK with a running emulator or connected device

## Install

```sh
brew install qaly-dev/tap/qaly
```

This installs two binaries:
- `qaly` — interactive CLI for recording and debugging
- `qaly-test` — standalone test runner for CI

> **Building from source** requires access to the private engine. Use the binary above.

## Set up environment

```bash
export ADB_BINARY="$HOME/Library/Android/sdk/platform-tools/adb"
# Optional: enable run logs for debugging
export QALY_RUN_DIR=runs
```

## Perceive the screen

```bash
qaly perceive
```

Returns a JSON tree of every visible UI element with id, role, label, bbox, and tappable flag.

## Tap, fill, assert

```bash
qaly tap "Search"
qaly fill "Search input" "cats"
qaly assert_visible "Search results"
```

Selectors are natural language labels, `e123` element ids, `@resource_id_suffix`, or full `com.pkg:id/element_id`.

## Run a test file

```bash
# Run all tests
qaly test my_feature.qaly.test

# Filter to one test
qaly test my_feature.qaly.test --filter "checkout"

# Run in parallel (4 emulators)
qaly test my_feature.qaly.test --workers 4
```

## Standalone test runner (for CI)

```bash
qaly-test my_feature.qaly.test
# exit 0 = all passed, exit 1 = any failed
```

## See also

- [.qaly.test file format](/reference/test-format)
- [Selector syntax](/reference/selectors)
- [Environment variables](/reference/env-vars)
