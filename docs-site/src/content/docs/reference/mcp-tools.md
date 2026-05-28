---
title: MCP tools reference
description: All 20 Qaly MCP tools with parameters and examples.
---

## Perception

### `perceive()`

Returns the current screen as a JSON element tree (text-only by default).

```json
{
  "foreground": "com.example.app",
  "elements": [
    { "id": "e1", "role": "button", "label": "Checkout", "resource_id": "btn_checkout", "bbox": [20, 100, 200, 140], "tappable": true }
  ],
  "warnings": []
}
```

### `screenshot(annotated?)`

Returns a PNG screenshot. `annotated: true` draws element bounding boxes and ids.

---

## Actions

### `tap(target)`

Taps the element matching `target`. See [Selector syntax](/reference/selectors).

### `fill(target, value)`

Clears and types `value` into the element matching `target`.

### `type_text(text)`

Types text at the current cursor position without clearing first.

### `swipe(direction)`

Swipes in a direction: `"up"`, `"down"`, `"left"`, `"right"`.

### `press_key(key)`

Sends a key event: `"BACK"`, `"HOME"`, `"ENTER"`, etc.

### `launch_app(package?)`

Launches the session app. Pass `package` to override.

### `shell(command)`

Runs a raw ADB shell command.

---

## Assertions

### `wait_for(target, timeout_ms?)`

Waits until `target` is visible. Default timeout: 5000ms.

### `assert_visible(target)`

Fails immediately if `target` is not currently visible.

---

## Test lifecycle

### `begin_test(file, goal)`

Starts recording a test.

### `end_test()`

Finishes recording and saves to `.qaly/<suite>/<NNN>_<slug>.json`.

### `run_test(file, goal)`

Replays a single recorded test.

### `run_tests(file, filter?, workers?)`

Replays all tests. Optional filter and worker count.

---

## Snapshots

### `snapshot_save(name)` / `snapshot_restore(name)` / `snapshot_list()` / `snapshot_delete(name)`

Manage named emulator snapshots.

---

## Self-healing

### `heal_step(file, goal, step_index)`

Manually heals a recording step by overwriting it with the agent's next action.
