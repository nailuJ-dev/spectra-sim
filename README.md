# spectra-sim

**Physics-based RF sensing simulation for terrestrial, maritime, underwater and space environments.**

`spectra-sim` is an open-source simulation framework for generating realistic RF, radar and sensing scenarios.

The project is designed for engineers and researchers working on wireless systems, RF machine learning, radar, SIGINT, 5G/ISAC, satellite communications and electromagnetic sensing.

Its goal is simple: provide reproducible RF environments that are realistic enough to develop, benchmark and stress-test sensing algorithms before field deployment.

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

Model wireless and radar propagation through configurable environments with path loss, multipath, channel effects and receiver impairments.

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
* Doppler and Doppler rate
* Earth-space RF links
* antenna pointing and geometry

The architecture is intended to support high-precision astrodynamics backends such as Orekit without coupling the Rust core to a specific external runtime.

`spectra-sim` 0.4 adds a deterministic space-link stack for research and engineering workflows:

* TLE/OMM + SGP4/SDP4 propagation;
* CCSDS OEM/OCM KVN/XML ingestion;
* IERS EOP-backed TEME/GCRF/ITRF transformations;
* absolute UTC/TAI/TT/UT1/GPS time handling;
* ITU-R P.676-13 gaseous and P.840-9 cloud attenuation;
* explicit rain/scintillation inputs for P.618/P.838 workflows;
* first-order ionospheric delay, phase advance and Faraday rotation;
* WGS-84 ground geometry, antenna pointing and link budgets;
* optional Orekit 13.1.8 precision/reference backend using a pinned local `orekit-data` snapshot.

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

## Quick start

```bash
git clone https://github.com/nailuJ-dev/spectra-sim.git
cd spectra-sim

cargo build --release
cargo test --all-targets --all-features
```

Explore the CLI:

```bash
cargo run --bin spectra-sim -- --help
```

Example scenarios are available in the repository.

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
