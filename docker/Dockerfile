# ── Stage 1: Build ────────────────────────────────
FROM rust:1.77-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/filehub-core/Cargo.toml crates/filehub-core/
COPY crates/filehub-entity/Cargo.toml crates/filehub-entity/
COPY crates/filehub-database/Cargo.toml crates/filehub-database/
COPY crates/filehub-cache/Cargo.toml crates/filehub-cache/
COPY crates/filehub-storage/Cargo.toml crates/filehub-storage/
COPY crates/filehub-auth/Cargo.toml crates/filehub-auth/
COPY crates/filehub-service/Cargo.toml crates/filehub-service/
COPY crates/filehub-realtime/Cargo.toml crates/filehub-realtime/
COPY crates/filehub-worker/Cargo.toml crates/filehub-worker/
COPY crates/filehub-plugin/Cargo.toml crates/filehub-plugin/
COPY crates/filehub-plugin-sdk/Cargo.toml crates/filehub-plugin-sdk/
COPY crates/filehub-api/Cargo.toml crates/filehub-api/
COPY crates/plugin-flexnet/Cargo.toml crates/plugin-flexnet/
COPY crates/plugin-cad-converter/Cargo.toml crates/plugin-cad-converter/
COPY crates/filehub-cli/Cargo.toml crates/filehub-cli/

# Create dummy source files for dependency compilation caching
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN for dir in crates/*/; do \
        mkdir -p "$dir/src" && \
        echo "" > "$dir/src/lib.rs"; \
    done
RUN mkdir -p crates/filehub-cli/src && echo "fn main() {}" > crates/filehub-cli/src/main.rs

# Build dependencies only (cached layer)
RUN cargo build --release 2>/dev/null || true

# Remove dummy files
RUN find . -name "*.rs" -path "*/src/*" -delete
RUN rm -rf target/release/.fingerprint/filehub-* \
    target/release/.fingerprint/plugin-* \
    target/release/deps/filehub_* \
    target/release/deps/plugin_* \
    target/release/deps/libfilehub_* \
    target/release/deps/libplugin_*

# Copy actual source code
COPY src/ src/
COPY crates/ crates/
COPY migrations/ migrations/
COPY config/ config/

# Build the application
RUN cargo build --release --bin suzuki-filehub
RUN cargo build --release --bin filehub-cli

# ── Stage 2: Runtime ──────────────────────────────
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r filehub && useradd -r -g filehub -m -d /home/filehub filehub

# Create application directories
RUN mkdir -p /opt/filehub/config \
    /opt/filehub/data/storage/local \
    /opt/filehub/data/cache/thumbnails \
    /opt/filehub/data/cache/conversions \
    /opt/filehub/data/temp \
    /opt/filehub/data/logs \
    /opt/filehub/data/plugins \
    /opt/filehub/data/backups \
    /opt/filehub/migrations \
    && chown -R filehub:filehub /opt/filehub

WORKDIR /opt/filehub

# Copy binaries
COPY --from=builder /app/target/release/suzuki-filehub /usr/local/bin/filehub
COPY --from=builder /app/target/release/filehub-cli /usr/local/bin/filehub-cli

# Copy configuration and migrations
COPY --from=builder /app/config/ /opt/filehub/config/
COPY --from=builder /app/migrations/ /opt/filehub/migrations/

# Set permissions
RUN chmod +x /usr/local/bin/filehub /usr/local/bin/filehub-cli

# Switch to non-root user
USER filehub

# Environment
ENV FILEHUB_CONFIG=/opt/filehub/config/default.toml
ENV FILEHUB_ENV=production
ENV RUST_LOG=info

# Expose ports
EXPOSE 8080 8081

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Use tini as init system
ENTRYPOINT ["tini", "--"]

# Default command
CMD ["filehub"]