# qaly

Playwright MCP, but for Android. An agent-first tool for controlling Android emulators — an AI agent can see the screen, tap elements, type text, and assert UI state, all through a clean MCP interface.

## How it works

```
Agent (Claude, etc.)
      │  MCP tools
      ▼
  qaly-mcp  ◄──── or ────►  qaly  (terminal / scripts)
      │                       │
      └──────────┬────────────┘
                 ▼
          qaly-daemon (pre-compiled)
                 │
                 ▼
        Android emulator (ADB)
```

`qaly-mcp` and the `qaly` CLI communicate with `qaly-daemon` over gRPC on `localhost:50052`. The daemon is distributed as a pre-compiled binary via the Homebrew tap and manages device sessions, test recording, replay, and self-healing.

## Install

```bash
brew install qaly-dev/tap/qaly
# installs qaly, qaly-mcp, qaly-test, and qaly-daemon on PATH
```

Or grab a binary from [github.com/qaly-dev/qaly/releases](https://github.com/qaly-dev/qaly/releases).

## Quick start

### Prerequisites

- Android SDK with `adb`
- A running Android emulator

```bash
# Start your emulator
$HOME/Library/Android/sdk/emulator/emulator \
  -avd Medium_Phone_API_36.1 \
  -no-window -no-audio -no-snapshot -gpu swiftshader_indirect &

# Wait for boot
adb wait-for-device shell 'while [[ "$(getprop sys.boot_completed)" != "1" ]]; do sleep 1; done'
```

### Setup

```bash
qaly init
# Interactive wizard: detects prerequisites, registers qaly-mcp with your AI agent,
# creates a sample test.
```

## MCP server

`qaly-mcp` speaks the Model Context Protocol over stdio. Wire it into any MCP-compatible agent.

### Claude Desktop / Claude Code

```bash
# The wizard registers qaly-mcp automatically
qaly init

# Or register manually
claude mcp add qaly qaly-mcp -s user
```

Or add to your MCP config manually:

```json
{
  "mcpServers": {
    "qaly": {
      "command": "/path/to/qaly-mcp",
      "env": {
        "ADB_BINARY": "/Users/you/Library/Android/sdk/platform-tools/adb",
        "EMULATOR_BINARY": "/Users/you/Library/Android/sdk/emulator/emulator"
      }
    }
  }
}
```

> **Important:** Both `ADB_BINARY` and `EMULATOR_BINARY` must be set if the Android SDK is not in `PATH` (this is the default on macOS).

### Tools exposed

**Perception & interaction**

| Tool | Description |
|------|-------------|
| `perceive` | Capture screen → element tree (JSON) |
| `screenshot` | Raw PNG screenshot |
| `launch_app` | Launch app by package ID |
| `tap` | Tap element by ID (`e3`), label, or `@resource_id` suffix |
| `type_text` | Type into the focused field |
| `fill` | Tap a target then type text |
| `swipe` | Swipe `up` / `down` / `left` / `right` |
| `press_key` | Press `back` / `home` / `enter` / `recent` |
| `wait_for` | Block until a label appears (default 5 s timeout) |
| `assert_visible` | Assert a label is visible; error if not |
| `shell` | Run a raw `adb shell` command |

**Test recording & replay**

| Tool | Description |
|------|-------------|
| `begin_test` | Start recording a test; agent actions are captured |
| `end_test` | Stop recording; saves `.json` recording alongside the `.test` file |
| `run_test` | Replay a single recorded goal deterministically (no LLM) |
| `run_tests` | Replay all goals in a `.test` file; supports `workers`, `headless`, `avd_name` |
| `heal_step` | Overwrite a step in a recording after manually correcting a failure |

**Snapshot management**

| Tool | Description |
|------|-------------|
| `snapshot_save` | Save a named emulator snapshot |
| `snapshot_restore` | Restore a named snapshot |
| `snapshot_list` | List all saved snapshots |
| `snapshot_delete` | Delete a named snapshot |

### Typical agent loop

```
perceive()                   → see current screen + element IDs
launch_app(pkg)              → open the app under test
perceive()                   → refresh after launch
tap("@tab_menu_alarm")       → interact by resource_id suffix (unambiguous)
tap("Add alarm")             → interact by label
fill("e5", "07")             → interact by element ID
assert_visible("7:30 AM")   → verify outcome
```

## CLI

Same commands as the MCP server, available as a terminal tool or in shell scripts.

```bash
export ADB_BINARY=$HOME/Library/Android/sdk/platform-tools/adb

qaly launch com.google.android.deskclock
qaly perceive --json
qaly perceive --out screen.png
qaly tap "Add alarm"
qaly tap "@tab_menu_alarm"         # by resource_id suffix
qaly fill e5 "07"
qaly wait_for "7:30 AM" --timeout-ms 8000
qaly assert_visible "7:30 AM"
qaly shell "dumpsys battery"

# Setup & environment
qaly init                          # interactive wizard: detect, install, register MCP, sample test
qaly doctor                        # read-only environment status (no changes)

# Test replay
qaly test tests/clock.qaly.test
qaly test tests/clock.qaly.test --filter "create alarm" --workers 2 --headless
qaly test tests/clock.qaly.test --auto-heal --debug    # dev iteration only — never in CI
```

`qaly test` flags: `--filter` `--workers N` `--headless` `--avd NAME`
`--strict-labels` `--auto-heal` `--debug` `--debug-dir PATH`.

## Test recording & replay

Write a `.qaly.test` file describing what you want to test:

```yaml
# tests/clock.qaly.test
app: com.google.android.deskclock
clean_state: app_data      # pm clear before each test (fast, ~200ms)

- goal: Create a 7:30 AM alarm and verify it appears in the list
- goal: Delete all alarms and verify the list is empty
```

**First run** — ask the agent to record (via MCP or CLI):
> "Record the test 'Create a 7:30 AM alarm' from tests/clock.qaly.test"

The agent explores the app, uses `begin_test` / `end_test`, and the recording is saved to `tests/.qaly/clock/`.

**Subsequent runs** — deterministic replay, no LLM needed:

```bash
qaly-test tests/clock.qaly.test
```

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 PASS   Create a 7:30 AM alarm and verify it appears      (1.8s)
 FAIL   Delete all alarms                                  (2.1s)
        → step 4 failed: ElementNotFound "Delete"
        → suggestions: "Remove", "Clear"
 SKIP   Make recovery password                             (no recording)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3 tests  ·  1 passed  ·  1 failed  ·  1 unrecorded
```

### Self-healing

When a selector fails during replay with `--auto-heal`, qaly attempts automatic
recovery through three strategies, in order:
1. **Normalize** — strip diacritics, lowercase, collapse whitespace/punctuation.
2. **Fuzzy match** — Levenshtein edit-distance against visible labels.
3. **OCR fallback** — locate the label visually via `tesseract` and tap by coordinates (requires `tesseract` installed; skipped with a one-time warning if absent).

A successful heal patches the recording in place.
Use `--auto-heal` during development only — never in CI.

For manual recovery, the agent calls **`heal_step(file, goal, step_index)`** to
overwrite a broken step in the recording after correcting it interactively.

### Parallel execution

```bash
# Run tests across 3 emulators; qaly manages extra emulators automatically
qaly test tests/suite.test --workers 3 --headless --avd Medium_Phone_API_36.1
```

### Fixtures

```yaml
app: com.example
clean_state: disabled   # opt out of auto-reset

[fixtures]
logged_in:
  snapshot: logged_in_state

- goal: Do something that requires login
  fixture: logged_in
```

## Selectors

Elements can be targeted by:

| Syntax | Resolves by |
|---|---|
| `e3` | Short element ID (from `perceive`) |
| `"Add alarm"` | Label (exact, case-insensitive; then substring) |
| `@tab_menu_alarm` | `resource_id` suffix (unambiguous when labels conflict) |
| `com.example:id/btn` | Full `resource_id` |

When a label matches multiple elements, qaly returns `AmbiguousLabel` with candidates so the agent can retry with an ID.

## Environment variables

| Variable | Description |
|----------|-------------|
| `ADB_BINARY` | Path to `adb`. Defaults to `adb` in `PATH`. **Required on macOS if the SDK is not in PATH.** |
| `EMULATOR_BINARY` | Path to the `emulator` binary. Required when `run_tests` needs to spawn additional emulators. |
| `ANDROID_SERIAL` | Device serial (e.g. `emulator-5554`). Optional if one device is attached. |
| `ADB_SERVER_SOCKET` | Override adb server socket (e.g. `tcp:host.docker.internal:5037` for Docker). |
| `AVD_NAME` | Default AVD name when spawning managed emulators. |

## Documentation

Full documentation at [qaly.dev](https://qaly.dev).

## License

MIT
