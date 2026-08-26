# SDK integration

`spectra-sim` communicates with downstream consumers through JSON only.

## SIGINT

```bash
cargo run --release -- sigint examples/scenarios/sigint_urban.json --out artifacts/sigint
```

Use `artifacts/sigint/iq_capture.json` as the raw I/Q capture. It contains exactly:

- `id`
- `timestamp_ms`
- `sample_rate_hz`
- `center_frequency_hz`
- `samples[]` with `i` and `q`

`truth.json` is intentionally separate and must not be fed to the model during normal inference.

## C-UAS recorded sensing

```bash
cargo run --release -- cuas examples/scenarios/cuas_drone_isac.json --out artifacts/cuas
```

`cuas_scenario.json` contains a recorded reference measurement, candidate kinematics, optional cooperative identity and expected class label.

## C-UAS recorded ISAC

The same run emits `cuas_isac_scenario.json` with the 13-dimensional public reference feature vector and an explicit simulated authorization/configuration token.

These tokens are simulation metadata only. They do not authorize use of a real network or active transmitter.


## Direct Golden Path handoff

`spectra-sim sigint ...` writes both `iq_capture.json` and `sigint_scenario.json`. The latter is the complete public wrapper consumed by `sigint-golden-demo --scenario`, so no repository dependency or conversion script is required.
