# Contributing to qaly

Thanks for your interest! Contributions are welcome for the open-source crates (`qaly-mcp`, `qaly`, `qaly-test`), docs, and examples.

## Building

Building from source requires access to the private `qaly-core` engine.
If you don't have access, you can still contribute to docs and examples — no build required.

```sh
git clone https://github.com/qaly-dev/qaly
cd qaly
# Requires SSH access to BigBangStudios/sim-mcp for qaly-core
cargo build
```

## Git hooks

A pre-push hook runs `cargo clippy --workspace -- -D warnings` to catch lint errors before they reach CI. Enable it once after cloning:

```sh
git config core.hooksPath .githooks
```

## Issues and PRs

- Open issues at [github.com/qaly-dev/qaly/issues](https://github.com/qaly-dev/qaly/issues)
- For engine bugs (auto-heal, perception, replay), open an issue — we'll triage internally
