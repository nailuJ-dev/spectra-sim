use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelMaturity {
    ValidatedReference,
    ValidatedWithWarnings,
    ExperimentalResearch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicsQuality {
    Validated,
    ValidatedWithWarnings,
    Extrapolated,
    Experimental,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValiditySeverity {
    Warning,
    Extrapolation,
    Experimental,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidityIssue {
    pub component: String,
    pub message: String,
    pub severity: ValiditySeverity,
}

impl ValidityIssue {
    pub fn warning(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self { component: component.into(), message: message.into(), severity: ValiditySeverity::Warning }
    }

    pub fn extrapolation(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self { component: component.into(), message: message.into(), severity: ValiditySeverity::Extrapolation }
    }

    pub fn experimental(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self { component: component.into(), message: message.into(), severity: ValiditySeverity::Experimental }
    }

    pub fn invalid(component: impl Into<String>, message: impl Into<String>) -> Self {
        Self { component: component.into(), message: message.into(), severity: ValiditySeverity::Invalid }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub name: String,
    pub version: String,
    pub reference: String,
    pub maturity: ModelMaturity,
    pub valid_frequency_hz: Option<(f64, f64)>,
    pub valid_temperature_c: Option<(f64, f64)>,
    pub valid_salinity_psu: Option<(f64, f64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ValidityLedger {
    models: Vec<ModelMetadata>,
    issues: Vec<ValidityIssue>,
}

impl ValidityLedger {
    pub fn models(&self) -> &[ModelMetadata] { &self.models }
    pub fn issues(&self) -> &[ValidityIssue] { &self.issues }

    pub fn push(&mut self, issue: ValidityIssue) { self.issues.push(issue); }

    pub fn record_model(
        &mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        reference: impl Into<String>,
        maturity: ModelMaturity,
    ) {
        if maturity == ModelMaturity::ExperimentalResearch {
            self.issues.push(ValidityIssue::experimental("model", "experimental research model enabled"));
        } else if maturity == ModelMaturity::ValidatedWithWarnings {
            self.issues.push(ValidityIssue::warning("model", "model enabled with documented caveats"));
        }
        self.models.push(ModelMetadata {
            name: name.into(), version: version.into(), reference: reference.into(), maturity,
            valid_frequency_hz: None, valid_temperature_c: None, valid_salinity_psu: None,
        });
    }

    pub fn quality(&self) -> PhysicsQuality {
        self.issues.iter().fold(PhysicsQuality::Validated, |quality, issue| {
            let next = match issue.severity {
                ValiditySeverity::Warning => PhysicsQuality::ValidatedWithWarnings,
                ValiditySeverity::Extrapolation => PhysicsQuality::Extrapolated,
                ValiditySeverity::Experimental => PhysicsQuality::Experimental,
                ValiditySeverity::Invalid => PhysicsQuality::Invalid,
            };
            quality.max(next)
        })
    }
}
