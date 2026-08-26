FROM rust:1.85-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked || cargo build --release

FROM debian:bookworm-slim
RUN useradd --create-home --uid 10001 spectra
COPY --from=build /src/target/release/spectra-sim /usr/local/bin/spectra-sim
USER spectra
ENTRYPOINT ["/usr/local/bin/spectra-sim"]
