## Unreleased

### Added
- `spectra-sim space-pass <config.json>`: SGP4 + EOP (TEME→ITRF) ground-station pass prediction with azimuth, elevation, range, range rate, one-way Doppler and Doppler rate, visibility windows, `pass.json`, `elevation.svg` and `doppler.svg`. Library API: `PassConfig`, `compute_pass`, `parse_eop`, `visibility_windows`, `pass_sample_count` (at most 100000 samples). Instantaneous geometry (no light-time correction). Example: `examples/space/vanguard1_pass.json` with illustrative (non-IERS) EOP values.

### Changed
- The validity report now catalogues every model a scenario's jobs exercise: waveform synthesis, kTB noise, receiver impairments, antenna patterns, monostatic radar equation, constant RCS, array angle resolution, micro-Doppler index (experimental, so C-UAS runs with rotor targets report `EXPERIMENTAL`) and OFDM ISAC relations. The `scope` warning text changed accordingly.

## 0.5.0

### Added

#### Command-line interface
- The CLI is now built on `clap`: `spectra-sim --help`, `--version` (`spectra-sim 0.5.0`) and `<subcommand> --help` work everywhere.
- `validate <scenario> [--json]` checks a scenario file without simulating it and prints which jobs (`sigint`, `cuas`) it can run.
- `examples list` and `examples show <name>` print the example scenarios embedded in the binary, so a release binary is usable without a repository checkout.
- `replay <scenario> <manifest> --kind sigint|cuas` re-runs a scenario and verifies it against a stored `replay_manifest.json`. It prints `MATCH` or a one-line reason and exits 2 on any mismatch. The library equivalent is `verify_replay`, returning a `ReplayVerdict` (`Match`, `ScenarioMismatch`, `VersionMismatch`, `SchemaMismatch`, `OutputMismatch`).
- `sweep <scenario> --kind sigint|cuas --seeds A..B [--randomization <file>] [--jobs N] [--out <dir>] [--force]` runs a Monte Carlo batch with one `seed_<n>/` directory per seed and a `sweep_manifest.json` index of per-seed output hashes. The seed range is half-open and limited to 100000 seeds; `--jobs` must be 1 to 1024 (default: available parallelism).
- `plot <iq_capture.json> [--out <dir>] [--fft <N>]` writes dependency-free SVG plots: `constellation.svg`, `spectrum.svg` and `spectrogram.svg`. Output is byte-identical across runs.
- `schema scenario|sigint-run|cuas-run` prints a JSON Schema (see below).
- `sigint --also npy,sigmf` additionally writes the I/Q capture as a NumPy `.npy` file and as a SigMF dataset (see below).
- `sigint` and `cuas` now also write `validity_report.md`, a human-readable Markdown rendering of the physics validity ledger.

#### Exit codes
- 0 success; 2 invalid input or usage (including a failed `replay`); 3 I/O error (file missing, unreadable or unwritable).

#### Bounded input reads
- Every JSON input the CLI reads (scenarios, randomization configs, replay manifests, I/Q captures for `plot`) is limited to 32 MiB; larger files are rejected with exit 2 instead of being read into memory. The Python bindings apply the same cap to scenario strings.

#### JSON Schemas
- `schemas/scenario.schema.json`, `schemas/sigint_run.schema.json` and `schemas/cuas_run.schema.json` (JSON Schema, generated from the Rust types with `schemars`) are committed and kept in sync by a test. A new `schema` Cargo feature, enabled by default, provides `spectra_sim::{scenario_schema, sigint_run_schema, cuas_run_schema}` and the `schema` subcommand; `cargo build --no-default-features` still compiles.
- The `sigint-run` and `cuas-run` schemas describe the in-memory `SigintRun` / `CuasRun` aggregates (simulation result plus manifest) returned by `run_sigint` / `run_cuas`. They are not a file the CLI writes; the CLI writes the separate files listed in `docs/SDK_INTEGRATION.md`.

#### ML-friendly I/Q formats
- `iq_capture.npy`: NumPy format 1.0, dtype `<c8` (`complex64`), shape `(N,)`.
- `iq_capture.sigmf-data` / `iq_capture.sigmf-meta`: SigMF `cf32_le` dataset with metadata (`core:version`, `core:datatype`, `core:sample_rate`, `core:dataset`, and a capture segment carrying `core:frequency` and `core:sample_start`). The checksum field is omitted.
- Library: `spectra_sim::formats::{iq_to_npy_bytes, sigmf_data_bytes, sigmf_meta_json}`.

#### SVG plots
- Library: `spectra_sim::viz::{constellation_svg, spectrum_svg, spectrogram_svg, timeline_svg}`.
- The dB axis of the spectrum and spectrogram is the Hann-windowed periodogram power (normalised by the window power) of the stored I/Q values, in dB relative to a squared sample value of 1. It is not dBFS (it is not referenced to the ADC full scale), so read relative levels, not absolute ones.
- The CLI bounds the work: the constellation draws at most 2000 points, the FFT length is a power of two up to 2^20 (default 256), and the spectrogram is limited to 4096 cells (frames are skipped and bins merged by the smallest integer factors that fit).
- A `hop` larger than the FFT length can produce a trailing frame that starts past the end of the capture, reported as a -200 dB row; the CLI (`hop = fft / 2`) never hits this.

#### Validity report
- `spectra_sim::report::{validity_report_markdown, scenario_validity_ledger}` build the report written as `validity_report.md`. Its scope is the propagation model only: `free_space` is labelled a validated reference (closed-form Friis), `two_ray` and `rician` are validated with warnings (idealised, not calibrated against measurements). Every report carries a "synthetic data: not field-validated" warning and an explicit warning that radar, ISAC, antenna-pattern and receiver models are not catalogued. The maturity labels are judgment calls documented in `src/report.rs`.

#### Space
- `doppler_rate_hz_per_s` computes the Doppler rate (time derivative of the one-way Doppler shift, Hz/s) by a central difference of the light-time-corrected link solution.

#### Python bindings
- New workspace member `bindings/python` (PyO3 0.29, `abi3-py39`, built with maturin): the `spectra_sim` module with `validate`, `run_sigint` (returns a `SigintResult` with a numpy `complex64` array `iq`, `sample_rate_hz`, `center_frequency_hz`, `truth_json`, `manifest_json`), `run_cuas`, `verify_replay` and `__version__`. Results are bit-identical to the CLI (the replay manifest hashes match). Build from source with maturin (see `bindings/python/README.md`); publishing to PyPI is not part of this release.

#### Release workflow, fuzzing and CI gates
- `.github/workflows/release.yml`: pushing a tag `vX.Y.Z` (or `vX.Y.Z-rc.N`) builds CLI binaries for Linux, macOS and Windows, the Orekit sidecar JAR and a GHCR Docker image, then publishes a GitHub Release with `.sha256` files. A `verify-tag` job refuses to continue unless the tag equals the root `Cargo.toml` version and the sidecar `pom.xml` version.
- `fuzz/`: `cargo-fuzz` harnesses for 11 parsers (`tle`, `omm_json`, `omm_kvn`, `omm_xml`, `oem_kvn`, `oem_xml`, `ocm_kvn`, `ocm_xml`, `eop_finals`, `eop_c04`, `scenario_json`) with seed corpora, excluded from the workspace and run weekly in CI as a smoke test.
- CI: line-coverage floor of 76% for the root crate (`cargo llvm-cov`), an advisory `cargo-semver-checks` job (compared against the 0.4.1 baseline commit), the Python bindings job (`maturin develop` + `pytest`), and a `supply-chain` job (`cargo deny` + `cargo audit`).
- Dependabot now also covers the Maven sidecar, the pip adapters, `fuzz/` and `bindings/python`.
- `LICENSE-APACHE` and `LICENSE-MIT` texts (the crate is `Apache-2.0 OR MIT`); `CONTRIBUTING.md` documents the development gates and the release checklist.
- A provenance test (`provenance_versions_agree`) checks that the crate, the Orekit sidecar `pom.xml`, the sidecar JAR name in `scripts/release_check.sh` and `orekit-sidecar/README.md`, and the Python bindings crate carry the same version.

### Changed
- **API change:** `ReplayManifest` has a new field `output_schema_version` (current value `"spectra-sim-output-v1"`, also exported as `OUTPUT_SCHEMA_VERSION`). Manifests written by 0.4.1 and earlier lack the field and deserialize with the value `"unversioned"`. Code constructing `ReplayManifest` with a struct literal must add the field.
- **API change:** `RunKind` and `ReplayVerdict` are `#[non_exhaustive]`; `match` on them needs a wildcard arm.
- **Behaviour change:** I/O errors (missing or unreadable input, unwritable output) now exit with code 3. Previously every failure exited with code 2. Invalid input and usage errors still exit with 2.
- **Behaviour narrowing:** `AbsoluteEpoch::from_calendar` now rejects years outside -4799..=9999.
- Replay verdict semantics: a differing scenario hash is a `ScenarioMismatch`; otherwise the scenario is re-run and an equal output hash is a `Match` (whatever the manifest's schema tag). A differing output is reported as `VersionMismatch` if the manifest was written by another simulator version, else `SchemaMismatch` if its output schema differs from the current one, else `OutputMismatch`.
- `sweep` details: per-seed directories contain the run files (for `sigint`: `iq_capture.json`, `truth.json`, `sigint_scenario.json`, `replay_manifest.json`) but not the effective per-seed scenario; use `randomize --seed N` to reproduce it. `--force` allows writing into a non-empty output directory, keeps any existing `seed_*` directories and removes a stale `sweep_manifest.json`; `sweep_manifest.json` is the authoritative index.

### Fixed
- TLE lines whose checksum column is not a digit are now rejected with an error. They previously caused an arithmetic overflow panic in debug builds (found by fuzzing).
- `sweep` rejects `--jobs` above 1024 instead of failing while spawning threads; all JSON inputs are read through a bounded reader (see above).

### Notes
- Output hashes are unchanged from 0.4.1: the same scenario and seed produce the same `output_sha256`. The `output_hash_is_unchanged` test in `tests/sdk_contracts.rs` pins this. The manifest's `simulator_version` changes to `0.5.0`, but it is not part of `output_sha256`.
- Supply-chain status: RUSTSEC-2026-0543 (`chrono` 0.4.45 flagged *unmaintained*, an informational advisory and not a vulnerability) is a mandatory dependency of `sgp4 =2.4.0`, already the latest `sgp4` release, so there is no upgrade path and no feature flag removes it. It is explicitly ignored (maintainer-approved), with the rationale in `.cargo/audit.toml` and `deny.toml`, which must stay in sync. `cargo audit --deny warnings` and `scripts/release_check.sh` pass on this point; any other advisory still fails. Revisit when `sgp4` drops or replaces `chrono`.
- The Docker image and release binaries are produced by `release.yml` only once a tag is pushed; none are published by this change.

## 0.4.1

### Breaking / dataset integrity
- `IqCaptureOut` now includes `full_scale_v` and `adc_bits`; `SigintTruth` now includes effective directional gains and `clipped_sample_fraction`.
- Raw I/Q samples change because AGC now targets measured waveform RMS and the IQ phase-imbalance model includes real I/Q cross-coupling.
- Radar/ISAC results can change when a non-free-space propagation model or directional antenna pattern is configured, because those settings are now actually consumed.
- Datasets generated with v0.4.0 pulsed-waveform AGC or oblique-incidence Fresnel physics should be regenerated rather than numerically relabelled.

### Fixed
- Corrected TE/TM Fresnel reflection equations and TM electric-field transmission at oblique incidence.
- Corrected AGC for duty-cycled/non-unit-power waveforms and receiver IQ quadrature skew.
- Preserved bracketed OCM `TRAJ_UNITS` lists.
- Accepted and preserved optional OEM acceleration triplets; malformed numeric state records are rejected explicitly.
- Skipped incomplete `finals2000A` prediction-tail rows without inventing zero LOD/dX/dY values.
- Enforced the passive-medium complex-permittivity sign convention.
- Cached embedded P.676-13 spectroscopy tables and enforced the model's 1–1000 GHz domain.
- Replaced flat local ENU-to-geodetic export with WGS-84 ENU→ECEF→geodetic conversion.
- Rejected duplicate emitter/receiver/target IDs and malformed CLI `--seed` values.

### Integrated
- Unified the two previously incompatible public `Complex64` implementations into one type while preserving the `MaterialComplex64` compatibility alias.
- Added serializable isotropic/Gaussian/elliptical/tabulated antenna patterns and connected platform attitude to terrestrial SIGINT/radar/ISAC gain.
- Connected `Scenario::propagation` to radar and ISAC with deterministic per-domain RNG streams.
- Expanded the Orekit 13.1.8 sidecar into a full OEM/OCM Cartesian precision oracle.

### Build and supply chain
- Pinned Rust 1.85.0 consistently across local toolchain, CI and Docker.
- Removed the Docker unlocked-build fallback.
- Enforced `cargo-deny` and `cargo-audit` in CI/release checks.
- Removed nested Maven `target/` artifacts from the deliverable and hardened Maven shading against stale signatures/module descriptors.
- Made SGP4/Orekit version provenance mechanically checkable instead of relying on duplicated literals.

## 0.4.0

### Added
- Absolute astronomical epochs and absolute ephemerides while preserving legacy relative simulation epochs.
- TLE/OMM parsing and deterministic SGP4 propagation.
- CCSDS OEM/OCM KVN/XML parsers, multi-segment OEM support and OCM trajectory-block preservation.
- IERS EOP 20u24 C04/finals2000A ingestion and EOP-backed IAU/Vallado frame transforms.
- ITU-R P.676-13 gaseous attenuation and P.840-9 cloud attenuation.
- Ionospheric delay/phase/Faraday correction utilities.
- Ground-station geometry, antenna patterns/pointing loss and complete link-budget calculations.
- Absolute one-way light-time/range-rate/Doppler solver.
- Orekit 13.1.8 precision/reference sidecar with local-only `orekit-data`.
- Space-input and data-snapshot provenance hashing.

### Changed
- Space-link solver now rejects mixed reference frames/time scales instead of subtracting incompatible vectors.
- Space documentation distinguishes native capabilities from external-data-dependent precision models.

### Determinism / safety
- No runtime network downloads for EOP, atmospheric climatology or Orekit data.
- Strict bounded EOP interpolation; no extrapolation.
- Missing high-precision EOP fields are not silently replaced by zero.
- OCM non-Cartesian trajectories are retained as raw elements rather than misrepresented as Cartesian PV states.

## 0.2.0

- Added scientific validity ledger.
- Added Meissner-Wentz sea-water dielectric model and lossy Maxwell propagation.
- Added complex Fresnel interfaces and stratified medium propagation.
- Added authoritative ephemeris interpolation and light-time corrected space RF geometry.

# Changelog

## 0.1.1

- Export `sigint_scenario.json` directly in the public SIGINT Golden Path contract.
- Add cross-repository compatibility documentation for the two SDKs.


## 0.1.0

- deterministic Rust RF/SIGINT simulation;
- free-space, two-ray and seeded Rician propagation;
- receiver impairments and raw-I/Q export;
- monostatic radar and rotor micro-Doppler approximation;
- 5G/OFDM MIMO-ISAC observables;
- C-UAS Golden Path exporters;
- deterministic domain randomization and SHA-256 replay manifests;
- optional Gazebo, Sionna RT and 5G-LENA bridges.
