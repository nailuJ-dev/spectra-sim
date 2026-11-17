//! Command-line interface for the `spectra-sim` binary.

mod examples;
mod io;
mod plot_cmd;
mod replay_cmd;
#[cfg(feature = "schema")]
mod schema_cmd;
mod sweep;
mod validate;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use spectra_sim::{run_cuas, run_sigint, DomainRandomization, Result, RunKind, SimError};

use io::{
    ensure_dir, load_scenario, read_json, write_cuas_outputs, write_iq_extras, write_json,
    write_sigint_outputs, write_validity_report,
};

#[derive(Debug, Parser)]
#[command(
    name = "spectra-sim",
    version,
    about = "Deterministic physics-based RF, radar, ISAC and space-link simulation toolkit"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Command-line spelling of [`RunKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KindArg {
    Sigint,
    Cuas,
}

impl KindArg {
    /// The spelling used on the command line and in `sweep_manifest.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            KindArg::Sigint => "sigint",
            KindArg::Cuas => "cuas",
        }
    }
}

impl From<KindArg> for RunKind {
    fn from(kind: KindArg) -> Self {
        match kind {
            KindArg::Sigint => RunKind::Sigint,
            KindArg::Cuas => RunKind::Cuas,
        }
    }
}

/// Extra I/Q interchange formats written by `sigint --also`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExtraFormat {
    /// NumPy `.npy` (`complex64`).
    Npy,
    /// SigMF `.sigmf-data` + `.sigmf-meta` (`cf32_le`).
    Sigmf,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a SIGINT scenario and write the capture, truth and manifest.
    Sigint {
        /// Path to the scenario JSON file.
        scenario: PathBuf,
        /// Output directory.
        #[arg(long, default_value = "artifacts/sigint")]
        out: PathBuf,
        /// Also write the I/Q capture as these formats (comma-separated or repeated).
        #[arg(long, value_enum, value_delimiter = ',')]
        also: Vec<ExtraFormat>,
    },
    /// Run a counter-UAS scenario and write radar/ISAC truth and manifest.
    Cuas {
        /// Path to the scenario JSON file.
        scenario: PathBuf,
        /// Output directory.
        #[arg(long, default_value = "artifacts/cuas")]
        out: PathBuf,
    },
    /// Apply domain randomization to a scenario.
    Randomize {
        /// Path to the scenario JSON file.
        scenario: PathBuf,
        /// Path to the randomization config JSON file.
        randomization: PathBuf,
        /// Seed for the randomization (defaults to the scenario seed).
        // Parsed manually so the legacy error message is preserved.
        #[arg(long, allow_hyphen_values = true)]
        seed: Option<String>,
        /// Output file.
        #[arg(long, default_value = "artifacts/randomized_scenario.json")]
        out: PathBuf,
    },
    /// Check a scenario file without running a simulation.
    Validate {
        /// Path to the scenario JSON file.
        scenario: PathBuf,
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Re-run a scenario and verify it against a stored replay manifest.
    Replay {
        /// Path to the scenario JSON file.
        scenario: PathBuf,
        /// Path to the replay_manifest.json written by a previous run.
        manifest: PathBuf,
        /// Which job the manifest was produced by.
        #[arg(long, value_enum)]
        kind: KindArg,
    },
    /// Run a scenario over a range of seeds (Monte Carlo batch).
    Sweep {
        /// Path to the scenario JSON file.
        scenario: PathBuf,
        /// Which job to run for every seed.
        #[arg(long, value_enum)]
        kind: KindArg,
        /// Half-open seed range `A..B` (at most 100000 seeds).
        // Parsed manually so range errors map to the usual exit code.
        #[arg(long, allow_hyphen_values = true)]
        seeds: String,
        /// Randomization config JSON applied per seed before the run.
        #[arg(long)]
        randomization: Option<PathBuf>,
        /// Worker threads, 1 to 1024 (default: available parallelism).
        #[arg(long)]
        jobs: Option<usize>,
        /// Output directory.
        #[arg(long, default_value = "artifacts/sweep")]
        out: PathBuf,
        /// Write into a non-empty output directory. Existing seed_* directories
        /// are kept; sweep_manifest.json is the authoritative index of a sweep.
        #[arg(long)]
        force: bool,
    },
    /// Render SVG plots (constellation, spectrum, spectrogram) of an I/Q capture.
    Plot {
        /// Path to an iq_capture.json written by `sigint`.
        capture: PathBuf,
        /// Output directory.
        #[arg(long, default_value = "artifacts/plots")]
        out: PathBuf,
        /// FFT length for the spectrum and spectrogram (power of two, 2 to 1048576).
        #[arg(long, default_value_t = 256)]
        fft: usize,
    },
    /// List or print the example scenarios embedded in this binary.
    Examples {
        #[command(subcommand)]
        action: ExamplesAction,
    },
    /// Print the JSON Schema of a scenario file or a run output.
    #[cfg(feature = "schema")]
    Schema {
        /// Which schema to print.
        #[arg(value_enum)]
        kind: schema_cmd::SchemaKind,
    },
}

#[derive(Debug, Subcommand)]
enum ExamplesAction {
    /// List the embedded example names.
    List,
    /// Print an embedded example to stdout.
    Show {
        /// Example name (see `examples list`).
        name: String,
    },
}

/// Maps an error to the process exit code.
pub fn exit_code(error: &SimError) -> i32 {
    match error {
        SimError::InvalidArgument(_)
        | SimError::NotFound(_)
        | SimError::DimensionLimit { .. }
        | SimError::NonFinite
        | SimError::Json(_) => 2,
        SimError::Io(_) => 3,
    }
}

/// Parses arguments, executes the command and returns the process exit code.
pub fn run() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        // clap prints help/version to stdout and usage errors to stderr,
        // and picks exit 0 / 2 accordingly.
        Err(error) => error.exit(),
    };
    match dispatch(cli.command) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("spectra-sim: {error}");
            exit_code(&error)
        }
    }
}

fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Sigint {
            scenario,
            out,
            also,
        } => cmd_sigint(&scenario, &out, &also),
        Command::Cuas { scenario, out } => cmd_cuas(&scenario, &out),
        Command::Randomize {
            scenario,
            randomization,
            seed,
            out,
        } => cmd_randomize(&scenario, &randomization, seed.as_deref(), &out),
        Command::Validate { scenario, json } => validate::run(&scenario, json),
        Command::Replay {
            scenario,
            manifest,
            kind,
        } => replay_cmd::run(&scenario, &manifest, kind),
        Command::Sweep {
            scenario,
            kind,
            seeds,
            randomization,
            jobs,
            out,
            force,
        } => sweep::run(
            &scenario,
            kind,
            &seeds,
            randomization.as_deref(),
            jobs,
            &out,
            force,
        ),
        Command::Plot { capture, out, fft } => plot_cmd::run(&capture, &out, fft),
        Command::Examples { action } => match action {
            ExamplesAction::List => examples::run_list(),
            ExamplesAction::Show { name } => examples::run_show(&name),
        },
        #[cfg(feature = "schema")]
        Command::Schema { kind } => schema_cmd::run(kind),
    }
}

fn cmd_sigint(scenario_path: &Path, out: &Path, also: &[ExtraFormat]) -> Result<()> {
    let scenario = load_scenario(scenario_path)?;
    let run = run_sigint(&scenario)?;
    write_sigint_outputs(out, &scenario, &run)?;
    write_validity_report(out, &scenario)?;
    write_iq_extras(out, &run.simulation.capture, also)
}

fn cmd_cuas(scenario_path: &Path, out: &Path) -> Result<()> {
    let scenario = load_scenario(scenario_path)?;
    let run = run_cuas(&scenario)?;
    write_cuas_outputs(out, &run)?;
    write_validity_report(out, &scenario)
}

fn cmd_randomize(
    scenario_path: &Path,
    randomization_path: &Path,
    seed: Option<&str>,
    out: &Path,
) -> Result<()> {
    let scenario = load_scenario(scenario_path)?;
    let config: DomainRandomization = read_json(randomization_path)?;
    let seed = match seed {
        Some(raw) => raw.parse::<u64>().map_err(|_| {
            SimError::InvalidArgument(format!("--seed must be an unsigned integer: {raw}"))
        })?,
        None => scenario.seed,
    };
    let randomized = config.apply(&scenario, seed)?;
    if let Some(parent) = out.parent() {
        ensure_dir(parent)?;
    }
    write_json(out, &randomized)
}
