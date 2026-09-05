# blocksign-mcp-server — Model Context Protocol server for AI agents.
# Same shape as services/ai-gateway/Dockerfile.

ARG RUST_VERSION=1.94
ARG DEBIAN_VERSION=bookworm

FROM rust:${RUST_VERSION}-slim-${DEBIAN_VERSION} AS builder

WORKDIR /build

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        build-essential \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release --locked --bin blocksign-mcp-server

FROM debian:${DEBIAN_VERSION}-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        tini \
        wget \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /app --shell /usr/sbin/nologin blocksign

WORKDIR /app
COPY --from=builder /build/target/release/blocksign-mcp-server /app/blocksign-mcp-server

USER blocksign

ENV MCP_BIND=0.0.0.0:8200
EXPOSE 8200

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/app/blocksign-mcp-server"]
