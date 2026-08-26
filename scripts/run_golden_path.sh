#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
rm -rf artifacts/golden-path
mkdir -p artifacts/golden-path
cargo run --release -- sigint examples/scenarios/sigint_urban.json --out artifacts/golden-path/sigint
cargo run --release -- cuas examples/scenarios/cuas_drone_isac.json --out artifacts/golden-path/cuas
cargo run --release -- randomize examples/scenarios/cuas_drone_isac.json examples/scenarios/randomization.json --seed 9001 --out artifacts/golden-path/randomized_cuas.json
python3 tools/reference_check.py
printf 'Golden Path outputs: %s\n' "$ROOT/artifacts/golden-path"
