# SDK integration

`spectra-sim` communicates with downstream consumers through JSON only.

## SIGINT

```bash
cargo run --release -- sigint examples/scenarios/sigint_urban.json --out artifacts/sigint
```

Use `artifacts/sigint/iq_capture.json` as the raw I/Q capture. It contains exactly:

- `id`
- `timestamp_ms`
- `sample_rate_hz`
- `center_frequency_hz`
- `samples[]` with `i` and `q`

`truth.json` is intentionally separate and must not be fed to the model during normal inference.

## C-UAS recorded sensing

```bash
cargo run --release -- cuas examples/scenarios/cuas_drone_isac.json --out artifacts/cuas
```

`cuas_scenario.json` contains a recorded reference measurement, candidate kinematics, optional cooperative identity and expected class label.

## C-UAS recorded ISAC

The same run emits `cuas_isac_scenario.json` with the 13-dimensional public reference feature vector and an explicit simulated authorization/configuration token.

These tokens are simulation metadata only. They do not authorize use of a real network or active transmitter.


## Direct Golden Path handoff

`spectra-sim sigint ...` writes both `iq_capture.json` and `sigint_scenario.json`. The latter is the complete public wrapper consumed by `sigint-golden-demo --scenario`, so no repository dependency or conversion script is required.

## Output files

| Command | Files written to `--out` |
| --- | --- |
| `sigint` | `iq_capture.json`, `truth.json`, `sigint_scenario.json`, `replay_manifest.json`, `validity_report.md`; with `--also`: `iq_capture.npy`, `iq_capture.sigmf-data`, `iq_capture.sigmf-meta` |
| `cuas` | `radar_truth.json`, `cuas_scenario.json`, `replay_manifest.json`, `validity_report.md`; with an ISAC configuration also `isac_truth.json` and `cuas_isac_scenario.json` |
| `plot` | `constellation.svg`, `spectrum.svg`, `spectrogram.svg` |
| `sweep` | `sweep_manifest.json` and one `seed_<n>/` directory per seed with that seed's run files (no per-seed effective scenario: reproduce it with `randomize --seed N`) |

## I/Q file layouts

`iq_capture.json` is the canonical capture. `sigint --also npy,sigmf` additionally writes the same samples in two ML-friendly layouts:

- **NumPy** (`iq_capture.npy`): NumPy format 1.0, dtype `<c8` (`complex64`, little-endian), shape `(N,)`. Load with `numpy.load`.
- **SigMF** (`iq_capture.sigmf-data` and `iq_capture.sigmf-meta`): the data file is the raw interleaved little-endian `f32` stream `I0 Q0 I1 Q1 ...` (SigMF datatype `cf32_le`). The metadata file has `global` (`core:version`, `core:datatype`, `core:sample_rate`, `core:dataset` naming the data file) and one `captures` segment (`core:sample_start` 0, `core:frequency` = the centre frequency). The optional checksum field is not written.

Samples are `f32` in all three layouts, so the three files carry identical values.

## JSON Schemas

The `schemas/` directory holds JSON Schemas generated from the Rust types and kept in sync by a test (`tests/schema_contracts.rs`):

| File | Describes |
| --- | --- |
| `schemas/scenario.schema.json` | the scenario input file accepted by every command |
| `schemas/sigint_run.schema.json` | the in-memory `SigintRun` aggregate returned by `run_sigint` |
| `schemas/cuas_run.schema.json` | the in-memory `CuasRun` aggregate returned by `run_cuas` |

`spectra-sim schema scenario|sigint-run|cuas-run` prints the same documents (the `schema` Cargo feature, on by default). The two run schemas describe the Rust return values (simulation result plus manifest), not a file the CLI writes: the CLI splits that data into the separate files listed above, so do not validate `iq_capture.json` or `truth.json` against them.

To regenerate after changing a type: `cargo run -q -- schema scenario > schemas/scenario.schema.json` (likewise `sigint-run` and `cuas-run`).

## Replay manifest and `output_schema_version`

`replay_manifest.json` records `schema_version` (`spectra-sim-replay-v1`), `seed`, `scenario_sha256`, `output_sha256`, `simulator_version` and `output_schema_version`. `output_schema_version` (currently `spectra-sim-output-v1`) names the version of the serialized output contract that `output_sha256` covers; it is bumped when that serialization changes. Manifests written before 0.5.0 have no such field and are read with the value `unversioned`. `simulator_version` is informational: it is part of the manifest but not of `output_sha256`, so a new simulator version that produces identical output still matches.

`spectra-sim replay <scenario> <manifest> --kind sigint|cuas` (library: `verify_replay`) decides in this order:

1. `ScenarioMismatch`: the hash of the scenario differs from `scenario_sha256`. Nothing is re-run.
2. The scenario is re-run. If the new output hash equals `output_sha256` the result is `Match` (printed as `MATCH`, exit 0), whatever the manifest's version or schema tag.
3. Otherwise `VersionMismatch` if `simulator_version` differs from the running simulator,
4. else `SchemaMismatch` if `output_schema_version` differs from the current one (this includes `unversioned` legacy manifests),
5. else `OutputMismatch`: same simulator and schema, different output, i.e. a determinism or regression problem.

Any verdict other than `Match` is reported on stderr and exits with code 2. `ReplayVerdict` and `RunKind` are `#[non_exhaustive]`; library code must keep a wildcard arm.

## Limits and exit codes

Every JSON input the CLI reads (scenario, randomization config, replay manifest, I/Q capture for `plot`) is capped at 32 MiB; a larger file is rejected with exit code 2. The Python bindings apply the same cap to scenario strings.

| Exit code | Meaning |
| --- | --- |
| 0 | success (`replay`: `MATCH`) |
| 2 | invalid input or usage: bad arguments, invalid or oversized scenario, malformed JSON, replay mismatch |
| 3 | I/O error: a file cannot be read, created or written |

Exit code 3 is new in 0.5.0; earlier versions exited with 2 for I/O errors as well.

