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
