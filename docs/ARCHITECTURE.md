# Architecture

```text
ScenarioManifest
      │
      ├── World / kinematics
      ├── Emitters / waveforms
      ├── Receivers / impairments
      ├── Targets / RCS / rotors
      ├── Environment
      └── seed
             │
             ▼
      Physics core (Rust)
        ┌────┼─────────┐
        ▼    ▼         ▼
       RF   Radar     ISAC
        │    │         │
        └────┼─────────┘
             ▼
       Ground truth
             │
       public exporters
        ┌────┴─────────┐
        ▼              ▼
   raw I/Q JSON   C-UAS scenario JSON
```

The simulator does not link against downstream model libraries. This preserves reproducibility, allows independent versioning, and prevents training or inference code from leaking into the physics layer.

High-fidelity tools are adapters, not dependencies:

```text
Gazebo ──state──┐
                ├──> Scenario / truth contracts ──> spectra-sim exporters
Sionna RT ─CIR──┤
                │
5G-LENA ─NR cfg─┘
```
