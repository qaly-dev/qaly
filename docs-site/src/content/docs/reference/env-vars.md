---
title: Environment variables
description: All environment variables supported by qaly, qaly-mcp, and qaly-test.
---

## Core

| Variable | Default | Description |
|---|---|---|
| `ADB_BINARY` | `adb` (from PATH) | Full path to the `adb` binary. |
| `ANDROID_SERIAL` | first device | ADB serial of the device to use. |
| `QALY_RUN_DIR` | unset | Save debug run logs to this directory. |

## Emulator (HybridController)

| Variable | Default | Description |
|---|---|---|
| `QALY_GRPC_PORT` | `8554` | Port for the emulator gRPC API. |
| `QALY_WS_PORT` | `7777` | Port for the qaly-agent WebSocket. |

## Feature flags

| Variable | Default | Description |
|---|---|---|
| `QALY_HEAL_LLM` | `0` | Set to `1` to enable LLM-based step healing as a last resort. |

## Testing

| Variable | Default | Description |
|---|---|---|
| `QALY_E2E` | `0` | Set to `1` to run end-to-end tests requiring a real emulator. |
| `QALY_BENCH` | `0` | Set to `1` to run performance benchmarks (requires `QALY_E2E=1`). |

## Legacy (sim-mcp)

| Old variable | New variable |
|---|---|
| `SIM_E2E` | `QALY_E2E` |
| `SIM_RUN_DIR` | `QALY_RUN_DIR` |
