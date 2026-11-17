//! Human-readable Markdown rendering of a [`ValidityLedger`].

use std::fmt::Write as _;

use crate::validity::{
    ModelMaturity, PhysicsQuality, ValidityIssue, ValidityLedger, ValiditySeverity,
};
use crate::world::{PropagationModel, Scenario};

fn quality_label(quality: PhysicsQuality) -> &'static str {
    match quality {
        PhysicsQuality::Validated => "VALIDATED",
        PhysicsQuality::ValidatedWithWarnings => "VALIDATED_WITH_WARNINGS",
        PhysicsQuality::Extrapolated => "EXTRAPOLATED",
        PhysicsQuality::Experimental => "EXPERIMENTAL",
        PhysicsQuality::Invalid => "INVALID",
    }
}

fn maturity_label(maturity: ModelMaturity) -> &'static str {
    match maturity {
        ModelMaturity::ValidatedReference => "validated_reference",
        ModelMaturity::ValidatedWithWarnings => "validated_with_warnings",
        ModelMaturity::ExperimentalResearch => "experimental_research",
    }
}

fn severity_label(severity: ValiditySeverity) -> &'static str {
    match severity {
        ValiditySeverity::Warning => "warning",
        ValiditySeverity::Extrapolation => "extrapolation",
        ValiditySeverity::Experimental => "experimental",
        ValiditySeverity::Invalid => "invalid",
    }
}

/// Escapes characters that would break a Markdown table cell.
fn escape_cell(text: &str) -> String {
    text.replace('|', "\\|").replace(['\n', '\r'], " ")
}

fn validity_range(
    frequency_hz: Option<(f64, f64)>,
    temperature_c: Option<(f64, f64)>,
    salinity_psu: Option<(f64, f64)>,
) -> String {
    let mut parts = Vec::new();
    if let Some((lo, hi)) = frequency_hz {
        parts.push(format!("{lo} Hz to {hi} Hz"));
    }
    if let Some((lo, hi)) = temperature_c {
        parts.push(format!("{lo} C to {hi} C"));
    }
    if let Some((lo, hi)) = salinity_psu {
        parts.push(format!("{lo} PSU to {hi} PSU"));
    }
    if parts.is_empty() {
        "—".to_string()
    } else {
        parts.join("; ")
    }
}

/// Renders the ledger as a Markdown validity report.
pub fn validity_report_markdown(ledger: &ValidityLedger) -> String {
    let mut out = String::from("# Physics validity report\n\n");
    let _ = writeln!(
        out,
        "Overall quality: {}\n",
        quality_label(ledger.quality())
    );
    out.push_str("| Model | Maturity | Reference | Validity range |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for model in ledger.models() {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            escape_cell(&format!("{} {}", model.name, model.version)),
            maturity_label(model.maturity),
            escape_cell(&model.reference),
            escape_cell(&validity_range(
                model.valid_frequency_hz,
                model.valid_temperature_c,
                model.valid_salinity_psu,
            )),
        );
    }
    out.push_str("\n## Issues\n\n");
    if ledger.issues().is_empty() {
        out.push_str("No issues recorded.\n");
    } else {
        for issue in ledger.issues() {
            let _ = writeln!(
                out,
                "- {} — {}: {}",
                severity_label(issue.severity),
                issue.component,
                issue.message
            );
        }
    }
    out
}

/// Builds the validity ledger implied by a scenario's selected models.
pub fn scenario_validity_ledger(scenario: &Scenario) -> ValidityLedger {
    let mut ledger = ValidityLedger::default();
    match scenario.propagation {
        PropagationModel::FreeSpace => ledger.record_model(
            "free_space",
            "1",
            "Friis transmission equation (closed form)",
            ModelMaturity::ValidatedReference,
        ),
        PropagationModel::TwoRay => {
            ledger.record_model(
                "two_ray",
                "1",
                "Two-ray ground-reflection model",
                ModelMaturity::ValidatedWithWarnings,
            );
            ledger.push(idealised_propagation_warning());
        }
        PropagationModel::Rician { .. } => {
            ledger.record_model(
                "rician",
                "1",
                "Rician fading with log-normal shadowing",
                ModelMaturity::ValidatedWithWarnings,
            );
            ledger.push(idealised_propagation_warning());
        }
    }
    ledger.push(ValidityIssue::warning(
        "dataset",
        "synthetic data: not field-validated",
    ));
    ledger
}

fn idealised_propagation_warning() -> ValidityIssue {
    ValidityIssue::warning(
        "propagation",
        "idealised statistical/geometric model; not calibrated against measurements",
    )
}
