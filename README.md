# spectra-sim

**Physics-based RF sensing simulation for terrestrial, maritime, underwater and space environments.**

`spectra-sim` is an open-source simulation framework for generating realistic RF, radar and sensing scenarios.

The project is designed for engineers and researchers working on wireless systems, RF machine learning, radar, SIGINT, 5G/ISAC, satellite communications and electromagnetic sensing.

Its goal is simple: provide reproducible RF environments that are realistic enough to develop, benchmark and stress-test sensing algorithms before field deployment.

## Quickstart

### Install

* **Release binaries and Docker image.** Pushing a version tag runs the release workflow (`.github/workflows/release.yml`), which is set up to attach `spectra-sim` binaries for Linux, macOS and Windows (with `.sha256` files) and the Orekit sidecar JAR to the GitHub Release, and to publish a Docker image `ghcr.io/nailuj-dev/spectra-sim:<tag>` (run it with `docker run --rm ghcr.io/nailuj-dev/spectra-sim:<tag> examples list`). These only exist for tags that have been pushed; until a release is published, use one of the options below.
* **From source.** `cargo build --release --locked` produces `target/release/spectra-sim`; `cargo run --release -- <command>` runs it directly. The commands below assume `spectra-sim` is on your `PATH`; otherwise substitute `cargo run --release --`.
* **Python.** There is no `pip install spectra-sim` yet. Build the bindings from source with maturin, see [`bindings/python/README.md`](bindings/python/README.md).

### First run

```bash
spectra-sim examples list                              # embedded examples (`randomization` is a config for `randomize`, not a scenario)
spectra-sim examples show sigint_urban > sigint_urban.json
spectra-sim validate sigint_urban.json                 # check the file without simulating (--json for machines)

spectra-sim sigint sigint_urban.json --out out --also npy,sigmf
spectra-sim plot out/iq_capture.json --out out/plots   # constellation, spectrum, spectrogram SVGs
spectra-sim replay sigint_urban.json out/replay_manifest.json --kind sigint   # prints MATCH

# Monte Carlo: one run per seed 0..15 (half-open range, at most 100000 seeds)
spectra-sim sweep sigint_urban.json --kind sigint --seeds 0..16 --out out/sweep
```

`examples list` also shows `randomization`: that is a randomization config (use it with `randomize` or `sweep --randomization`), not a scenario, so `validate` rejects it. `sigint` writes `iq_capture.json`, `truth.json`, `sigint_scenario.json`, `replay_manifest.json` and `validity_report.md` into `--out`; `sweep` writes one `seed_<n>/` directory per seed plus a `sweep_manifest.json` index of per-seed output hashes. `replay` exits 2 and names the reason when the scenario, simulator version, output schema or output hash differs from the manifest. Exit codes: 0 success, 2 invalid input, 3 I/O error. See [`docs/SDK_INTEGRATION.md`](docs/SDK_INTEGRATION.md) for the file contracts and JSON Schemas.

### Formats

| Format | Files | Content |
| --- | --- | --- |
| JSON (always) | `iq_capture.json` | `samples[]` of `{i, q}` (`f32`) plus `sample_rate_hz`, `center_frequency_hz` |
| NumPy (`--also npy`) | `iq_capture.npy` | `complex64` (`<c8`), shape `(N,)` |
| SigMF (`--also sigmf`) | `iq_capture.sigmf-data`, `iq_capture.sigmf-meta` | interleaved little-endian `f32` I/Q (`cf32_le`); metadata with sample rate and centre frequency |

```python
import numpy as np
iq = np.load("out/iq_capture.npy")            # complex64 array
```

Or run the simulator from Python (after building the bindings, see above):

```python
import spectra_sim

scenario = open("examples/scenarios/sigint_urban.json").read()
result = spectra_sim.run_sigint(scenario)     # result.iq is a numpy complex64 array
print(result.iq.shape, result.sample_rate_hz)
print(spectra_sim.verify_replay(scenario, result.manifest_json, "sigint"))   # "match"
```

### Models and validation status

Each `sigint` / `cuas` run also writes `validity_report.md`. Its scope is the propagation model only, and it always states that the data is synthetic and not field-validated (see [`docs/VALIDATION.md`](docs/VALIDATION.md) for the validation ladder).

| Model | Status in the validity report | Basis |
| --- | --- | --- |
| `free_space` propagation | validated reference | closed-form Friis equation |
| `two_ray` propagation | validated with warnings | idealised, not calibrated against measurements |
| `rician` propagation | validated with warnings | idealised statistical model, not calibrated against measurements |
| radar, ISAC, antenna-pattern and receiver models | not yet catalogued in the validity report | no entry in the report |

## What's new in 0.5.0

Command-line `validate`, `examples`, `replay`, `sweep`, `plot` and `schema`; `.npy` / SigMF export; SVG plots; a validity report; published JSON Schemas; a native Doppler-rate observable; Python bindings; a tag-driven release workflow; and fuzzing, coverage and semver CI gates. Output hashes are unchanged from 0.4.1. Breaking and behavioural changes (I/O errors now exit 3, `#[non_exhaustive]` on `RunKind` / `ReplayVerdict`, a 32 MiB input cap, calendar years limited to -4799..=9999) are listed in [`CHANGELOG.md`](CHANGELOG.md).

### Known limitations

* The validity report covers the selected propagation model and a synthetic-data notice only; radar, ISAC, antenna and receiver models are not catalogued yet.
* Release binaries are built on GitHub's `ubuntu-latest`, `macos-latest` (Apple silicon, so no Intel macOS binary) and `windows-latest` runners; the Linux binary links the runner's glibc.
* The `sigint-run` / `cuas-run` JSON Schemas describe in-memory run results, not a single file written by the CLI.
* The Python package is not published to PyPI; build it from source (see above).

## What it does

`spectra-sim` models the physical environment between emitters, propagation media, sensors and receivers.

Core capabilities include:

* RF signal and I/Q scenario generation
* radar and RF sensing simulation
* 5G / ISAC scenarios
* multipath and propagation effects
* deterministic simulation and replay
* synthetic truth generation
* configurable receiver and channel impairments
* scenario randomization and Monte Carlo workflows
* structured JSON export for external ML and sensing pipelines

## Multi-medium RF physics

### Terrestrial

Model wireless and radar propagation through configurable environments with path loss, multipath, channel effects and receiver impairments. The same explicit `free_space`, `two_ray` or seeded `rician` propagation model is consumed by the SIGINT, radar and ISAC paths. Directional antenna patterns can be attached to emitters, receivers, radar and ISAC configurations; their gain is evaluated from the platform `roll_deg` / `pitch_deg` / `yaw_deg` attitude and the instantaneous line of sight.

### Underwater and maritime

The v0.2 physics layer introduces RF propagation through conductive and dielectric media.

Supported concepts include:

* freshwater and seawater
* temperature and salinity dependent electrical properties
* complex permittivity
* conductivity
* attenuation and phase constants
* skin depth
* phase velocity
* lossy-medium propagation
* stratified water columns
* air/water and water/seabed interfaces
* complex Fresnel TE/TM reflection and transmission
* seabed and material layers

This makes it possible to model scenarios such as:

```text
air
──────────── sea surface
seawater
──────────── thermocline
deep water
──────────── seabed
```

without reducing underwater propagation to a simple free-space path-loss correction.

### Space and NTN simulation

`spectra-sim` is designed to consume professional orbital state information rather than relying on simplified satellite trajectories.

The space RF layer supports the architecture required for:

* TLE / OMM propagation through SGP4-compatible backends
* CCSDS ephemerides
* authoritative position and velocity states
* orbital reference frames
* precise link geometry
* iterative light-time computation
* slant range
* line-of-sight range rate
* one-way Doppler shift (its rate is available via `doppler_rate_hz_per_s`, a central difference)
* Earth-space RF links
* antenna pointing and geometry

The Rust core remains independent from Java, while the optional Orekit sidecar acts as a precision/reference oracle. It can propagate TLEs, transform states, and return full Cartesian state histories parsed from OEM and OCM so native results can be compared against an independent implementation.

`spectra-sim` 0.5.0 provides a deterministic space-link stack for research and engineering workflows:

* TLE/OMM + SGP4/SDP4 propagation;
* CCSDS OEM/OCM KVN/XML ingestion;
* IERS EOP-backed TEME/GCRF/ITRF transformations;
* absolute UTC/TAI/TT/UT1/GPS time handling;
* ITU-R P.676-13 gaseous and P.840-9 cloud attenuation;
* explicit rain/scintillation inputs for P.618/P.838 workflows;
* first-order ionospheric delay, phase advance and Faraday rotation;
* WGS-84 ground geometry, antenna pointing and link budgets;
* optional Orekit 13.1.8 precision/reference backend using a pinned local `orekit-data` snapshot, including detailed OEM/OCM Cartesian oracle output.

The library does not silently download EOP, climatology or Orekit data, and does not substitute missing precision inputs with invented defaults.

## Scientific validity

A simulator should not silently produce numbers outside the validity range of its physical models.

`spectra-sim` therefore introduces an explicit physics validity layer.

Models can expose:

```text
VALIDATED
VALIDATED_WITH_WARNINGS
EXTRAPOLATED
EXPERIMENTAL
INVALID
```

along with information about:

* physical model used
* reference
* validity range
* missing environmental data
* extrapolation
* model maturity

The long-term objective is to make simulation provenance as important as the simulated data itself.

## Example applications

`spectra-sim` can be used to develop and evaluate:

* RF signal classifiers
* specific emitter identification
* spectrum monitoring
* interference detection
* radar perception
* C-UAS sensing
* maritime sensing
* satellite communication algorithms
* NTN signal processing
* RF anomaly detection
* synthetic datasets for RF ML
* sim-to-real evaluation pipelines

## Architecture

```text
Scenario
   ↓
World / geometry
   ↓
Propagation media
   ↓
Interfaces and multipath
   ↓
Antenna
   ↓
Receiver
   ↓
I/Q + sensor observations
   ↓
Truth + metadata + validity
```

The simulator is deliberately modular so higher-fidelity propagation or astrodynamics backends can be integrated without changing downstream sensing APIs.

## Build from source

```bash
git clone https://github.com/nailuJ-dev/spectra-sim.git
cd spectra-sim

cargo build --release --locked
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

Explore the CLI:

```bash
cargo run --bin spectra-sim -- --help
```

Example scenarios are available in the repository and embedded in the binary (`spectra-sim examples list`).

## Design principles

`spectra-sim` prioritizes:

* physics over visual plausibility
* reproducibility over hidden randomness
* explicit uncertainty over false precision
* composable models over monolithic simulation
* documented validity over silent approximation
* interoperability over vendor lock-in

It is not intended to pretend that analytical models replace full-wave solvers or field measurements.

Where higher fidelity is required, the architecture is designed to connect to specialized simulation backends.

## Open-source boundary

This repository focuses on physical simulation and reproducible sensing data.

It deliberately does not contain proprietary persistent electromagnetic world models, global emitter identity systems, customer-specific calibration corpora or private learned propagation models.

## Contributing

Contributions are welcome, especially around:

* RF propagation models
* underwater electromagnetics
* atmospheric and ionospheric propagation
* orbital mechanics integration
* radar and ISAC
* reference datasets
* validation against measurements
* numerical benchmarks

If you find a physical assumption that can be improved, open an issue with a reference, measurement or reproducible test case.

## Support the project

If `spectra-sim` is useful to your work:

* star the repository
* use it in experiments and prototypes
* report unrealistic behavior
* contribute reference scenarios
* cite or link the project when publishing results
* share it with RF, radar and wireless engineers

Adoption and external validation are the most valuable forms of support at this stage.

## About

`spectra-sim` is developed as part of the open-source RF and electromagnetic sensing work initiated by **Anderion Systems**.

The broader objective is to improve the software infrastructure available for machines that need to observe, model and understand the electromagnetic environment.

**Anderion Systems** — https://anderion-systems.com

## License

See the repository `LICENSE` file.
