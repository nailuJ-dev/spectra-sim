# Sionna RT bridge

This adapter exports a site-specific CIR with deterministic `PathSolver(seed=...)` settings.

```bash
python -m venv .venv
. .venv/bin/activate
pip install -r adapters/sionna_rt/requirements.txt
python adapters/sionna_rt/export_cir.py \
  --scene path/to/scene.xml \
  --frequency-hz 3500000000 \
  --tx 0,0,10 --rx 180,130,85 \
  --max-depth 4 --seed 41 \
  --out artifacts/sionna/cir.json
```

The CIR artifact is intentionally kept separate from the L0/L1 analytical core. Use it for cross-validation and site-specific calibration before replacing analytical propagation with ray-traced paths in a downstream experiment.

## CIR to raw I/Q

After exporting the CIR, convert it to a downstream-compatible raw I/Q capture:

```bash
python adapters/sionna_rt/simulate_iq_from_cir.py \
  --scenario examples/scenarios/sigint_urban.json \
  --cir artifacts/sionna/cir.json \
  --out artifacts/sionna/iq_capture.json
```

This path preserves the scenario seed, applies the ray-traced multipath coefficients and delays, then applies the reference receiver AGC/noise/oscillator/IQ/ADC model.
