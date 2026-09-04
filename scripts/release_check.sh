#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "release_check: required tool is missing: $1" >&2
    exit 1
  fi
}

require_tool cargo
require_tool python3
require_tool cargo-audit
require_tool cargo-deny
require_tool mvn
require_tool java

cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo build --release --locked
cargo audit --deny warnings
cargo deny check
mvn -q -f orekit-sidecar/pom.xml package
test -f orekit-sidecar/target/spectra-orekit-sidecar-0.4.1.jar
python3 -m py_compile adapters/gazebo/state_snapshot_to_scenario.py adapters/sionna_rt/export_cir.py adapters/sionna_rt/simulate_iq_from_cir.py adapters/5g_lena/trace_to_isac_config.py tools/reference_check.py
python3 tools/reference_check.py
python3 adapters/gazebo/state_snapshot_to_scenario.py --scenario examples/scenarios/cuas_drone_isac.json --snapshot adapters/gazebo/example_snapshot.json --out /tmp/spectra-gazebo-scenario.json
python3 adapters/5g_lena/trace_to_isac_config.py --trace adapters/5g_lena/example_nr_trace.csv --out /tmp/spectra-isac-config.json
if python3 -c "import numpy" >/dev/null 2>&1; then
  python3 adapters/sionna_rt/simulate_iq_from_cir.py --scenario examples/scenarios/sigint_urban.json --cir adapters/sionna_rt/example_cir.json --out /tmp/spectra-sionna-iq.json
else
  echo "release_check: numpy is required to validate the Sionna adapter" >&2
  exit 1
fi
