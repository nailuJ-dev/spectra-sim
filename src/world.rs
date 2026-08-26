use serde::{Deserialize, Serialize};

use crate::{Result, SimError, Vec3};

const MAX_ENTITIES: usize = 4_096;
const MAX_TEXT: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KinematicState {
    pub position_enu_m: Vec3,
    pub velocity_enu_mps: Vec3,
    pub roll_deg: f64,
    pub pitch_deg: f64,
    pub yaw_deg: f64,
}

impl KinematicState {
    pub fn validate(&self) -> Result<()> {
        self.position_enu_m.validate()?;
        self.velocity_enu_mps.validate()?;
        for value in [self.roll_deg, self.pitch_deg, self.yaw_deg] {
            if !value.is_finite() {
                return Err(SimError::NonFinite);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Antenna {
    pub gain_dbi: f64,
    pub polarization_loss_db: f64,
    pub cable_loss_db: f64,
}

impl Antenna {
    pub fn validate(&self) -> Result<()> {
        if [self.gain_dbi, self.polarization_loss_db, self.cable_loss_db]
            .into_iter()
            .all(f64::is_finite)
            && self.polarization_loss_db >= 0.0
            && self.cable_loss_db >= 0.0
        {
            Ok(())
        } else {
            Err(SimError::InvalidArgument(
                "invalid antenna parameters".into(),
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Waveform {
    ContinuousTone {
        offset_hz: f64,
    },
    PulseTrain {
        offset_hz: f64,
        pulse_repetition_hz: f64,
        duty_cycle: f64,
    },
    Ofdm {
        subcarriers: usize,
        subcarrier_spacing_hz: f64,
    },
}

impl Waveform {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::ContinuousTone { offset_hz } => finite(*offset_hz),
            Self::PulseTrain {
                offset_hz,
                pulse_repetition_hz,
                duty_cycle,
            } => {
                finite(*offset_hz)?;
                if !pulse_repetition_hz.is_finite() || *pulse_repetition_hz <= 0.0 {
                    return Err(SimError::InvalidArgument(
                        "pulse_repetition_hz must be positive".into(),
                    ));
                }
                if !duty_cycle.is_finite() || !(0.0..=1.0).contains(duty_cycle) {
                    return Err(SimError::InvalidArgument(
                        "duty_cycle must be in [0,1]".into(),
                    ));
                }
                Ok(())
            }
            Self::Ofdm {
                subcarriers,
                subcarrier_spacing_hz,
            } => {
                if *subcarriers == 0 || *subcarriers > 4_096 {
                    return Err(SimError::DimensionLimit {
                        actual: *subcarriers,
                        max: 4_096,
                    });
                }
                if !subcarrier_spacing_hz.is_finite() || *subcarrier_spacing_hz <= 0.0 {
                    return Err(SimError::InvalidArgument(
                        "subcarrier_spacing_hz must be positive".into(),
                    ));
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Emitter {
    pub id: String,
    pub center_frequency_hz: f64,
    pub bandwidth_hz: f64,
    pub tx_power_dbm: f64,
    pub antenna: Antenna,
    pub state: KinematicState,
    pub waveform: Waveform,
}

impl Emitter {
    pub fn validate(&self) -> Result<()> {
        validate_id(&self.id)?;
        positive("center_frequency_hz", self.center_frequency_hz)?;
        positive("bandwidth_hz", self.bandwidth_hz)?;
        finite(self.tx_power_dbm)?;
        self.antenna.validate()?;
        self.state.validate()?;
        self.waveform.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receiver {
    pub id: String,
    pub sample_rate_hz: f64,
    pub noise_figure_db: f64,
    pub adc_bits: u8,
    pub full_scale_v: f64,
    pub oscillator_ppm: f64,
    pub phase_noise_rad_std: f64,
    pub iq_gain_imbalance_db: f64,
    pub iq_phase_imbalance_deg: f64,
    pub agc_target_rms: f64,
    pub antenna: Antenna,
    pub state: KinematicState,
}

impl Receiver {
    pub fn validate(&self) -> Result<()> {
        validate_id(&self.id)?;
        positive("sample_rate_hz", self.sample_rate_hz)?;
        if !self.noise_figure_db.is_finite() || self.noise_figure_db < 0.0 {
            return Err(SimError::InvalidArgument(
                "noise_figure_db must be non-negative".into(),
            ));
        }
        if !(4..=24).contains(&self.adc_bits) {
            return Err(SimError::InvalidArgument(
                "adc_bits must be in 4..=24".into(),
            ));
        }
        positive("full_scale_v", self.full_scale_v)?;
        finite(self.oscillator_ppm)?;
        if !self.phase_noise_rad_std.is_finite() || self.phase_noise_rad_std < 0.0 {
            return Err(SimError::InvalidArgument(
                "phase_noise_rad_std must be non-negative".into(),
            ));
        }
        finite(self.iq_gain_imbalance_db)?;
        finite(self.iq_phase_imbalance_deg)?;
        if !self.agc_target_rms.is_finite() || !(0.01..=0.9).contains(&self.agc_target_rms) {
            return Err(SimError::InvalidArgument(
                "agc_target_rms must be in [0.01,0.9]".into(),
            ));
        }
        self.antenna.validate()?;
        self.state.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Drone,
    Bird,
    Aircraft,
    Ship,
    Vehicle,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperativeIdentity {
    pub kind: String,
    pub identity: String,
    pub source_confidence: f32,
}

impl CooperativeIdentity {
    pub fn validate(&self) -> Result<()> {
        if !matches!(self.kind.as_str(), "Ais" | "Adsb" | "RemoteId") {
            return Err(SimError::InvalidArgument(
                "cooperative kind must be Ais, Adsb or RemoteId".into(),
            ));
        }
        validate_id(&self.identity)?;
        if !self.source_confidence.is_finite() || !(0.0..=1.0).contains(&self.source_confidence) {
            return Err(SimError::InvalidArgument(
                "source_confidence must be in [0,1]".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub id: String,
    pub kind: TargetKind,
    pub state: KinematicState,
    pub rcs_m2: f64,
    pub rotor_radius_m: f64,
    pub rotor_rpm: f64,
    pub rotor_count: u16,
    pub rf_tx_power_dbm: Option<f64>,
    pub rf_center_frequency_hz: Option<f64>,
    pub rf_bandwidth_hz: Option<f64>,
    pub rf_burstiness: f32,
    pub angular_rate_dps: f32,
    pub cooperative_identity: Option<CooperativeIdentity>,
}

impl Target {
    pub fn validate(&self) -> Result<()> {
        validate_id(&self.id)?;
        self.state.validate()?;
        if !self.rcs_m2.is_finite() || self.rcs_m2 < 0.0 {
            return Err(SimError::InvalidArgument(
                "rcs_m2 must be non-negative".into(),
            ));
        }
        if !self.rotor_radius_m.is_finite() || self.rotor_radius_m < 0.0 {
            return Err(SimError::InvalidArgument(
                "rotor_radius_m must be non-negative".into(),
            ));
        }
        if !self.rotor_rpm.is_finite() || self.rotor_rpm < 0.0 {
            return Err(SimError::InvalidArgument(
                "rotor_rpm must be non-negative".into(),
            ));
        }
        match (
            self.rf_tx_power_dbm,
            self.rf_center_frequency_hz,
            self.rf_bandwidth_hz,
        ) {
            (None, None, None) => {}
            (Some(power), Some(frequency), Some(bandwidth)) => {
                finite(power)?;
                positive("rf_center_frequency_hz", frequency)?;
                positive("rf_bandwidth_hz", bandwidth)?;
            }
            _ => return Err(SimError::InvalidArgument(
                "RF emitter metadata must provide power, center frequency and bandwidth together"
                    .into(),
            )),
        }
        if !self.rf_burstiness.is_finite() || !(0.0..=1.0).contains(&self.rf_burstiness) {
            return Err(SimError::InvalidArgument(
                "rf_burstiness must be in [0,1]".into(),
            ));
        }
        if !self.angular_rate_dps.is_finite() {
            return Err(SimError::NonFinite);
        }
        if let Some(identity) = &self.cooperative_identity {
            identity.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub temperature_k: f64,
    pub ground_reflection_coefficient: f64,
    pub extra_loss_db: f64,
}

impl Environment {
    pub fn validate(&self) -> Result<()> {
        if !self.temperature_k.is_finite() || self.temperature_k <= 0.0 {
            return Err(SimError::InvalidArgument(
                "temperature_k must be positive".into(),
            ));
        }
        if !self.ground_reflection_coefficient.is_finite()
            || !(-1.0..=1.0).contains(&self.ground_reflection_coefficient)
        {
            return Err(SimError::InvalidArgument(
                "ground_reflection_coefficient must be in [-1,1]".into(),
            ));
        }
        if !self.extra_loss_db.is_finite() || self.extra_loss_db < 0.0 {
            return Err(SimError::InvalidArgument(
                "extra_loss_db must be non-negative".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PropagationModel {
    FreeSpace,
    TwoRay,
    Rician {
        k_factor_db: f64,
        shadowing_std_db: f64,
    },
}

impl PropagationModel {
    pub fn validate(&self) -> Result<()> {
        if let Self::Rician {
            k_factor_db,
            shadowing_std_db,
        } = self
        {
            finite(*k_factor_db)?;
            if !shadowing_std_db.is_finite() || *shadowing_std_db < 0.0 {
                return Err(SimError::InvalidArgument(
                    "shadowing_std_db must be non-negative".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadarConfig {
    pub carrier_frequency_hz: f64,
    pub bandwidth_hz: f64,
    pub tx_power_dbm: f64,
    pub tx_gain_dbi: f64,
    pub rx_gain_dbi: f64,
    pub noise_figure_db: f64,
    pub system_loss_db: f64,
    pub coherent_time_s: f64,
    pub array_elements: usize,
    pub element_spacing_lambda: f64,
}

impl RadarConfig {
    pub fn validate(&self) -> Result<()> {
        positive("carrier_frequency_hz", self.carrier_frequency_hz)?;
        positive("bandwidth_hz", self.bandwidth_hz)?;
        finite(self.tx_power_dbm)?;
        finite(self.tx_gain_dbi)?;
        finite(self.rx_gain_dbi)?;
        if !self.noise_figure_db.is_finite() || self.noise_figure_db < 0.0 {
            return Err(SimError::InvalidArgument(
                "noise_figure_db must be non-negative".into(),
            ));
        }
        if !self.system_loss_db.is_finite() || self.system_loss_db < 0.0 {
            return Err(SimError::InvalidArgument(
                "system_loss_db must be non-negative".into(),
            ));
        }
        positive("coherent_time_s", self.coherent_time_s)?;
        if self.array_elements == 0 || self.array_elements > 4_096 {
            return Err(SimError::DimensionLimit {
                actual: self.array_elements,
                max: 4_096,
            });
        }
        if !self.element_spacing_lambda.is_finite()
            || self.element_spacing_lambda <= 0.0
            || self.element_spacing_lambda > 4.0
        {
            return Err(SimError::InvalidArgument(
                "element_spacing_lambda must be in (0,4]".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IsacConfig {
    pub carrier_frequency_hz: f64,
    pub bandwidth_hz: f64,
    pub subcarrier_spacing_hz: f64,
    pub symbols: usize,
    pub array_elements: usize,
    pub element_spacing_lambda: f64,
    pub tx_power_dbm: f64,
    pub tx_gain_dbi: f64,
    pub rx_gain_dbi: f64,
    pub noise_figure_db: f64,
    pub system_loss_db: f64,
}

impl IsacConfig {
    pub fn validate(&self) -> Result<()> {
        positive("carrier_frequency_hz", self.carrier_frequency_hz)?;
        positive("bandwidth_hz", self.bandwidth_hz)?;
        positive("subcarrier_spacing_hz", self.subcarrier_spacing_hz)?;
        if self.symbols == 0 || self.symbols > 1_000_000 {
            return Err(SimError::DimensionLimit {
                actual: self.symbols,
                max: 1_000_000,
            });
        }
        if self.array_elements == 0 || self.array_elements > 4_096 {
            return Err(SimError::DimensionLimit {
                actual: self.array_elements,
                max: 4_096,
            });
        }
        if !self.element_spacing_lambda.is_finite()
            || self.element_spacing_lambda <= 0.0
            || self.element_spacing_lambda > 4.0
        {
            return Err(SimError::InvalidArgument(
                "element_spacing_lambda must be in (0,4]".into(),
            ));
        }
        finite(self.tx_power_dbm)?;
        finite(self.tx_gain_dbi)?;
        finite(self.rx_gain_dbi)?;
        if !self.noise_figure_db.is_finite() || self.noise_figure_db < 0.0 {
            return Err(SimError::InvalidArgument(
                "noise_figure_db must be non-negative".into(),
            ));
        }
        if !self.system_loss_db.is_finite() || self.system_loss_db < 0.0 {
            return Err(SimError::InvalidArgument(
                "system_loss_db must be non-negative".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub altitude_m: f64,
}

impl Origin {
    pub fn validate(&self) -> Result<()> {
        if !self.latitude_deg.is_finite() || !(-90.0..=90.0).contains(&self.latitude_deg) {
            return Err(SimError::InvalidArgument(
                "latitude_deg out of range".into(),
            ));
        }
        if !self.longitude_deg.is_finite() || !(-180.0..=180.0).contains(&self.longitude_deg) {
            return Err(SimError::InvalidArgument(
                "longitude_deg out of range".into(),
            ));
        }
        finite(self.altitude_m)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SigintJob {
    pub emitter_id: String,
    pub receiver_id: String,
    pub samples: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuasJob {
    pub sensor_id: String,
    pub target_id: String,
    pub radar: RadarConfig,
    pub isac: Option<IsacConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema_version: String,
    pub seed: u64,
    pub timestamp_ms: u64,
    pub origin: Origin,
    pub environment: Environment,
    pub propagation: PropagationModel,
    pub emitters: Vec<Emitter>,
    pub receivers: Vec<Receiver>,
    pub targets: Vec<Target>,
    pub sigint_job: Option<SigintJob>,
    pub cuas_job: Option<CuasJob>,
}

impl Scenario {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != "spectra-sim-scenario-v1" {
            return Err(SimError::InvalidArgument(
                "unsupported scenario schema_version".into(),
            ));
        }
        self.origin.validate()?;
        self.environment.validate()?;
        self.propagation.validate()?;
        if self.emitters.len() > MAX_ENTITIES
            || self.receivers.len() > MAX_ENTITIES
            || self.targets.len() > MAX_ENTITIES
        {
            return Err(SimError::DimensionLimit {
                actual: self
                    .emitters
                    .len()
                    .max(self.receivers.len())
                    .max(self.targets.len()),
                max: MAX_ENTITIES,
            });
        }
        for value in &self.emitters {
            value.validate()?;
        }
        for value in &self.receivers {
            value.validate()?;
        }
        for value in &self.targets {
            value.validate()?;
        }
        if let Some(job) = &self.sigint_job {
            validate_id(&job.emitter_id)?;
            validate_id(&job.receiver_id)?;
            if job.samples == 0 || job.samples > 16_384 {
                return Err(SimError::DimensionLimit {
                    actual: job.samples,
                    max: 16_384,
                });
            }
        }
        if let Some(job) = &self.cuas_job {
            validate_id(&job.sensor_id)?;
            validate_id(&job.target_id)?;
            job.radar.validate()?;
            if let Some(isac) = &job.isac {
                isac.validate()?;
            }
        }
        Ok(())
    }

    pub fn emitter(&self, id: &str) -> Result<&Emitter> {
        self.emitters
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(|| SimError::NotFound(format!("emitter {id}")))
    }
    pub fn receiver(&self, id: &str) -> Result<&Receiver> {
        self.receivers
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(|| SimError::NotFound(format!("receiver {id}")))
    }
    pub fn target(&self, id: &str) -> Result<&Target> {
        self.targets
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(|| SimError::NotFound(format!("target {id}")))
    }
}

fn validate_id(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > MAX_TEXT {
        Err(SimError::InvalidArgument(
            "identifier must be non-empty and bounded".into(),
        ))
    } else {
        Ok(())
    }
}
fn positive(name: &str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(SimError::InvalidArgument(format!(
            "{name} must be finite and positive"
        )))
    }
}
fn finite(value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(SimError::NonFinite)
    }
}
