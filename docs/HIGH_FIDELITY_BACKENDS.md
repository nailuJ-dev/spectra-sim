# High-fidelity backends

## Gazebo Harmonic

Gazebo is used for rigid-body dynamics, trajectories and sensor/world state. `adapters/gazebo/world.sdf` is a minimal scene and `state_snapshot_to_scenario.py` maps an exported ENU state snapshot into a spectra-sim scenario.

The bridge intentionally transfers state through files rather than linking Gazebo into the Rust process. This keeps simulation runs reproducible and CI-friendly.

## Sionna RT 2.0.x

`adapters/sionna_rt/export_cir.py` uses the public Sionna RT API to:

- load a Mitsuba scene;
- configure transmitter and receiver arrays;
- place a transmitter and receiver;
- execute `PathSolver` with a fixed seed;
- export the non-normalized channel impulse response coefficients and delays.

The adapter output is an external propagation artifact. It is not required for L0/L1 runs.

## 5G-LENA

5G-LENA is used as an optional NR network / protocol configuration source, not as the electromagnetic sensing engine. `trace_to_isac_config.py` maps a bounded CSV export into the spectra-sim `IsacConfig` contract.

This division avoids claiming that network-system simulation alone is a radar simulator: range/Doppler/angle sensing remains governed by the physics engine or Sionna RT channel outputs.
