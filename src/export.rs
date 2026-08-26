use serde::{Deserialize, Serialize};

use crate::{cuas_reference_features, free_space_path_loss_db, thermal_noise_dbm, IsacMeasurement, Origin, RadarMeasurement, Result, Scenario, SigintSimulation, SimError, Target, TargetKind, Vec3};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionGeo { pub latitude_deg: f64, pub longitude_deg: f64, pub altitude_m: f64 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VelocityNed { pub north_mps: f64, pub east_mps: f64, pub down_mps: f64 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateOut { pub timestamp_ms: u64, pub position: PositionGeo, pub velocity: VelocityNed }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CooperativeTrackOut {
    pub kind: String, pub identity: String, pub timestamp_ms: u64, pub position: PositionGeo,
    pub velocity: VelocityNed, pub source_confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceSensorMeasurementsOut {
    pub range_m: f64, pub radial_velocity_mps: f64, pub azimuth_deg: f64, pub elevation_deg: f64,
    pub snr_db: f32, pub doppler_spread_hz: f32, pub micro_doppler_index: f32, pub rcs_proxy: f32,
    pub rf_energy: f32, pub burstiness: f32, pub angular_rate_dps: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedSensorFrameOut { pub id: String, pub sensor_id: String, pub timestamp_ms: u64, pub measurements: ReferenceSensorMeasurementsOut }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum GoldenCuasSourceOut {
    Recorded { frame: RecordedSensorFrameOut },
    IsacRecorded { input: GoldenIsacRecordedInputOut },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenIsacRecordedInputOut {
    pub infrastructure_id: String, pub operator_id: String, pub timestamp_ms: u64,
    pub valid_from_ms: u64, pub valid_until_ms: u64, pub authorization_token: String,
    pub configuration_token: String, pub features: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenCuasScenarioOut {
    pub name: String, pub source: GoldenCuasSourceOut, pub candidate: Option<CandidateOut>,
    pub cooperative_tracks: Vec<CooperativeTrackOut>, pub expected_label: Option<String>,
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenSigintScenarioOut {
    pub capture: crate::IqCaptureOut,
    pub expected_label: Option<String>,
}

pub fn sigint_golden_scenario(scenario: &Scenario, capture: &crate::IqCaptureOut) -> Result<GoldenSigintScenarioOut> {
    let job = scenario.sigint_job.as_ref().ok_or_else(|| SimError::InvalidArgument("scenario has no sigint_job".into()))?;
    let emitter = scenario.emitter(&job.emitter_id)?;
    let expected_label = match &emitter.waveform {
        crate::Waveform::ContinuousTone { .. } => Some("continuous_tone".to_string()),
        crate::Waveform::PulseTrain { .. } => Some("pulsed_carrier".to_string()),
        crate::Waveform::Ofdm { .. } => None,
    };
    Ok(GoldenSigintScenarioOut { capture: capture.clone(), expected_label })
}

pub fn sigint_sdk_capture(simulation: &SigintSimulation) -> &crate::IqCaptureOut { &simulation.capture }

pub fn cuas_recorded_scenario(scenario: &Scenario, sensor_id: &str, target: &Target, measurement: &RadarMeasurement) -> Result<GoldenCuasScenarioOut> {
    let candidate = CandidateOut { timestamp_ms: scenario.timestamp_ms, position: enu_to_geo(&scenario.origin, target.state.position_enu_m)?, velocity: enu_velocity_to_ned(target.state.velocity_enu_mps) };
    let cooperative_tracks = cooperative_tracks(scenario, target)?;
    let rf_energy = rf_energy_proxy(scenario, sensor_id, target)?;
    let expected = expected_label(target.kind).to_string();
    Ok(GoldenCuasScenarioOut {
        name: format!("simulated-{expected}-{}", target.id),
        source: GoldenCuasSourceOut::Recorded { frame: RecordedSensorFrameOut {
            id: format!("sim-frame-{}-{}", sensor_id, target.id), sensor_id: sensor_id.to_string(), timestamp_ms: scenario.timestamp_ms,
            measurements: ReferenceSensorMeasurementsOut {
                range_m: measurement.range_m, radial_velocity_mps: measurement.radial_velocity_mps, azimuth_deg: measurement.azimuth_deg,
                elevation_deg: measurement.elevation_deg, snr_db: measurement.snr_db, doppler_spread_hz: measurement.doppler_spread_hz,
                micro_doppler_index: measurement.micro_doppler_index, rcs_proxy: measurement.rcs_m2.min(f32::MAX as f64) as f32,
                rf_energy, burstiness: target.rf_burstiness, angular_rate_dps: target.angular_rate_dps,
            }
        } },
        candidate: Some(candidate), cooperative_tracks, expected_label: Some(expected),
    })
}

pub fn cuas_isac_scenario(scenario: &Scenario, infrastructure_id: &str, operator_id: &str, target: &Target, measurement: &IsacMeasurement) -> Result<GoldenCuasScenarioOut> {
    let candidate = CandidateOut { timestamp_ms: scenario.timestamp_ms, position: enu_to_geo(&scenario.origin, target.state.position_enu_m)?, velocity: enu_velocity_to_ned(target.state.velocity_enu_mps) };
    let features = cuas_reference_features(&measurement.radar, rf_energy_proxy(scenario, infrastructure_id, target)?, target.rf_burstiness, target.angular_rate_dps);
    Ok(GoldenCuasScenarioOut {
        name: format!("simulated-isac-{}", target.id),
        source: GoldenCuasSourceOut::IsacRecorded { input: GoldenIsacRecordedInputOut {
            infrastructure_id: infrastructure_id.to_string(), operator_id: operator_id.to_string(), timestamp_ms: scenario.timestamp_ms,
            valid_from_ms: scenario.timestamp_ms.saturating_sub(1_000), valid_until_ms: scenario.timestamp_ms.saturating_add(1_000),
            authorization_token: format!("sim-authorization-{}", scenario.seed), configuration_token: format!("sim-config-{}", scenario.seed), features,
        } },
        candidate: Some(candidate), cooperative_tracks: cooperative_tracks(scenario, target)?, expected_label: Some(expected_label(target.kind).to_string()),
    })
}

pub fn enu_to_geo(origin: &Origin, enu_m: Vec3) -> Result<PositionGeo> {
    origin.validate()?; enu_m.validate()?;
    let lat_rad = origin.latitude_deg.to_radians();
    let latitude = origin.latitude_deg + enu_m.y / 111_320.0;
    let cos_lat = lat_rad.cos().abs().max(1e-6);
    let longitude = origin.longitude_deg + enu_m.x / (111_320.0 * cos_lat);
    Ok(PositionGeo { latitude_deg: latitude, longitude_deg: longitude, altitude_m: origin.altitude_m + enu_m.z })
}

pub fn enu_velocity_to_ned(velocity: Vec3) -> VelocityNed { VelocityNed { north_mps: velocity.y, east_mps: velocity.x, down_mps: -velocity.z } }

fn cooperative_tracks(scenario: &Scenario, target: &Target) -> Result<Vec<CooperativeTrackOut>> {
    let Some(identity) = &target.cooperative_identity else { return Ok(Vec::new()); };
    identity.validate()?;
    Ok(vec![CooperativeTrackOut { kind: identity.kind.clone(), identity: identity.identity.clone(), timestamp_ms: scenario.timestamp_ms,
        position: enu_to_geo(&scenario.origin, target.state.position_enu_m)?, velocity: enu_velocity_to_ned(target.state.velocity_enu_mps), source_confidence: identity.source_confidence }])
}

fn expected_label(kind: TargetKind) -> &'static str { match kind { TargetKind::Drone => "drone", TargetKind::Bird => "bird", TargetKind::Aircraft => "aircraft", TargetKind::Ship => "ship", TargetKind::Vehicle => "vehicle", TargetKind::Other => "background" } }
fn rf_energy_proxy(scenario: &Scenario, sensor_id: &str, target: &Target) -> Result<f32> {
    let (Some(tx_dbm), Some(frequency_hz), Some(bandwidth_hz)) = (target.rf_tx_power_dbm, target.rf_center_frequency_hz, target.rf_bandwidth_hz) else { return Ok(0.0); };
    let sensor = scenario.receiver(sensor_id)?;
    let range = sensor.state.position_enu_m.distance(target.state.position_enu_m).max(0.01);
    let path_loss = free_space_path_loss_db(frequency_hz, range)?;
    let received_dbm = tx_dbm + sensor.antenna.gain_dbi - sensor.antenna.cable_loss_db - sensor.antenna.polarization_loss_db - path_loss - scenario.environment.extra_loss_db;
    let noise_dbm = thermal_noise_dbm(bandwidth_hz.min(sensor.sample_rate_hz), scenario.environment.temperature_k, sensor.noise_figure_db)?;
    let snr_db = received_dbm - noise_dbm;
    let detectability = 1.0 / (1.0 + (-(snr_db - 3.0) / 6.0).exp());
    Ok(detectability.clamp(0.0, 1.0) as f32)
}

pub fn ensure_feature_dim(features: &[f32], expected: usize) -> Result<()> {
    if features.len() != expected { return Err(SimError::InvalidArgument(format!("expected {expected} features, got {}", features.len()))); }
    Ok(())
}
