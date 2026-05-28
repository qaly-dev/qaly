---
title: Replay & CI integration
description: Run recorded tests deterministically in CI without an LLM.
---

## Local replay

```bash
# Run all tests in a .qaly.test file
qaly-test my_feature.qaly.test

# Run matching tests only
qaly-test my_feature.qaly.test --filter checkout

# Exit code 0 = all passed, 1 = any failed
```

## Parallel replay

```bash
# 4 workers = 4 emulators running tests in parallel
qaly-test my_feature.qaly.test --workers 4
```

Qaly auto-starts additional AVDs and distributes tests across them.

## CI configuration (GitHub Actions)

```yaml
name: Mobile tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Android emulator
        uses: reactivecircus/android-emulator-runner@v2
        with:
          api-level: 36
          script: |
            brew install qaly-dev/tap/qaly
            qaly-test my_feature.qaly.test
        env:
          ADB_BINARY: ${{ env.ANDROID_SDK_ROOT }}/platform-tools/adb
```

## Output format

```
Running 3 tests in my_feature.qaly.test

  ✓ add item to cart (0.9s)
  ✓ complete checkout (1.3s)
  ✗ verify order confirmation (2.1s)
    Step 3: wait_for "Order confirmed" — element not found after 5000ms

1 failed, 2 passed in 4.3s
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | All tests passed |
| 1 | One or more tests failed |
| 2 | Configuration error |
