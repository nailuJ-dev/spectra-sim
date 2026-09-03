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
