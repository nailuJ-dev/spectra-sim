//! `schema` subcommand: print the JSON Schema of a scenario or run output.

use clap::ValueEnum;
use spectra_sim::{cuas_run_schema, scenario_schema, sigint_run_schema, Result};

/// Which schema to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SchemaKind {
    /// Scenario input file.
    Scenario,
    /// Result of a SIGINT run.
    SigintRun,
    /// Result of a counter-UAS run.
    CuasRun,
}

pub fn run(kind: SchemaKind) -> Result<()> {
    let schema = match kind {
        SchemaKind::Scenario => scenario_schema(),
        SchemaKind::SigintRun => sigint_run_schema(),
        SchemaKind::CuasRun => cuas_run_schema(),
    };
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
