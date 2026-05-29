FROM rust:1-bookworm

# Install adb via apt — native ARM64/AMD64 binary, no architecture mismatch on Apple Silicon.
RUN apt-get update && apt-get install -y --no-install-recommends \
        adb ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release

# Talk to the host's adb server (emulator runs on the host).
ENV ADB_SERVER_SOCKET=tcp:host.docker.internal:5037

# Default to the MCP server; override for the CLI.
CMD ["./target/release/qaly-mcp"]
