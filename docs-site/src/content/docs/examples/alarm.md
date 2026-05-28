---
title: Create an alarm (Clock app walkthrough)
description: End-to-end example recording and replaying a test that creates an alarm in the Android Clock app.
---

## 1. Create the `.qaly.test` file

```yaml
# clock.qaly.test
app: com.google.android.deskclock
clean_state: app_data

- goal: create alarm at 7:30 AM
```

## 2. Record with your AI agent

Ask Claude:

```
Record all tests in clock.qaly.test.
```

Claude will navigate the app and save the recording to `.qaly/clock/001_create_alarm_at_7_30_am.json`.

## 3. Replay

```bash
qaly-test clock.qaly.test
```

Output:
```
✓ create alarm at 7:30 AM (0.9s)
All 1 tests passed in 0.9s.
```

## 4. Add to CI

```yaml
- name: Run alarm test
  run: qaly-test clock.qaly.test
  env:
    ADB_BINARY: ${{ env.ANDROID_SDK_ROOT }}/platform-tools/adb
```
