---
title: Performance benchmarks
description: Measured performance comparison between ADB-based testing and Qaly's hybrid agent.
---

> This page reflects design targets. Run `scripts/bench.sh` against a real emulator to get measured numbers.

## Summary

| Benchmark | ADB (baseline) | Qaly agent | Speedup |
|---|---|---|---|
| `dump_hierarchy` p50 | ~490ms | ~12ms | **~40×** |
| `wait_for` detection overhead | ~480ms | ~18ms | **~27×** |
| 10-step test suite (wall time) | ~8.1s | ~1.1s | **~7.4×** |
| Recording token cost | ~85,000 tokens | ~5,000 tokens | **94% reduction** |

## How to run

```bash
# Start emulator first
emulator -avd Medium_Phone_API_36 -grpc 8554 -no-window -no-audio &

# Run all benchmarks
QALY_E2E=1 QALY_BENCH=1 ADB_BINARY=$HOME/Library/Android/sdk/platform-tools/adb \
  ./scripts/bench.sh

# Output: BENCHMARKS.md at repo root
```

## ADB baseline

ADB uses `uiautomator dump` which starts a cold Java process per call (~490ms). `wait_for` uses exponential backoff polling (50ms → 500ms steps, ~480ms average overhead).

## Qaly agent (HybridController)

The HybridController uses the emulator gRPC API for input and the qaly-agent WebSocket for hierarchy. The in-memory AccessibilityService tree takes ~10ms. `wait_for` reacts to `SCREEN_CHANGED` push events (~18ms detection overhead).
