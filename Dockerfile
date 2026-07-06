FROM rust:1.81-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libfontconfig1-dev \
    libxcb-shape0-dev libxcb-xfixes0-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /tuba
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 libfontconfig1 \
    chromium chromium-common \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

COPY --from=builder /tuba/target/release/tuba /usr/local/bin/tuba

ENV TUBA_CHROME_PATH=/usr/bin/chromium

ENTRYPOINT ["tuba"]
CMD []
