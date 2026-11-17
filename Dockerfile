# Reproducible build: compiler and dependency graph are both pinned.
FROM rust:1.85.0-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked

FROM debian:bookworm-slim
# Static OCI labels; version/revision/created are added at build time by
# docker/metadata-action in the release workflow.
LABEL org.opencontainers.image.source="https://github.com/nailuJ-dev/spectra-sim" \
      org.opencontainers.image.licenses="Apache-2.0 OR MIT"
RUN useradd --create-home --uid 10001 spectra
COPY --from=build /src/target/release/spectra-sim /usr/local/bin/spectra-sim
USER spectra
ENTRYPOINT ["/usr/local/bin/spectra-sim"]
