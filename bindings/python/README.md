# spectra-sim for Python

Python bindings (PyO3 + maturin) for the deterministic `spectra-sim` RF, radar
and ISAC simulation toolkit. The results are bit-identical to the Rust CLI: the
replay manifest hashes match.

## Install (development)

```sh
python -m venv .venv && . .venv/bin/activate
pip install maturin pytest numpy
cd bindings/python
maturin develop
pytest tests
```

## Usage

```python
import json
import spectra_sim

scenario = open("examples/scenarios/sigint_urban.json").read()

spectra_sim.validate(scenario)                # raises ValueError if invalid

result = spectra_sim.run_sigint(scenario)
result.iq                                     # numpy complex64 array
result.sample_rate_hz, result.center_frequency_hz
truth = json.loads(result.truth_json)
manifest = json.loads(result.manifest_json)

run = json.loads(spectra_sim.run_cuas(open("examples/scenarios/cuas_drone_isac.json").read()))

# "match", "scenario_mismatch", "version_mismatch", "schema_mismatch" or "output_mismatch"
spectra_sim.verify_replay(scenario, result.manifest_json, "sigint")

spectra_sim.__version__
```

Scenarios are passed as JSON strings (at most 32 MiB) and are validated exactly
as the CLI does. Library errors become `ValueError` (I/O errors become
`OSError`).

## Layout and version choices

- The native extension is `spectra_sim._spectra_sim` (cdylib lib name
  `_spectra_sim`, to avoid clashing with the `spectra_sim` library crate it
  depends on); the pure-Python package `spectra_sim` in `python/` re-exports it.
- `pyo3 = "0.23"` (resolved 0.23.5) and `numpy = "0.23"` (resolved 0.23.0)
  compile on the pinned Rust 1.85.0 and support Python 3.13, so no lower pin or
  newer line was needed. The extension is built with `abi3-py39`, one wheel per
  platform for Python >= 3.9.
- `pyo3/extension-module` is enabled through `[tool.maturin] features` rather
  than in `Cargo.toml`, so `cargo clippy/test --workspace` links without
  libpython.
- The crate version must equal the root crate's version (`Cargo.toml` and
  `pyproject.toml`); the test suite checks `__version__` against the root crate.
