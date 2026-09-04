# spectra-sim 0.4.1 — audit remediation

This release applies the actionable findings from the v0.4.0 code audit and closes the associated integration gaps rather than leaving disconnected model code in the public API.

## Corrected defects

- Fresnel TE/TM reflection equations and TM transmission at oblique incidence.
- RMS-targeted AGC for duty-cycled and non-unit-power waveforms.
- Symmetric IQ gain/quadrature-skew model with I/Q cross-coupling.
- OCM bracketed `TRAJ_UNITS` preservation.
- OEM KVN/XML optional acceleration ingestion and preservation.
- `finals2000A` incomplete prediction-row handling without zero substitution.
- Passive-medium complex-permittivity sign validation.
- P.676-13 table caching and explicit 1–1000 GHz validity enforcement.
- WGS-84 ENU→ECEF→geodetic export instead of an unbounded flat local approximation.
- Duplicate entity-ID rejection and strict malformed `--seed` handling.
- Reproducible locked Docker/toolchain/CI build gates and enforced Rust supply-chain checks.
- Truthful SGP4/Orekit provenance and concise structured Orekit startup failures.
- Sionna IQ-imbalance parity and 5G-LENA row-bound validation.

## Integration gaps closed

- `roll_deg`, `pitch_deg` and `yaw_deg` now drive directional terrestrial antenna gain.
- Serializable antenna patterns are connected to SIGINT emitters/receivers and radar/ISAC configurations.
- `Scenario::propagation` now affects radar and ISAC, not only SIGINT.
- RF/material complex values now share one public `Complex64` implementation.
- Orekit OEM/OCM operations return complete Cartesian P/V/A state histories so the sidecar can be used as an independent validation oracle.

## Audit suggestions intentionally not applied

Two runbook suggestions were not implemented because their premises are incorrect:

1. **UTC two-part-JD normalization was not changed.** SOFA's UTC quasi-JD leap-second representation is already handled through `dtf2d`/UTC↔TAI conversions. A regression test around the 2016-12-31 leap second now verifies one-second SI spacing instead of changing working time arithmetic.
2. **`panic = "abort"` remains in the release profile.** Cargo's test harness does not use the release panic strategy in the way assumed by the audit; removing it would not fix a demonstrated defect.

The proposed stdin/stdout Orekit deadlock was also not reproduced by the protocol design: Java consumes a complete newline-delimited request before producing its response. The sidecar protocol remains bounded by the Rust/native input limits, while the oracle response has been expanded for validation.

## Output compatibility

`IqCaptureOut`, `SigintTruth` and `RadarMeasurement` gained fields. Raw I/Q samples and radar/ISAC values can change because previously disconnected or incorrect physics is now active. Regenerate downstream golden data and training datasets from the corrected simulator.
