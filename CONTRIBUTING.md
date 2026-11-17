# Contributing

## Local gate

Run all of this before opening a pull request (Rust 1.85 is pinned by `rust-toolchain.toml`):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo build --no-default-features
cargo deny check
cargo audit --deny warnings
python3 -m py_compile adapters/gazebo/state_snapshot_to_scenario.py adapters/sionna_rt/export_cir.py adapters/sionna_rt/simulate_iq_from_cir.py adapters/5g_lena/trace_to_isac_config.py tools/reference_check.py
```

`cargo install cargo-deny cargo-audit --locked` provides the two supply-chain tools.

## Fuzzing

The parsers that accept untrusted text (TLE, OMM, OEM, OCM, EOP, scenario JSON) have cargo-fuzz targets in `fuzz/` (a separate crate, not a workspace member, built on nightly):

```bash
cargo install cargo-fuzz --locked
cargo +nightly fuzz list
cargo +nightly fuzz run oem_kvn -- -max_total_time=60 -timeout=10 -rss_limit_mb=2048 -max_len=65536
```

Seed inputs live in `fuzz/corpus/<target>/`; keep them tiny and commit only hand-picked seeds, not generated corpus entries. Crashes land in `fuzz/artifacts/` (git-ignored). A panic or hang in a parser is a bug: add a regression test with the exact input to `tests/regression_audit.rs` (it must fail first), fix the parser to return an error, then re-run the target. CI runs every target for 30 s each week.

## Coverage

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo llvm-cov --all-features --summary-only
```

CI fails if line coverage of the root crate drops below the threshold in `.github/workflows/ci.yml`; raise it when coverage grows.

## Python bindings

`bindings/python` is a workspace member (PyO3 + maturin). Test it in a virtual environment:

```bash
python3 -m venv .venv-bindings
.venv-bindings/bin/pip install maturin pytest numpy
VIRTUAL_ENV=$PWD/.venv-bindings .venv-bindings/bin/maturin develop -m bindings/python/Cargo.toml
.venv-bindings/bin/pytest bindings/python/tests
```

## JSON schemas

`schemas/` is generated from the Rust types. After changing a serialized type, regenerate and commit; `tests/schema_contracts.rs` fails when the files are stale:

```bash
cargo run -q -- schema scenario > schemas/scenario.schema.json
cargo run -q -- schema sigint-run > schemas/sigint_run.schema.json
cargo run -q -- schema cuas-run > schemas/cuas_run.schema.json
```

## Releasing

1. Bump the version everywhere, in lock-step: root `Cargo.toml`, `bindings/python/Cargo.toml`, `orekit-sidecar/pom.xml`, the JAR name in `scripts/release_check.sh`, and `Cargo.lock` (a plain `cargo build` refreshes it; commit the result).
2. Update `CHANGELOG.md`.
3. Run `scripts/release_check.sh` (needs cargo-audit, cargo-deny, Maven and Java).
4. Tag `vX.Y.Z` (pre-releases: `vX.Y.Z-rc.N`) and push the tag. `release.yml` refuses to publish unless the tag equals the `Cargo.toml` version and the `pom.xml` version, then builds the binaries, JAR and GHCR image.
5. Once a release is tagged, move the `baseline-rev` of the `semver` CI job to that tag.

## Physics and tests

Physics changes must include a closed-form regression test or a clearly documented validation reference. Keep stochastic tests seeded and reproducible.
