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
- `pyo3 = "0.29"` (resolved 0.29.3) and `numpy = "0.29"` (resolved 0.29.0)
  compile on the pinned Rust 1.85.0 and support Python 3.13. Older lines were
  rejected by `cargo audit`: pyo3 < 0.24.1 has RUSTSEC-2025-0020 and
  pyo3 < 0.29 has RUSTSEC-2026-0176/0177. No advisory is ignored. The extension is built with `abi3-py39`, one wheel per
  platform for Python >= 3.9.
- `pyo3/extension-module` is enabled through `[tool.maturin] features` rather
  than in `Cargo.toml`, so `cargo clippy/test --workspace` links without
  libpython.
- The package version is taken from `bindings/python/Cargo.toml`
  (`dynamic = ["version"]` in `pyproject.toml`), which must equal the root
  crate's version; the test suite checks `__version__` against the root crate.
- Building an sdist or `pip install` from source needs the core crate (a path
  dependency on `../..`) bundled alongside; publishing to PyPI is out of scope
  for now.
