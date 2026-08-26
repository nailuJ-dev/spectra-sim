# Gazebo bridge

`world.sdf` is a minimal Gazebo Harmonic-compatible world for visual and physics integration experiments. The simulator core remains independent from Gazebo.

The bridge contract is a bounded ENU snapshot:

```json
{"schema_version":"spectra-gazebo-state-v1","entities":[{"id":"drone-01","position_enu_m":[180,130,85],"velocity_enu_mps":[4,12,1]}]}
```

Apply a snapshot:

```bash
python adapters/gazebo/state_snapshot_to_scenario.py \
  --scenario examples/scenarios/cuas_drone_isac.json \
  --snapshot adapters/gazebo/example_snapshot.json \
  --out artifacts/gazebo/cuas_from_gazebo.json
```

A production bridge may obtain the snapshot from Gazebo Transport, ROS 2 or a custom system plugin. Keep that transport layer outside the physics core so recorded state can be replayed without Gazebo.
