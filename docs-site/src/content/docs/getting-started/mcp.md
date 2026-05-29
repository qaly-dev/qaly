---
title: Getting Started with MCP
description: Set up Qaly as an MCP server so your AI agent can record and replay Android tests.
---

## Prerequisites

- macOS or Linux
- Android Studio installed with at least one AVD (Android Virtual Device) created
- Claude Code or any MCP-compatible AI agent

## Install

```sh
brew install qaly-dev/tap/qaly
```

This installs the `qaly-mcp` binary (MCP server) and the `qaly` CLI.

> **Building from source** requires access to the private engine. Use the binary above.

## Configure your AI agent

The fastest path is the interactive wizard — it detects your Android SDK and registers `qaly-mcp` automatically:

```bash
qaly init
```

Or register manually with Claude Code:

```bash
claude mcp add qaly qaly-mcp -s user
```

If you need to set environment variables (required when the Android SDK is not in `PATH`, which is the default on macOS), add them to the MCP server config:

```json
{
  "mcpServers": {
    "qaly": {
      "command": "qaly-mcp",
      "env": {
        "ADB_BINARY": "/Users/you/Library/Android/sdk/platform-tools/adb",
        "EMULATOR_BINARY": "/Users/you/Library/Android/sdk/emulator/emulator"
      }
    }
  }
}
```

> **Note:** Both `ADB_BINARY` and `EMULATOR_BINARY` must be set if the Android SDK is not in `PATH`. Without `ADB_BINARY`, Qaly cannot detect already-running emulators. Without `EMULATOR_BINARY`, spawning extra emulators for parallel runs fails.

## Start an emulator

```bash
# List your AVDs
emulator -list-avds

# Start one (headless, hardware-accelerated)
emulator -avd Medium_Phone_API_36 -no-window -no-audio -gpu swiftshader_indirect &

# Wait until booted
adb wait-for-device shell getprop sys.boot_completed
# Returns "1" when ready
```

## Record your first test

Open Claude and ask it to record a test:

```
Record a test that: opens the Settings app, goes to About Phone, and asserts the Android version is visible.
```

Claude will call `perceive()` to see the screen, then `begin_test`, tap through the steps, and call `end_test`. The recording is saved to `.qaly/settings/001_assert_android_version.json`.

## Replay without AI

```bash
qaly-test settings.qaly.test
```

Output:
```
✓ assert android version is visible (1.2s)
All 1 tests passed in 1.2s.
```

## Available tools

See [MCP tools reference](/reference/mcp-tools) for all 20 tools with parameters and examples.
