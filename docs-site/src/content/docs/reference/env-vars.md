---
title: Environment variables
description: All environment variables supported by qaly, qaly-mcp, and qaly-test.
---

## Core

| Variable | Default | Description |
|---|---|---|
| `ADB_BINARY` | `adb` (from PATH) | Full path to the `adb` binary. Required on macOS if the SDK is not in PATH. |
| `EMULATOR_BINARY` | `emulator` (from PATH) | Full path to the `emulator` binary. Required when `test --workers` needs to spawn extra emulators. |
| `ANDROID_SERIAL` | first device | ADB serial of the device to use. |
| `AVD_NAME` | first AVD | AVD to launch when spawning emulators. |
| `ADB_SERVER_SOCKET` | unset | Override the adb server socket (e.g. `tcp:host.docker.internal:5037` for the Docker topology). Consumed by `adb` itself. |
| `QALY_RUN_DIR` | unset | Save debug run logs to this directory. |

## Emulator (HybridController)

| Variable | Default | Description |
|---|---|---|
| `QALY_GRPC_PORT` | `8554` | Port for the emulator gRPC API. |
| `QALY_WS_PORT` | `7777` | Port for the qaly-agent WebSocket. |

## Testing

| Variable | Default | Description |
|---|---|---|
| `QALY_E2E` | `0` | Set to `1` to run end-to-end tests requiring a real emulator. |
| `QALY_BENCH` | `0` | Set to `1` to run performance benchmarks (requires `QALY_E2E=1`). |
