//! Human-readable Markdown rendering of a [`ValidityLedger`].

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::validity::{
    ModelMaturity, PhysicsQuality, ValidityIssue, ValidityLedger, ValiditySeverity,
};
use crate::world::{PropagationModel, Scenario, Waveform};
use crate::AntennaPatternModel;

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
    flatten(text).replace('|', "\\|")
}

/// Maps line breaks to spaces so the text stays on a single Markdown line.
fn flatten(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
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
                flatten(&issue.component),
                flatten(&issue.message)
            );
        }
    }
    out
}

/// Builds the validity ledger implied by a scenario's selected models.
///
/// Every model that the scenario's jobs actually exercise is catalogued once:
/// the propagation model, and for a SIGINT job the emitter waveform, the
/// receiver noise/impairment chain and both antenna patterns; for a CUAS job
/// the radar equation, the target RCS model, the micro-Doppler index (when the
/// target has rotors), the ISAC resolution relations (when ISAC is enabled)
/// and every configured radar/ISAC antenna pattern. Maturity labels describe
/// the documented assumptions of each model, not a measurement campaign; the
/// always-on `dataset` warning says so.
pub fn scenario_validity_ledger(scenario: &Scenario) -> ValidityLedger {
    let mut catalog = Catalog::default();
    catalog.propagation(scenario.propagation);

    if let Some(job) = &scenario.sigint_job {
        if let Ok(emitter) = scenario.emitter(&job.emitter_id) {
            catalog.waveform(&emitter.waveform);
            catalog.antenna(&emitter.antenna.pattern);
        }
        if let Ok(receiver) = scenario.receiver(&job.receiver_id) {
            catalog.receiver_chain();
            catalog.antenna(&receiver.antenna.pattern);
        }
    }

    if let Some(job) = &scenario.cuas_job {
        catalog.model(
            "monostatic_radar_equation",
            "Classical monostatic radar equation (R^4 spreading)",
            ModelMaturity::ValidatedReference,
            None,
        );
        catalog.model(
            "rcs_constant",
            "Constant radar cross-section per target",
            ModelMaturity::ValidatedWithWarnings,
            Some(ValidityIssue::warning(
                "radar",
                "RCS is constant per target: no Swerling fluctuation and no aspect dependence",
            )),
        );
        catalog.model(
            "array_angle_resolution",
            "Uniform linear array aperture resolution, asin(lambda / aperture)",
            ModelMaturity::ValidatedReference,
            None,
        );
        catalog.antenna(&job.radar.tx_pattern);
        catalog.antenna(&job.radar.rx_pattern);
        if let Ok(target) = scenario.target(&job.target_id) {
            if target.rotor_count > 0 {
                catalog.model(
                    "micro_doppler_index",
                    "Heuristic rotor micro-Doppler index (tip-velocity span, saturating normalisation)",
                    ModelMaturity::ExperimentalResearch,
                    None,
                );
            }
        }
        if let Some(isac) = &job.isac {
            catalog.model(
                "ofdm_isac_resolution",
                "OFDM sensing relations: c/2B range resolution, c/(2 delta_f) unambiguous range",
                ModelMaturity::ValidatedReference,
                None,
            );
            catalog.antenna(&isac.tx_pattern);
            catalog.antenna(&isac.rx_pattern);
        }
    }

    let mut ledger = catalog.ledger;
    ledger.push(ValidityIssue::warning(
        "dataset",
        "synthetic data: not field-validated",
    ));
    ledger.push(ValidityIssue::warning(
        "scope",
        "models are catalogued from the scenario configuration; maturity reflects documented \
         model assumptions, not measurement campaigns; material and layered-media models carry \
         their own ledgers (MaterialProperties::validity) and are not listed here",
    ));
    ledger
}

#[derive(Default)]
struct Catalog {
    ledger: ValidityLedger,
    seen: BTreeSet<&'static str>,
}

impl Catalog {
    fn model(
        &mut self,
        name: &'static str,
        reference: &'static str,
        maturity: ModelMaturity,
        caveat: Option<ValidityIssue>,
    ) {
        if !self.seen.insert(name) {
            return;
        }
        self.ledger.record_model(name, "1", reference, maturity);
        if let Some(issue) = caveat {
            self.ledger.push(issue);
        }
    }

    fn propagation(&mut self, propagation: PropagationModel) {
        match propagation {
            PropagationModel::FreeSpace => self.model(
                "free_space",
                "Friis transmission equation (closed form)",
                ModelMaturity::ValidatedReference,
                None,
            ),
            PropagationModel::TwoRay => self.model(
                "two_ray",
                "Two-ray ground-reflection model",
                ModelMaturity::ValidatedWithWarnings,
                Some(idealised_propagation_warning()),
            ),
            PropagationModel::Rician { .. } => self.model(
                "rician",
                "Rician fading with log-normal shadowing",
                ModelMaturity::ValidatedWithWarnings,
                Some(idealised_propagation_warning()),
            ),
        }
    }

    fn waveform(&mut self, waveform: &Waveform) {
        let name = match waveform {
            Waveform::ContinuousTone { .. } => "waveform_continuous_tone",
            Waveform::PulseTrain { .. } => "waveform_pulse_train",
            Waveform::Ofdm { .. } => "waveform_ofdm",
        };
        self.model(
            name,
            "Analytic complex-baseband synthesis",
            ModelMaturity::ValidatedReference,
            None,
        );
    }

    fn receiver_chain(&mut self) {
        self.model(
            "thermal_noise_ktb",
            "kTB thermal noise with receiver noise figure",
            ModelMaturity::ValidatedReference,
            None,
        );
        self.model(
            "receiver_impairments",
            "Behavioural CFO (oscillator ppm), random-walk phase noise, IQ gain/phase imbalance, \
             RMS-targeted AGC and uniform ADC quantisation",
            ModelMaturity::ValidatedWithWarnings,
            Some(ValidityIssue::warning(
                "receiver",
                "behavioural impairment models; not calibrated against measured hardware",
            )),
        );
    }

    fn antenna(&mut self, pattern: &AntennaPatternModel) {
        match pattern {
            AntennaPatternModel::Isotropic => self.model(
                "antenna_isotropic",
                "Isotropic radiator (boresight gain applied in every direction)",
                ModelMaturity::ValidatedReference,
                None,
            ),
            AntennaPatternModel::Gaussian { .. } => self.model(
                "antenna_gaussian",
                "Gaussian main-lobe approximation with a gain floor",
                ModelMaturity::ValidatedWithWarnings,
                Some(main_lobe_warning()),
            ),
            AntennaPatternModel::EllipticalGaussian { .. } => self.model(
                "antenna_elliptical_gaussian",
                "Elliptical Gaussian main-lobe approximation with a gain floor",
                ModelMaturity::ValidatedWithWarnings,
                Some(main_lobe_warning()),
            ),
            AntennaPatternModel::TabulatedRelative { .. } => self.model(
                "antenna_tabulated",
                "User-supplied relative gain table interpolated in off-axis angle",
                ModelMaturity::ValidatedWithWarnings,
                Some(ValidityIssue::warning(
                    "antenna",
                    "pattern accuracy is limited by the supplied samples",
                )),
            ),
        }
    }
}

fn idealised_propagation_warning() -> ValidityIssue {
    ValidityIssue::warning(
        "propagation",
        "idealised statistical/geometric model; not calibrated against measurements",
    )
}

fn main_lobe_warning() -> ValidityIssue {
    ValidityIssue::warning(
        "antenna",
        "analytic main-lobe approximation; sidelobe structure is replaced by a gain floor",
    )
}
