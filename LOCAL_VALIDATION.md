# Local validation before push

This remediation package was statically validated in the generation environment, but that environment does not contain a Rust toolchain or Maven. Run the following locally before pushing:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo build --release --locked
```

Supply-chain gates, if installed:

```bash
cargo deny check
cargo audit --deny warnings
```

Orekit sidecar:

```bash
cd orekit-sidecar
mvn -q package
cd ..
```

Runtime requires a local Orekit data snapshot:

```bash
export OREKIT_DATA_DIR=/absolute/path/to/orekit-data
```

Then optionally:

```bash
./scripts/release_check.sh
./scripts/run_golden_path.sh
```

The 0.4.1 remediation changes deterministic golden outputs for paths affected by corrected Fresnel, AGC/IQ, directional antenna, and radar propagation physics. Review and regenerate committed golden artifacts only after all tests are green.
