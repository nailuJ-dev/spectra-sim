# spectra-sim

`spectra-sim` is a standalone, deterministic physics-based simulator for RF/SIGINT, monostatic radar and 5G/NR-style integrated sensing and communication (ISAC) experiments.

It is intentionally independent from any downstream ML SDK. Its integration surface is versioned JSON only.

## What is implemented

The built-in Rust engine provides:

- free-space path loss (Friis);
- two-ray ground reflection;
- deterministic Rician fading and log-normal shadowing;
- thermal noise using `kTB` plus receiver noise figure;
- one-way and monostatic Doppler;
- transmitter / receiver antenna gains and losses;
- oscillator frequency error, phase noise, I/Q imbalance, AGC and ADC quantization;
- CW, pulse-train and compact OFDM waveform generation;
- monostatic radar equation;
- range / radial velocity / azimuth / elevation ground truth;
- rotor micro-Doppler approximation;
- radar range, velocity and array-angle resolution estimates;
- 5G/OFDM MIMO-ISAC sensing observables;
- deterministic domain randomization;
- replay manifests with SHA-256 digests;
- exporters matching the public SIGINT I/Q and C-UAS Golden Path JSON contracts.

## Quick start

```bash
cargo run --release -- sigint examples/scenarios/sigint_urban.json --out artifacts/sigint
cargo run --release -- cuas examples/scenarios/cuas_drone_isac.json --out artifacts/cuas
```

Or:

```bash
./scripts/run_golden_path.sh
```

The SIGINT run writes:

```text
artifacts/sigint/
├── iq_capture.json
├── truth.json
└── replay_manifest.json
```

The C-UAS run writes:

```text
artifacts/cuas/
├── radar_truth.json
├── cuas_scenario.json
├── isac_truth.json
├── cuas_isac_scenario.json
└── replay_manifest.json
```

`iq_capture.json` can be passed to a compatible raw-I/Q Golden Path. `cuas_scenario.json` and `cuas_isac_scenario.json` match the public recorded-sensing / recorded-ISAC Golden Path contracts.

## Fidelity ladder

| Level | Backend | Purpose |
| --- | --- | --- |
| L0 | Rust analytical engine | CI, regression, deterministic replay |
| L1 | Rust stochastic + domain randomization | dataset generation and robustness training |
| L2 | Gazebo + Sionna RT + optional NR trace bridge | pre-hardware digital-twin studies |

The optional adapters are deliberately out-of-process. The core simulator never requires Gazebo, Sionna RT, ns-3 / 5G-LENA, a GPU, or a network service.

## Important limitation

Physics-based simulation is not field validation. Material properties, antenna patterns, clutter, RF front-end behavior, target RCS and micro-Doppler models must ultimately be calibrated against measurements. The included scenarios are reference fixtures, not operational accuracy evidence.

See `docs/PHYSICS.md`, `docs/HIGH_FIDELITY_BACKENDS.md`, `docs/SDK_INTEGRATION.md` and `docs/VALIDATION.md`.

## Compatibility

| spectra-sim | SIGINT SDK | C-UAS SDK |
|---|---|---|
| 0.1.1 | 0.4.1 | 0.5.1 |

Integration uses only public JSON files.
