#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
COUNT="${1:-32}"
OUT="${2:-artifacts/domain-dataset}"
mkdir -p "$OUT/scenarios" "$OUT/outputs"
for ((i=0;i<COUNT;i++)); do
  seed=$((100000+i))
  scenario="$OUT/scenarios/cuas-${seed}.json"
  cargo run --release -- randomize examples/scenarios/cuas_drone_isac.json examples/scenarios/randomization.json --seed "$seed" --out "$scenario"
  cargo run --release -- cuas "$scenario" --out "$OUT/outputs/${seed}"
done
printf 'Generated %s deterministic physical variants in %s\n' "$COUNT" "$OUT"
