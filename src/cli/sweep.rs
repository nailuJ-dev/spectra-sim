//! `sweep` subcommand: run one scenario over a range of seeds (Monte Carlo batches).

use std::fs;
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use spectra_sim::{run_cuas, run_sigint, DomainRandomization, Result, RunKind, Scenario, SimError};

use super::io::{
    ensure_dir, io_with_path, load_scenario, read_json, write_cuas_outputs, write_json,
    write_sigint_outputs,
};
use super::KindArg;

/// Largest number of seeds a single sweep may cover.
const MAX_SEEDS: u64 = 100_000;

/// Largest accepted `--jobs` value (each job is an OS thread).
const MAX_JOBS: usize = 1024;

/// Schema tag written into `sweep_manifest.json`.
const SWEEP_SCHEMA_VERSION: &str = "spectra-sim-sweep-v1";

/// Index of a sweep: one entry per seed, sorted by seed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SweepManifest {
    pub schema_version: String,
    pub kind: String,
    pub entries: Vec<SweepEntry>,
}

/// Result of one seed of a sweep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SweepEntry {
    pub seed: u64,
    pub output_sha256: String,
}

/// Parses a half-open seed range `A..B` with `B > A` and at most [`MAX_SEEDS`] seeds.
pub fn parse_seed_range(raw: &str) -> Result<Range<u64>> {
    let bad =
        |detail: &str| SimError::InvalidArgument(format!("--seeds must be A..B ({detail}): {raw}"));
    let (start, end) = raw
        .split_once("..")
        .ok_or_else(|| bad("expected two unsigned integers separated by '..'"))?;
    let start: u64 = start
        .parse()
        .map_err(|_| bad("start is not an unsigned integer"))?;
    let end: u64 = end
        .parse()
        .map_err(|_| bad("end is not an unsigned integer"))?;
    if end <= start {
        return Err(bad("end must be greater than start"));
    }
    if end - start > MAX_SEEDS {
        return Err(bad(&format!("at most {MAX_SEEDS} seeds per sweep")));
    }
    Ok(start..end)
}

/// Locks a mutex, ignoring poisoning (the protected data stays consistent here).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Fails unless `out` is absent, empty, or `force` is set.
fn check_out_dir(out: &Path, force: bool) -> Result<()> {
    match fs::read_dir(out) {
        Ok(mut entries) => {
            if !force && entries.next().is_some() {
                return Err(SimError::Io(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!(
                        "{}: output directory is not empty (use --force to overwrite)",
                        out.display()
                    ),
                )));
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(SimError::Io(std::io::Error::new(
            error.kind(),
            format!("{}: {error}", out.display()),
        ))),
    }
}

/// Prefixes an error with the failing seed, keeping its exit-code class.
fn with_seed_context(seed: u64, error: SimError) -> SimError {
    let prefix = format!("sweep aborted at seed {seed}");
    match error {
        SimError::Io(error) => SimError::Io(std::io::Error::new(
            error.kind(),
            format!("{prefix}: {error}"),
        )),
        SimError::NotFound(message) => SimError::NotFound(format!("{prefix}: {message}")),
        SimError::InvalidArgument(message) => {
            SimError::InvalidArgument(format!("{prefix}: {message}"))
        }
        // Every remaining variant is an input error (exit 2), like InvalidArgument.
        other => SimError::InvalidArgument(format!("{prefix}: {other}")),
    }
}

/// Runs one seed and writes its artifacts under `out/seed_<seed>/`.
fn run_seed(
    scenario: &Scenario,
    randomization: Option<&DomainRandomization>,
    kind: RunKind,
    seed: u64,
    out: &Path,
) -> Result<SweepEntry> {
    let mut scenario = match randomization {
        Some(config) => config.apply(scenario, seed)?,
        None => scenario.clone(),
    };
    scenario.seed = seed;
    let dir = out.join(format!("seed_{seed}"));
    let output_sha256 = match kind {
        RunKind::Sigint => {
            let run = run_sigint(&scenario)?;
            write_sigint_outputs(&dir, &scenario, &run)?;
            run.manifest.output_sha256
        }
        RunKind::Cuas => {
            let run = run_cuas(&scenario)?;
            write_cuas_outputs(&dir, &run)?;
            run.manifest.output_sha256
        }
        // `RunKind` is non-exhaustive: refuse kinds this binary does not know.
        _ => {
            return Err(SimError::InvalidArgument(
                "unsupported run kind for sweep".into(),
            ))
        }
    };
    Ok(SweepEntry {
        seed,
        output_sha256,
    })
}

/// Runs every seed of `seeds` on `jobs` worker threads.
///
/// Workers stop picking up seeds after the first failure; the error of the
/// lowest failing seed is returned so the result does not depend on timing.
fn run_all(
    scenario: &Scenario,
    randomization: Option<&DomainRandomization>,
    kind: RunKind,
    seeds: &Range<u64>,
    jobs: usize,
    out: &Path,
) -> Result<Vec<SweepEntry>> {
    let total = seeds.end - seeds.start;
    let next = AtomicU64::new(0);
    let failed = AtomicBool::new(false);
    let entries: Mutex<Vec<SweepEntry>> = Mutex::new(Vec::new());
    let first_error: Mutex<Option<(u64, SimError)>> = Mutex::new(None);
    let spawn_error: Mutex<Option<SimError>> = Mutex::new(None);
    // `jobs` is a usize and `total` is at most MAX_SEEDS, so this cannot truncate.
    let workers = jobs.min(usize::try_from(total).unwrap_or(usize::MAX));

    let worker = || loop {
        if failed.load(Ordering::SeqCst) {
            break;
        }
        let index = next.fetch_add(1, Ordering::SeqCst);
        if index >= total {
            break;
        }
        let seed = seeds.start + index;
        match run_seed(scenario, randomization, kind, seed, out) {
            Ok(entry) => lock(&entries).push(entry),
            Err(error) => {
                failed.store(true, Ordering::SeqCst);
                let mut slot = lock(&first_error);
                if slot.as_ref().is_none_or(|(existing, _)| seed < *existing) {
                    *slot = Some((seed, with_seed_context(seed, error)));
                }
                break;
            }
        }
    };

    std::thread::scope(|scope| {
        for number in 0..workers {
            let spawned = std::thread::Builder::new()
                .name(format!("sweep-worker-{number}"))
                .spawn_scoped(scope, worker);
            if let Err(error) = spawned {
                // Stop the workers that are already running, then report the failure.
                failed.store(true, Ordering::SeqCst);
                *lock(&spawn_error) = Some(SimError::Io(std::io::Error::new(
                    error.kind(),
                    format!("sweep aborted: cannot start worker thread {number}: {error}"),
                )));
                break;
            }
        }
    });

    if let Some((_, error)) = first_error
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner)
    {
        return Err(error);
    }
    if let Some(error) = spawn_error
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner)
    {
        return Err(error);
    }
    let mut entries = entries.into_inner().unwrap_or_else(PoisonError::into_inner);
    entries.sort_by_key(|entry| entry.seed);
    Ok(entries)
}

/// Executes the `sweep` subcommand.
pub fn run(
    scenario_path: &Path,
    kind: KindArg,
    seeds: &str,
    randomization_path: Option<&Path>,
    jobs: Option<usize>,
    out: &Path,
    force: bool,
) -> Result<()> {
    let seeds = parse_seed_range(seeds)?;
    let jobs = match jobs {
        Some(0) => {
            return Err(SimError::InvalidArgument(
                "--jobs must be at least 1".into(),
            ))
        }
        Some(jobs) if jobs > MAX_JOBS => {
            return Err(SimError::InvalidArgument(format!(
                "--jobs must be at most {MAX_JOBS}: {jobs}"
            )))
        }
        Some(jobs) => jobs,
        None => std::thread::available_parallelism().map_or(1, usize::from),
    };
    let scenario = load_scenario(scenario_path)?;
    let randomization: Option<DomainRandomization> =
        randomization_path.map(read_json).transpose()?;
    check_out_dir(out, force)?;
    ensure_dir(out)?;
    // A failed forced sweep must not leave the previous sweep's index behind.
    let manifest_path = out.join("sweep_manifest.json");
    match fs::remove_file(&manifest_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_with_path(error, &manifest_path)),
    }

    let entries = run_all(
        &scenario,
        randomization.as_ref(),
        kind.into(),
        &seeds,
        jobs,
        out,
    )?;
    let manifest = SweepManifest {
        schema_version: SWEEP_SCHEMA_VERSION.into(),
        kind: kind.as_str().into(),
        entries,
    };
    write_json(&manifest_path, &manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_ranges() {
        assert_eq!(parse_seed_range("0..3").ok(), Some(0..3));
        assert_eq!(parse_seed_range("7..8").ok(), Some(7..8));
        assert_eq!(parse_seed_range("0..100000").ok(), Some(0..100_000));
    }

    #[test]
    fn rejects_invalid_ranges() {
        for raw in [
            "5..5",
            "10..2",
            "a..b",
            "0..1000000000",
            "3",
            "",
            "..",
            "0..",
            "..3",
            "-1..3",
            "0..=3",
            "0..100001",
        ] {
            assert!(
                matches!(parse_seed_range(raw), Err(SimError::InvalidArgument(_))),
                "{raw:?} should be rejected"
            );
        }
    }
}
