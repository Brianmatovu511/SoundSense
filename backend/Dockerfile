# syntax=docker/dockerfile:1

############################
# Build stage
############################
FROM rustlang/rust:nightly-slim AS build
WORKDIR /app

# System deps needed by crates:
# - reqwest (tls/openssl)
# - serialport on linux may pull libudev-sys (needs libudev-dev)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    ca-certificates \
    libudev-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first (better caching)
COPY Cargo.toml Cargo.lock* ./

# Copy source and migrations
COPY src ./src
COPY tests ./tests
COPY migrations ./migrations

# Build binaries
RUN cargo build --release --bin soundsense-backend
RUN cargo build --release --bin sound-simulator

############################
# Runtime stage
############################
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Runtime deps:
# - ca-certificates: TLS
# - libssl3: reqwest/openssl runtime
# - curl OR wget: so docker-compose healthcheck can call /healthz
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 10001 appuser

COPY --from=build /app/target/release/soundsense-backend /usr/local/bin/soundsense-backend
COPY --from=build /app/target/release/sound-simulator /usr/local/bin/sound-simulator

EXPOSE 8080
USER appuser

CMD ["/usr/local/bin/soundsense-backend"]