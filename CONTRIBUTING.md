# Contributing

Before opening a pull request run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
python3 -m py_compile adapters/gazebo/state_snapshot_to_scenario.py adapters/sionna_rt/export_cir.py adapters/sionna_rt/simulate_iq_from_cir.py adapters/5g_lena/trace_to_isac_config.py tools/reference_check.py
```

Physics changes must include a closed-form regression test or a clearly documented validation reference. Keep stochastic tests seeded and reproducible.
