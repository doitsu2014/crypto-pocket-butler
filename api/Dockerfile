# Build stage
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./
COPY migration/Cargo.toml ./migration/

# Create dummy source files to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    mkdir -p migration/src && \
    echo "fn main() {}" > migration/src/main.rs

# Build dependencies only
RUN cargo build --release && \
    rm -rf src target/release/deps/crypto_pocket_butler_backend* && \
    rm -rf migration/src

# Copy actual source code
COPY src ./src
COPY migration ./migration

# Build the application and migration binary
RUN cargo build --release && \
    cd migration && \
    cargo build --release

# Runtime stage
FROM alpine:3.21

# Install runtime dependencies
RUN apk add --no-cache libgcc openssl ca-certificates

WORKDIR /app

# Copy the binaries from builder
COPY --from=builder /app/target/release/crypto-pocket-butler-backend /app/crypto-pocket-butler-backend
COPY --from=builder /app/target/release/migration /app/migration/migration

# Copy entrypoint script
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Expose the application port
EXPOSE 3000

# Run the entrypoint script
ENTRYPOINT ["/app/docker-entrypoint.sh"]

