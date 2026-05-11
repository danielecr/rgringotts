# ── Stage 1: build ────────────────────────────────────────────────────────────
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libgringotts-dev libmcrypt-dev libmhash-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN cargo build --release

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        libgringotts2 libmcrypt4 libmhash2 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/rgringotts /usr/local/bin/rgringotts

EXPOSE 7979
CMD ["rgringotts"]
