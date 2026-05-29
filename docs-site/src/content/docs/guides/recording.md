---
title: Recording your first test
description: How to record a test suite with an AI agent via MCP or the CLI.
---

## What is a recording?

A recording is a concrete sequence of actions that replays a test without an AI. It is stored as a JSON file in `.qaly/<suite>/<NNN>_<slug>.json` alongside your `.qaly.test` file.

## The `.qaly.test` file

Before recording, create a `.qaly.test` file describing what you want to test:

```yaml
# my_feature.qaly.test
app: com.example.myapp
clean_state: app_data   # reset app data before each test (~200ms)

- goal: add item to cart
- goal: complete checkout
- goal: verify order confirmation
```

Each `goal` becomes one test case. Goals are natural-language descriptions — the AI agent uses them to know what to record.

## Recording via MCP (recommended)

Ask your AI agent:

```
Record all tests in my_feature.qaly.test.
```

The agent will:
1. Call `begin_test(file: "my_feature.qaly.test", goal: "add item to cart")`
2. Call `perceive()` to see the screen
3. Navigate the app, calling `tap`, `fill`, `wait_for`, `assert_visible` as needed
4. Call `end_test()` to save the recording
5. Repeat for each goal

Recordings are saved to `.qaly/my_feature/`. They are gitignored by default.

## Clean state options

| Value | Behavior | Speed |
|---|---|---|
| `app_data` | `pm clear <package>` — wipes app data + cache | ~200ms |
| `snapshot: name` | Restore a named emulator snapshot | 1–5s |
| `none` | No reset between tests | 0ms |

`app_data` is the recommended default for most test suites.
