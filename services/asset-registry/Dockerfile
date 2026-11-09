# Build from the repository root:
#   docker build -f services/asset-registry/Dockerfile -t asset-registry .
FROM rust:1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY services ./services
RUN cargo build --release --locked -p asset-registry --bin asset-registry

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --shell /usr/sbin/nologin app

COPY --from=builder /app/target/release/asset-registry /usr/local/bin/asset-registry

USER 10001
EXPOSE 8081
HEALTHCHECK --interval=30s --timeout=3s --start-period=15s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8081/healthz || exit 1
ENTRYPOINT ["/usr/local/bin/asset-registry"]
