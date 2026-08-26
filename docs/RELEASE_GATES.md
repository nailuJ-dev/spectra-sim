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
cargo audit
cargo deny check
```

The release tree must not contain private registries, path dependencies outside the repository, or mandatory network services.

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
