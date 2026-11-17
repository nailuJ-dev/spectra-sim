//! `schema` subcommand: print the JSON Schema of a scenario or an in-memory run result.

use clap::ValueEnum;
use spectra_sim::{cuas_run_schema, scenario_schema, sigint_run_schema, Result};

/// Which schema to print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SchemaKind {
    /// Scenario input file.
    Scenario,
    /// In-memory SIGINT run result (simulation + manifest), not a file the CLI writes.
    SigintRun,
    /// In-memory counter-UAS run result (simulation + manifest), not a file the CLI writes.
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
