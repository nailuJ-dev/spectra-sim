# spectra-sim User-Facing Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the 15 user-facing improvements proposed after the repo analysis (CLI, validation, formats, bindings, visualisation, Doppler rate, distribution, quality gates) as release 0.5.0.

**Architecture:** The simulation core (`src/*.rs`) stays dependency-light and unchanged in behaviour; every serialized output shape and every existing replay hash stays byte-identical. New user-facing code lives in new modules: `src/cli/` (command layer, used only by the binary), `src/formats.rs` (npy/SigMF), `src/viz.rs` (SVG), `src/report.rs` (Markdown), plus a separate `bindings/python` workspace crate for PyO3. JSON Schema generation is an opt-in default feature.

**Tech Stack:** Rust 1.85 (pinned), clap 4 (derive), schemars 0.8 (optional feature `schema`), std threads for sweeps, PyO3 0.23 + rust-numpy 0.23 + maturin, GitHub Actions, cargo-fuzz, cargo-semver-checks, cargo-llvm-cov.

**Spec:** No separate spec document. The requirements are items 1–15 of the "Améliorations proposées pour les utilisateurs" list in the session that produced this plan (session `01M3JkKVwUpyDTyqzwpHkkGE`). Decisions that deviate from that list: HDF5 is dropped (needs a C library; `.npy` + SigMF cover ML and SDR use), and the Doppler-rate item is library-only because the CLI has no space command.

## Global Constraints

- Rust toolchain `1.85.0`, `edition = "2021"`, `rust-version = "1.85"`; `Cargo.lock` committed, builds use `--locked`.
- `#![forbid(unsafe_code)]` in the core crate; `clippy::unwrap_used`/`expect_used` denied outside tests (already enforced in `src/lib.rs` and the binary — keep the attribute on the new `src/cli/` code via the binary's crate attribute).
- Gate for every commit: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-targets --all-features` all pass.
- Determinism: no new output may depend on thread count, wall clock, locale or hash-map iteration order. Existing `output_sha256` values must not change.
- Existing serialized structs keep `#[serde(deny_unknown_fields)]`; fields added to them must be `#[serde(default)]` so older files still parse.
- No network access at runtime. `cargo deny` licences stay `MIT`, `Apache-2.0`, `Unicode-3.0` (+ `BSD-2-Clause` only if Task 12's `numpy` crate requires it, added to `deny.toml` explicitly).
- CLI exit codes: `0` ok, `2` invalid input (`InvalidArgument`, `NotFound`, `DimensionLimit`, `NonFinite`, `Json`), `3` I/O (`Io`), `1` anything else. The existing message `--seed must be an unsigned integer: <raw>` is preserved verbatim.
- Existing CLI invocations (`sigint`, `cuas`, `randomize`, with `--out`/`--seed`) keep working unchanged.

## Review Focus

- A scenario file that is empty, not JSON, over 32 MiB, or has an unknown field → `validate`/`sigint`/`cuas` exit `2` with a one-line message naming the problem, never a panic or a backtrace.
- `--out` pointing at an unwritable path or an existing file → exit `3` with the path in the message; `sweep` refuses a non-empty output directory unless `--force` is given.
- `sweep --seeds` with an empty (`5..5`), reversed (`10..2`), non-numeric, or enormous (`0..1000000000`) range → exit `2`; the limit is 100 000 seeds.
- An I/Q capture with 0 samples, 1 sample, or a non-power-of-two length → `.npy`, SigMF and SVG outputs are produced without panicking (SVG shows an empty plot).
- `replay --verify` against a manifest written by a different `simulator_version`, or with `output_schema_version` absent (pre-0.5.0 file) → reported as a distinct, non-hash message and still exit `2` on mismatch, exit `0` on match.

---

## File Structure

| Path | Responsibility |
|---|---|
| `src/replay.rs` (modify) | `ReplayManifest.output_schema_version`; `verify_replay` |
| `src/cli/mod.rs` (create) | clap definitions, dispatch, exit-code mapping |
| `src/cli/{validate,examples,replay_cmd,sweep,schema_cmd,plot_cmd,io}.rs` (create) | one file per command; `io.rs` = bounded read/write helpers |
| `src/bin/spectra-sim.rs` (modify) | thin `main` that includes `src/cli/mod.rs` via `#[path = "../cli/mod.rs"] mod cli;` |
| `src/formats.rs` (create) | `.npy` and SigMF encoders |
| `src/viz.rs` (create) | SVG constellation / spectrum / spectrogram / timeline |
| `src/report.rs` (create) | Markdown validity report |
| `src/space/absolute_link.rs` (modify) | `doppler_rate_hz_per_s` |
| `schemas/*.schema.json` (create, generated) | published JSON Schemas |
| `examples/*.rs` (create) | runnable library examples |
| `bindings/python/` (create) | PyO3 crate, `pyproject.toml`, pytest suite |
| `fuzz/` (create) | cargo-fuzz targets (excluded from the workspace) |
| `.github/workflows/{ci,release}.yml` (modify/create) | python, coverage, semver, release jobs |

---

### Task 1: Output schema version in the replay manifest

**Files:**
- Modify: `src/replay.rs`
- Test: `tests/sdk_contracts.rs` (append)

**Interfaces:**
- Produces: `pub const OUTPUT_SCHEMA_VERSION: &str = "spectra-sim-output-v1";` and `ReplayManifest.output_schema_version: String` (`#[serde(default = "legacy_output_schema")]`, default value `"unversioned"`). `replay_manifest()` sets it to `OUTPUT_SCHEMA_VERSION`. `output_sha256` computation is untouched.

- [ ] **Step 1: Write failing tests** in `tests/sdk_contracts.rs`:
  - `manifest_carries_output_schema_version`: `run_sigint(&sigint_golden_scenario_input)` → `manifest.output_schema_version == "spectra-sim-output-v1"`.
  - `legacy_manifest_without_schema_field_still_parses`: parse the JSON of a manifest with the field removed → `output_schema_version == "unversioned"`.
  - `output_hash_is_unchanged`: for `examples/scenarios/sigint_urban.json`, assert `manifest.output_sha256` equals the value captured *before* this task (run the current binary first and paste the hex into the test).
- [ ] **Step 2:** Capture the pre-change hash: `cargo run -q -- sigint examples/scenarios/sigint_urban.json --out /tmp/h && grep output_sha256 /tmp/h/replay_manifest.json`. Run `cargo test --test sdk_contracts` → the first two tests FAIL (field missing).
- [ ] **Step 3: Implement** the field, constant and serde default in `src/replay.rs`.
- [ ] **Step 4:** `cargo test --test sdk_contracts` → PASS (including the hash test).
- [ ] **Step 5: Commit** `feat: version the output contract in the replay manifest`.

---

### Task 2: Migrate the CLI to clap with typed exit codes

**Files:**
- Modify: `Cargo.toml` (add `clap = { version = "4", features = ["derive"] }`), `src/bin/spectra-sim.rs`
- Create: `src/cli/mod.rs`, `src/cli/io.rs`
- Test: `tests/cli_validation.rs` (append)

**Interfaces:**
- Produces in `src/cli/io.rs`: `load_scenario(path: &Path) -> Result<Scenario>` (32 MiB cap, `validate()` called), `write_json(path: &Path, value: &impl Serialize) -> Result<()>` (creates no parents; caller does), `ensure_dir(path: &Path) -> Result<()>`.
- Produces in `src/cli/mod.rs`: `pub fn run() -> i32` returning the exit code; `pub fn exit_code(error: &SimError) -> i32` implementing the Global Constraints table; `#[derive(clap::Parser)] struct Cli` with subcommand enum `Command { Sigint, Cuas, Randomize }` (more variants are added by later tasks). `--seed` is declared as `Option<String>` and parsed manually so the legacy message is preserved.

- [ ] **Step 1: Write failing tests** (spawn the binary like the existing test):
  - `help_lists_subcommands`: `--help` exits 0, stdout contains `sigint`, `cuas`, `randomize`.
  - `version_flag`: `--version` stdout contains `env!("CARGO_PKG_VERSION")`.
  - `missing_scenario_file_exits_3`: `sigint /nonexistent.json` → code 3, stderr contains `/nonexistent.json`.
  - `bad_json_exits_2`: a temp file containing `{` → code 2.
  - `legacy_seed_message_preserved`: existing test must still pass untouched.
- [ ] **Step 2:** `cargo test --test cli_validation` → new tests FAIL.
- [ ] **Step 3: Move** the body of the old `run()` per command into `src/cli/mod.rs` unchanged in behaviour (same output file names, same defaults `artifacts/sigint`, `artifacts/cuas`, `artifacts/randomized_scenario.json`). Binary `main` becomes `std::process::exit(cli::run())`. Keep the crate-level `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]` on the binary.
- [ ] **Step 4:** Run full gate; also run `cargo run -q -- sigint examples/scenarios/sigint_urban.json --out /tmp/h2` and `diff -r /tmp/h /tmp/h2` → empty (Task 1's output).
- [ ] **Step 5: Commit** `feat(cli): clap-based CLI with typed exit codes`.

---

### Task 3: `validate` command

**Files:**
- Create: `src/cli/validate.rs`
- Modify: `src/cli/mod.rs` (add `Command::Validate { scenario: PathBuf, #[arg(long)] json: bool }`)
- Test: `tests/cli_validation.rs`

**Interfaces:**
- Consumes: `load_scenario`.
- Produces: `validate::run(path: &Path, json: bool) -> Result<()>`. Success prints `ok: <n_emitters> emitters, <n_receivers> receivers, <n_targets> targets, jobs: [sigint, cuas]` (only the jobs present); with `--json` prints `{"valid":true,"jobs":[...]}`. Failure prints the `SimError` text to stderr and exits via `exit_code`.

- [ ] **Step 1: Write failing tests**: `validate_accepts_all_example_scenarios` (loop `examples/scenarios/*.json` except `randomization.json`, expect exit 0 and stdout starting `ok:`); `validate_rejects_unknown_field` (temp scenario with extra key → exit 2, stderr contains `unknown field`); `validate_rejects_duplicate_ids` (duplicate emitter id → exit 2, stderr contains the id); `validate_json_flag` (stdout parses as JSON with `valid == true`).
- [ ] **Step 2:** run → FAIL (no subcommand).
- [ ] **Step 3: Implement** `validate::run` using `load_scenario` only; no simulation is run.
- [ ] **Step 4:** run → PASS. **Step 5: Commit** `feat(cli): add validate command`.

---

### Task 4: `examples` command, embedded scenarios and runnable library examples

**Files:**
- Create: `src/cli/examples.rs`, `examples/quickstart_sigint.rs`, `examples/quickstart_cuas.rs`
- Modify: `src/cli/mod.rs` (add `Command::Examples { #[command(subcommand)] action: ExamplesAction }`, `ExamplesAction::{List, Show { name: String }}`)
- Test: `tests/cli_validation.rs`

**Interfaces:**
- Produces: `examples::EMBEDDED: &[(&str, &str)]` — `(name, include_str!("../../examples/scenarios/<name>.json"))` for the 8 scenario files, names without extension. `examples::list() -> Vec<&'static str>`; `examples::show(name: &str) -> Result<&'static str>` (`SimError::NotFound` for unknown names).
- The two `examples/*.rs` programs load an embedded-equivalent file path from `examples/scenarios/`, call `run_sigint` / `run_cuas`, and print `snr_db` / the radar measurement; they must run with `cargo run --example quickstart_sigint`.

- [ ] **Step 1: Write failing tests**: `examples_list_has_eight_entries` (stdout has 8 lines incl. `sigint_urban`); `examples_show_round_trips` (stdout equals file contents of `examples/scenarios/sigint_urban.json`); `examples_show_unknown_exits_2`; `embedded_examples_all_validate` (each embedded string deserializes into `Scenario` and `.validate()` is Ok — except `randomization`, which must parse as `DomainRandomization`).
- [ ] **Step 2:** run → FAIL. **Step 3: Implement** as specified. **Step 4:** run → PASS, and `cargo run --example quickstart_sigint` exits 0.
- [ ] **Step 5: Commit** `feat(cli): examples command and runnable library examples`.

---

### Task 5: `replay --verify`

**Files:**
- Modify: `src/replay.rs`, `src/cli/mod.rs`
- Create: `src/cli/replay_cmd.rs`
- Test: `tests/determinism.rs` (library), `tests/cli_validation.rs` (CLI)

**Interfaces:**
- Produces in `src/replay.rs`:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum RunKind { Sigint, Cuas }
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum ReplayVerdict { Match, OutputMismatch { expected: String, actual: String },
      ScenarioMismatch { expected: String, actual: String },
      VersionMismatch { manifest: String, current: String } }
  pub fn verify_replay(scenario: &Scenario, manifest: &ReplayManifest, kind: RunKind) -> Result<ReplayVerdict>;
  ```
  Order of checks: `scenario_sha256`, then `simulator_version` (only reported if hashes differ downstream — see tests), then re-run and compare `output_sha256`.
- CLI: `replay <scenario.json> <replay_manifest.json> --kind sigint|cuas` (`--kind` required; prints `MATCH` or a one-line reason; exit 0 on `Match`, 2 otherwise).

- [ ] **Step 1: Write failing tests**: `verify_matches_fresh_manifest` (`run_sigint` then `verify_replay(.., Sigint)` → `Match`); same for `Cuas`; `verify_detects_scenario_change` (alter `scenario.seed`; manifest from original → `ScenarioMismatch`); `verify_detects_tampered_output_hash` (flip one hex char → `OutputMismatch`); `verify_reports_version_mismatch` (manifest `simulator_version = "0.0.0"` and tampered output hash → `VersionMismatch`, *not* `OutputMismatch`); CLI `replay_cli_exit_codes` (0 on untouched manifest, 2 on tampered).
- [ ] **Step 2:** run → FAIL. **Step 3: Implement**; reuse `run_sigint`/`run_cuas` and `canonical_sha256`. **Step 4:** run → PASS. **Step 5: Commit** `feat: replay verification (library + CLI)`.

---

### Task 6: `sweep` — batch / Monte Carlo runs

**Files:**
- Create: `src/cli/sweep.rs`
- Modify: `src/cli/mod.rs`
- Test: `tests/cli_validation.rs`, `tests/determinism.rs`

**Interfaces:**
- Consumes: `load_scenario`, `run_sigint`, `run_cuas`, `DomainRandomization::apply(&Scenario, u64) -> Result<Scenario>` (existing), `RunKind`.
- Produces: `sweep::parse_seed_range(raw: &str) -> Result<std::ops::Range<u64>>` (`"A..B"`, half-open, `B > A`, `B - A <= 100_000`); `#[derive(Serialize, Deserialize)] struct SweepManifest { schema_version: String /* "spectra-sim-sweep-v1" */, kind: String, entries: Vec<SweepEntry> }`, `SweepEntry { seed: u64, output_sha256: String }`, entries sorted by seed.
- CLI: `sweep <scenario.json> --kind sigint|cuas --seeds A..B [--randomization FILE] [--jobs N] [--out DIR] [--force]`. Each seed writes `DIR/seed_<seed>/` with the same files the single-run command writes, plus `DIR/sweep_manifest.json`. Default `--jobs` = `std::thread::available_parallelism()`; implemented with `std::thread::scope` and an `AtomicU64` work counter (no new dependency). The first error aborts remaining work and is returned.

- [ ] **Step 1: Write failing tests**: `parse_seed_range` table (`"0..3"` ok; `"5..5"`, `"10..2"`, `"a..b"`, `"0..1000000000"`, `"3"` → `InvalidArgument`); `sweep_is_independent_of_job_count` (seeds `0..4` with `--jobs 1` and `--jobs 4` produce byte-identical `sweep_manifest.json`); `sweep_writes_one_dir_per_seed`; `sweep_refuses_non_empty_out_without_force` (exit 3); `sweep_with_randomization_changes_hashes` (with `examples/scenarios/randomization.json` the entries differ across seeds).
- [ ] **Step 2:** run → FAIL. **Step 3: Implement.** **Step 4:** run → PASS. **Step 5: Commit** `feat(cli): sweep command for Monte Carlo batches`.

---

### Task 7: Published JSON Schemas

**Files:**
- Modify: `Cargo.toml` (`schemars = { version = "0.8", optional = true }`, `[features] default = ["schema"]`, `schema = ["dep:schemars"]`), every `pub` type reachable from `Scenario`, `SigintRun`, `CuasRun` (add `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]`)
- Create: `src/cli/schema_cmd.rs`, `schemas/scenario.schema.json`, `schemas/sigint_run.schema.json`, `schemas/cuas_run.schema.json`
- Test: `tests/schema_contracts.rs` (create)

**Interfaces:**
- Produces: `pub fn scenario_schema() -> serde_json::Value`, `sigint_run_schema()`, `cuas_run_schema()` in `src/lib.rs` re-exports (module `schema`, `#[cfg(feature = "schema")]`). CLI: `schema scenario|sigint-run|cuas-run` prints pretty JSON.

- [ ] **Step 1: Write failing tests** in `tests/schema_contracts.rs` (`#![cfg(feature = "schema")]`): `committed_schemas_are_current` (each generated schema equals the parsed committed file); `every_example_scenario_matches_schema` — validate each scenario in `examples/scenarios/` structurally by checking that every top-level key is declared in `schema["properties"]` of the scenario schema (no new validator dependency); `schema_cli_prints_json` (stdout parses, has `"$schema"`).
- [ ] **Step 2:** run → FAIL. **Step 3: Implement** the derives (mechanical; chase compile errors type by type; a type with a manual `Serialize` needs a manual `JsonSchema`), the three functions and the CLI command; generate the committed files with `cargo run -q -- schema scenario > schemas/scenario.schema.json` etc. **Step 4:** run full gate, plus `cargo build --no-default-features` compiles. **Step 5: Commit** `feat: publish JSON schemas for scenarios and run outputs`.

---

### Task 8: `.npy` and SigMF export

**Files:**
- Create: `src/formats.rs`; modify `src/lib.rs` (`pub mod formats;` + re-export), `src/cli/mod.rs` (`sigint --also npy,sigmf`)
- Test: `tests/export_contracts.rs`

**Interfaces:**
- Consumes: `IqCaptureOut { samples: Vec<IqSampleOut{i:f32,q:f32}>, sample_rate_hz, center_frequency_hz, timestamp_ms, .. }`.
- Produces:
  - `pub fn iq_to_npy_bytes(capture: &IqCaptureOut) -> Vec<u8>` — NPY format 1.0, `descr: '<c8'`, `fortran_order: False`, `shape: (N,)`; header padded so total header length is a multiple of 64 and ends with `\n`; payload = `i` then `q` little-endian `f32` per sample.
  - `pub fn sigmf_data_bytes(capture: &IqCaptureOut) -> Vec<u8>` — same payload, `cf32_le`.
  - `pub fn sigmf_meta_json(capture: &IqCaptureOut, data_file_name: &str) -> serde_json::Value` with `global{"core:datatype":"cf32_le","core:sample_rate","core:version":"1.0.0","core:sha512"? omitted}`, `captures[0]{"core:sample_start":0,"core:frequency"}`, `annotations: []`.
- CLI writes `iq_capture.npy`, `iq_capture.sigmf-data`, `iq_capture.sigmf-meta` in addition to the always-written `iq_capture.json`.

- [ ] **Step 1: Write failing tests**: `npy_header_is_valid` (starts with `b"\x93NUMPY\x01\x00"`, `(10 + header_len) % 64 == 0`, header text contains `'<c8'` and `(3,)` for a 3-sample capture); `npy_payload_round_trips` (decode the payload back to the original `i`/`q`); `npy_empty_and_single_sample` (N=0 → shape `(0,)`, no panic; N=1 → `(1,)`); `sigmf_meta_fields` (sample rate / frequency / datatype / version asserted); `cli_also_flag_writes_files` (`--also npy,sigmf` → three extra files exist; unknown value → exit 2).
- [ ] **Step 2:** run → FAIL. **Step 3: Implement** by hand (no new dependency). **Step 4:** run → PASS; additionally, if `python3 -c "import numpy"` works, `np.load` the produced file and check `len == N` (manual check, not a test). **Step 5: Commit** `feat: npy and SigMF I/Q export`.

---

### Task 9: SVG visualisation and `plot` command

**Files:**
- Create: `src/viz.rs`, `src/cli/plot_cmd.rs`; modify `src/lib.rs`, `src/cli/mod.rs`
- Test: `tests/viz.rs` (create)

**Interfaces:**
- Produces:
  - `pub fn constellation_svg(capture: &IqCaptureOut, max_points: usize) -> String` (decimates uniformly to ≤ `max_points`; equal axes).
  - `pub fn spectrum_svg(capture: &IqCaptureOut, fft_len: usize) -> Result<String>` (`fft_len` must be a power of two ≥ 2, else `InvalidArgument`; Hann window, averaged periodogram; captures shorter than `fft_len` are zero-padded; 0 samples → empty plot).
  - `pub fn spectrogram_svg(capture: &IqCaptureOut, fft_len: usize, hop: usize, max_cells: usize) -> Result<String>` (rect heatmap; refuses `hop == 0`; downsamples so cells ≤ `max_cells`).
  - `pub fn timeline_svg(title: &str, y_label: &str, series: &[(f64, f64)]) -> String` (for Doppler/elevation vs time from library users; non-finite points are skipped).
  - private radix-2 FFT in `viz.rs`.
- CLI: `plot <iq_capture.json> --out DIR [--fft 256]` writes `constellation.svg`, `spectrum.svg`, `spectrogram.svg`.

- [ ] **Step 1: Write failing tests**: each function's output starts with `<svg` and ends with `</svg>`; `spectrum_peak_at_tone` (synthetic complex tone at `fs/8`, `fft_len=64` → the max bin index in an exposed `pub(crate) fn power_spectrum_db(&[IqSampleOut], usize) -> Vec<f64>` equals 8); `spectrum_rejects_non_power_of_two` (`fft_len=100` → Err); `zero_and_one_sample_do_not_panic`; `timeline_skips_nan`; `constellation_decimates` (100 000 samples, `max_points=500` → ≤ 500 `<circle`); `plot_cli_writes_three_files`.
- [ ] **Step 2:** run → FAIL. **Step 3: Implement** with string building only. **Step 4:** run → PASS; open one SVG from `sigint_urban` in a browser/Read to eyeball it. **Step 5: Commit** `feat: SVG constellation, spectrum, spectrogram and timeline plots`.

---

### Task 10: Human-readable validity report

**Files:**
- Create: `src/report.rs`; modify `src/lib.rs`, `src/cli/mod.rs`
- Test: `tests/validity.rs`

**Interfaces:**
- Consumes: `ValidityLedger::{models() -> &[ModelMetadata], issues() -> &[ValidityIssue], quality() -> PhysicsQuality}` from `src/validity.rs`.
- Produces: `pub fn validity_report_markdown(ledger: &ValidityLedger) -> String` — a title, an overall-quality line, a table `| Model | Maturity | Reference | Validity range |` (columns come from `ModelMetadata`), then an `## Issues` list (`severity — component: message`), or `No issues recorded.`.
- CLI: `sigint` and `cuas` write `validity_report.md` next to the JSON outputs. Locate where the run builds its ledger (`grep -rn ValidityLedger src`); if the run results do not expose it, add a sibling function (`run_sigint_with_ledger`) rather than adding a field to the serialized structs, so existing hashes are unchanged.

- [ ] **Step 1: Write failing tests**: `report_lists_models_and_quality` (ledger with one model, one `warning` issue → output contains the model name, the quality label and the issue message); `empty_ledger_report` (contains `No issues recorded.`); `cli_writes_validity_report` (file exists, non-empty, after `sigint` on `sigint_urban.json`).
- [ ] **Step 2:** run → FAIL. **Step 3: Implement.** **Step 4:** run → PASS and the `output_hash_is_unchanged` test from Task 1 still passes. **Step 5: Commit** `feat: Markdown validity report`.

---

### Task 11: Native Doppler rate

**Files:**
- Modify: `src/space/absolute_link.rs`
- Test: `tests/space_absolute_ephemeris.rs`

**Interfaces:**
- Consumes: `solve_absolute_one_way_link(tx, rx, receive_epoch, carrier_frequency_hz, max_iterations, tolerance_s, eop) -> Result<AbsoluteSpaceLinkGeometry>`.
- Produces: `pub fn doppler_rate_hz_per_s(tx: &AbsoluteEphemeris, rx: &AbsoluteEphemeris, receive_epoch: AbsoluteEpoch, carrier_frequency_hz: f64, max_iterations: usize, tolerance_s: f64, eop: Option<&EopSample>, step_s: f64) -> Result<f64>` — central difference of `doppler_hz` at `receive_epoch ± step_s` (`step_s` finite and > 0, else `InvalidArgument`). `AbsoluteSpaceLinkGeometry` is **not** changed (serialization compatibility).

- [ ] **Step 1: Write failing tests** (build ephemerides as the existing tests in this file do): `doppler_rate_matches_closed_form_at_closest_approach` — receiver on a straight line with speed `v` passing a fixed transmitter at distance `R0`, at closest approach the analytic rate is `-(f/c)·v²/R0`; assert relative error < 1e-3 for `v = 7500 m/s`, `R0 = 600 km`, `f = 2.2 GHz`; `doppler_rate_is_negative_for_approach_then_recede_sign` (sign flips between before and after closest approach); `doppler_rate_rejects_bad_step` (`0.0`, `NaN`, negative).
- [ ] **Step 2:** run → FAIL. **Step 3: Implement.** **Step 4:** run → PASS. **Step 5:** In `README.md`, replace "(Doppler rate is not yet a native observable)" with a mention of `doppler_rate_hz_per_s`. **Step 6: Commit** `feat(space): native Doppler rate observable`.

---

### Task 12: Python bindings (`pip install spectra-sim`)

**Files:**
- Modify: root `Cargo.toml` (add `[workspace] members = ["bindings/python"]`), `deny.toml` (only if needed), `.github/workflows/ci.yml` (job `python-bindings`)
- Create: `bindings/python/Cargo.toml`, `bindings/python/src/lib.rs`, `bindings/python/pyproject.toml` (maturin, module name `spectra_sim`), `bindings/python/tests/test_bindings.py`, `bindings/python/README.md`

**Interfaces:**
- Consumes: `spectra_sim::{Scenario, run_sigint, run_cuas, verify_replay, formats::iq_to_npy_bytes}` — the crate depends on the root crate by path with `default-features = false`.
- Produces (Python): `spectra_sim.run_sigint(scenario_json: str) -> SigintResult` with attributes `iq: numpy.ndarray[complex64]`, `truth_json: str`, `manifest_json: str`, `sample_rate_hz: float`, `center_frequency_hz: float`; `spectra_sim.run_cuas(scenario_json: str) -> str` (run JSON); `spectra_sim.validate(scenario_json: str) -> None` (raises `ValueError` with the `SimError` text); `spectra_sim.__version__`. `SimError` → `ValueError` (`Io` → `OSError`).
- Versions: `pyo3 = "0.23"`, `numpy = "0.23"` — if `cargo check` on 1.85 rejects them, take the newest releases that compile on 1.85 and record the choice in the crate README. The bindings crate may use `unsafe` through PyO3 macros; the `forbid(unsafe_code)` attribute stays only on the core crate.

- [ ] **Step 1: Write failing pytest tests**: `test_run_sigint_shapes` (`result.iq.dtype == np.complex64`, `len == scenario samples`); `test_run_sigint_matches_cli_hash` (manifest `output_sha256` equals the value the Rust CLI prints for the same scenario, read from `examples/scenarios/sigint_urban.json`); `test_validate_rejects_bad_json` (`pytest.raises(ValueError)`); `test_run_cuas_returns_json` (`json.loads` works, has `radar_measurement`).
- [ ] **Step 2:** `cd bindings/python && maturin develop && pytest` → FAIL (module missing). **Step 3: Implement** the `#[pymodule]`. **Step 4:** `pytest` → PASS; `cargo clippy --workspace --all-targets -- -D warnings` and `cargo deny check` pass. **Step 5:** CI job: setup-python 3.12, `pip install maturin pytest numpy`, `maturin develop`, `pytest`. **Step 6: Commit** `feat: Python bindings via PyO3`.

---

### Task 13: Release workflow — binaries, Docker image, sidecar JAR

**Files:**
- Create: `.github/workflows/release.yml`
- Modify: `Dockerfile` (OCI labels: `org.opencontainers.image.source`, `.version`, `.licenses="Apache-2.0 OR MIT"`)

**Interfaces:** none consumed or produced in code. Trigger: push of a tag matching `v*`.

- [ ] **Step 1: Write the workflow** with jobs: `binaries` (matrix `ubuntu-latest`, `macos-latest`, `windows-latest`; `cargo build --release --locked`; package `spectra-sim-<tag>-<target>.tar.gz` / `.zip` and a `.sha256`); `sidecar` (setup-java 17, `mvn -B -f orekit-sidecar/pom.xml package`, upload the shaded JAR + `.sha256`); `docker` (`docker/build-push-action`, tags `ghcr.io/${{ github.repository }}:${{ github.ref_name }}` and `latest`, `permissions: packages: write`); `release` (needs all, `softprops/action-gh-release` uploads every artifact). Top-level `permissions: contents: read`, with `contents: write` only on the `release` job.
- [ ] **Step 2: Verify locally what can be:** `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml'))"` exits 0; `docker build .` (if Docker is available) or `cargo build --release --locked` succeeds. State in the commit/PR that the workflow itself only runs on a tag and was not exercised.
- [ ] **Step 3: Commit** `ci: tag-driven release workflow with binaries, JAR and GHCR image`.

---

### Task 14: Quality gates — fuzzing, semver, coverage, contributor docs

**Files:**
- Create: `fuzz/Cargo.toml`, `fuzz/fuzz_targets/{tle,omm_json,omm_kvn,oem_kvn,ocm_xml,eop_finals,scenario_json}.rs`
- Modify: root `Cargo.toml` (`[workspace] exclude = ["fuzz"]`), `.github/workflows/ci.yml`, `CONTRIBUTING.md`

**Interfaces:**
- Consumes (parsers under test): `Tle::parse(..)` (`src/space/tle.rs:13`; read its signature before writing the target), `Omm::from_json/from_kvn/from_xml(&str)`, `Oem::from_kvn/from_xml(&str)`, `Ocm::from_kvn/from_xml(&str)`, `EopTable::from_finals2000a(&str)` / `from_c04_20u24(&str)` (confirm the exact type names in `src/space/`), `serde_json::from_slice::<Scenario>`. Each target feeds `&[u8]` (via `std::str::from_utf8` when the parser takes `&str`) and ignores the `Result` — the property is "never panics, never hangs".

- [ ] **Step 1: Write the fuzz targets** and a seed corpus (copy small valid inputs from `tests/` fixtures into `fuzz/corpus/<target>/`).
- [ ] **Step 2: Verify:** `cargo +nightly fuzz build` succeeds, then `cargo +nightly fuzz run <target> -- -max_total_time=30` for every target. Any crash found is a bug: add a regression test in `tests/regression_audit.rs` with the crashing input, fix the parser to return `Err`, re-run. (If nightly/cargo-fuzz cannot be installed offline, record which targets were not run.)
- [ ] **Step 3: CI:** add jobs `semver` (`obi1kenobi/cargo-semver-checks-action@v2`, `continue-on-error: true` until 0.5.0 is tagged, because 0.4.x→0.5.0 is a breaking-allowed bump), `coverage` (`cargo llvm-cov --all-features --fail-under-lines <N>`, where `<N>` = measured line coverage rounded down minus 2 — measure with `cargo llvm-cov --all-features --summary-only` and write the number into the workflow), and a weekly `fuzz-smoke` job (30 s per target, `schedule` only).
- [ ] **Step 4: Docs:** `CONTRIBUTING.md` gains: the full local gate (incl. `cargo deny check`, `cargo audit`), how to run fuzz/coverage/python tests, how to regenerate `schemas/`, and the release procedure (bump versions → CHANGELOG → tag `vX.Y.Z`).
- [ ] **Step 5: Commit** `ci: fuzzing, semver and coverage gates; contributor guide`.

---

### Task 15: Documentation and 0.5.0 release prep

**Files:**
- Modify: `Cargo.toml` (`version = "0.5.0"`), `Cargo.lock`, `orekit-sidecar/pom.xml`, `orekit-sidecar/README.md`, `scripts/release_check.sh`, `README.md`, `CHANGELOG.md`, `docs/SDK_INTEGRATION.md`
- Test: `tests/sdk_contracts.rs`

**Interfaces:** none produced.

- [ ] **Step 1: Write failing test** `provenance_versions_agree`: the crate version (`env!("CARGO_PKG_VERSION")`) equals the `<version>` in `orekit-sidecar/pom.xml` and appears in the JAR name used by `scripts/release_check.sh` (read the three files, assert on the version string).
- [ ] **Step 2:** run → PASS now; bump versions (`grep -rn "0\.4\.1"` to find every site; leave `docs/AUDIT_REMEDIATION_0.4.1.md` and historic CHANGELOG entries alone), re-run → still PASS, and `cargo build --locked` works after refreshing `Cargo.lock`.
- [ ] **Step 3: README:** add a "Quickstart" (install from release binary / Docker / `pip install spectra-sim`; run `examples list`, `validate`, `sigint`, `plot`, `replay`), a "Formats" table (JSON/npy/SigMF), a "Monte Carlo" section (`sweep`), a Python snippet, and a "model → validity → reference" table linking `docs/VALIDATION.md`.
- [ ] **Step 4: CHANGELOG `## 0.5.0`:** Added (CLI subcommands, schemas, formats, plots, validity report, Doppler rate, Python bindings, release workflow, fuzzing/semver/coverage), Changed (`ReplayManifest.output_schema_version`), Notes (output hashes unchanged from 0.4.1). `docs/SDK_INTEGRATION.md` documents the schema files and `output_schema_version`.
- [ ] **Step 5:** Run `scripts/release_check.sh` if its tools are installed (otherwise the individual commands it lists) and record which passed. **Step 6: Commit** `chore: prepare 0.5.0`.

---

## Self-Review

**Spec coverage** (list items → tasks): 1 CLI/clap → T2; 2 validate → T3; 3 init/list-examples → T4; 4 Rust examples + quickstart → T4, T15; 5 binaries/Docker → T13; 6 ML formats → T8 (HDF5 consciously dropped); 7 Python bindings → T12; 8 sweep → T6; 9 JSON Schema → T7; 10 schema version → T1 (`#[non_exhaustive]` intentionally not added — it would break struct-literal construction in existing tests and downstream code; `cargo-semver-checks` in T14 covers the compatibility goal); 11 replay verify → T5; 12 validity report → T10; 13 plots → T9; 14 Doppler rate → T11; 15 fuzz/semver/coverage/CONTRIBUTING → T14.

**Type consistency:** `RunKind` (T5) is reused by T6; `load_scenario`/`write_json` (T2) are used by T3–T6, T9; `IqCaptureOut`/`IqSampleOut{i,q:f32}` match `src/rf.rs`; `iq_to_npy_bytes` (T8) is consumed by T12; `output_schema_version` (T1) is the field T15 documents.

**Known open points the implementer must resolve by reading code:** exact type names of the EOP/TLE parsers (T14), where the `ValidityLedger` is built (T10), exact numeric hash captured in T1.

**Proportion:** one plan for 15 items across four areas. If a single execution session is too long, split at: A = T1–T6, B = T7–T10, C = T11, D = T12–T15; each part ends in a green gate.
