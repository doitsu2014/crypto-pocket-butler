# Build stage
FROM rust:alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static curl

WORKDIR /app

# Copy manifests and lock file
COPY Cargo.toml ./

# Copy actual source code
COPY . .
RUN cargo build --release

# Runtime stage
FROM alpine

# Install runtime dependencies
RUN apk add --no-cache libgcc openssl ca-certificates

WORKDIR /app

# Copy the binaries from builder
COPY --from=builder /app/target/release/crypto-pocket-butler-backend /app/crypto-pocket-butler-backend

# Expose the application port
EXPOSE 3001

# Run the entrypoint script
ENTRYPOINT ["/app/crypto-pocket-butler-backend"]

