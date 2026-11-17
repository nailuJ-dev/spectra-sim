# Release gates

`spectra-sim` is released only after the exact source tree intended for the tag passes all gates below.

## Rust correctness

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
```

## Supply-chain and dependency policy

```bash
cargo audit --deny warnings
cargo deny check
```

`scripts/release_check.sh` runs both. The CI `supply-chain` job runs `cargo-deny-action` and `rustsec/audit-check` without deny-warnings (`deny.toml` has `unmaintained = "workspace"`), so it reports warnings but does not fail on them.

Known open item: RUSTSEC-2026-0543 (`chrono` 0.4.45 unmaintained, pulled in by `sgp4 =2.4.0`) is reported as a warning. The CI job stays green, but `cargo audit --deny warnings` (and therefore `scripts/release_check.sh`) fails. The maintainer decision (bump `sgp4`, or record a documented advisory ignore) is still open; do not tag a release through `scripts/release_check.sh` until it is made.

The release tree must not contain private registries, path dependencies outside the repository, or mandatory network services.

## Continuous-integration gates

Besides the checks above, `.github/workflows/ci.yml` runs:

| Job | Gate |
| --- | --- |
| `python-bindings` | `maturin develop --locked -m bindings/python/Cargo.toml` and `pytest bindings/python/tests` |
| `orekit-sidecar` | `mvn -q -B -f orekit-sidecar/pom.xml package` |
| `coverage` | `cargo llvm-cov --all-features --fail-under-lines 76` (line-coverage floor of the root crate; raise it when coverage grows) |
| `semver` | `cargo-semver-checks` against the baseline commit named in the job. Advisory (`continue-on-error`) until 0.5.0 is tagged; afterwards move `baseline-rev` to the tag and make it blocking |
| `fuzz-smoke` | weekly (schedule only): 30 s of each of the 11 `cargo fuzz` targets in `fuzz/` |

The root crate's `Cargo.lock` is checked with `cargo build --release --locked`; `fuzz/Cargo.lock` is committed separately because `fuzz/` is not a workspace member.

## Release workflow

`.github/workflows/release.yml` runs only when a tag `vX.Y.Z` (or `vX.Y.Z-rc.N`) is pushed. Its `verify-tag` job must pass before anything is built: the tag has to equal `v` + the root `Cargo.toml` package version and `v` + the `<version>` in `orekit-sidecar/pom.xml`. The binaries (Linux, macOS, Windows), the sidecar JAR and the GHCR image are then built from that tag, and the GitHub Release is created only after all of them succeed. The test `provenance_versions_agree` (in `tests/sdk_contracts.rs`) checks before tagging that the crate, sidecar pom, sidecar JAR name (`scripts/release_check.sh`, `orekit-sidecar/README.md`) and Python bindings versions agree.

## Physics reference checks

```bash
python3 tools/reference_check.py
python3 -m py_compile \
  adapters/gazebo/state_snapshot_to_scenario.py \
  adapters/sionna_rt/export_cir.py \
  adapters/sionna_rt/simulate_iq_from_cir.py \
  adapters/5g_lena/trace_to_isac_config.py
```

When the optional high-fidelity dependencies are available, additionally execute the Sionna RT, Gazebo Harmonic, and 5G-LENA integration examples described in `docs/HIGH_FIDELITY_BACKENDS.md`.

## Golden path

```bash
./scripts/run_golden_path.sh
```

The command must create deterministic SIGINT and C-UAS outputs and the reference physics check must pass.

## Reproducibility

Run the same scenario twice with the same seed and compare the emitted replay manifests and output hashes. They must be byte-identical. Run with a changed seed and verify that a randomized scenario changes while remaining schema-valid.

## Validation boundary

Passing these gates demonstrates software correctness against the implemented analytical models and fixtures. It does not substitute for hardware-in-the-loop or field validation. Claims about real-world accuracy require an independently held-out real dataset or instrumented field campaign.
