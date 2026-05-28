# qaly

**Agent-first Android testing tool.** Record tests by having your AI agent tap through your app. Replay them in CI — no LLM, no flaky selectors.

## Install

**macOS / Linux (recommended)**
```sh
brew install qaly-dev/tap/qaly
```

**Manual download**
Download pre-built binaries from [github.com/qaly-dev/qaly/releases](https://github.com/qaly-dev/qaly/releases).

**Building from source**
`qaly-core` is closed-source. Building from this repository requires access
to the private engine. Use the pre-built binaries above.

## Quick start

1. Start your Android emulator.
2. Run `qaly init` — detects your setup and registers qaly-mcp with your AI agent.
3. Ask your agent to record a test: *"tap through the settings screen and save a test"*.
4. Replay it in CI: `qaly-test my-app.qaly.test`

## Docs

Full documentation at [qaly.dev/docs](https://qaly.dev/docs).

## License

MIT — see [LICENSE](LICENSE).
The engine (`qaly-core`) is closed-source and distributed as a compiled library.
