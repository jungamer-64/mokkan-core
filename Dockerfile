FROM rust:slim-bookworm AS base

WORKDIR /app

FROM base AS dev

ENV LISTEN_ADDR=0.0.0.0:8080

COPY . .

EXPOSE 8080

CMD ["cargo", "run", "--locked", "--bin", "mokkan_core"]

FROM base AS builder

COPY . .

RUN cargo build --release --locked --bin mokkan_core

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && addgroup --system app \
    && adduser --system --ingroup app --home /app app \
    && mkdir -p /app/spec \
    && chown -R app:app /app

WORKDIR /app

ENV LISTEN_ADDR=0.0.0.0:8080

COPY --from=builder /app/target/release/mokkan_core /usr/local/bin/mokkan_core

EXPOSE 8080

USER app

CMD ["/usr/local/bin/mokkan_core"]
