use serde::{Deserialize, Serialize};

use crate::{
    cuas_isac_scenario, cuas_recorded_scenario, replay_manifest, simulate_iq,
    simulate_isac_with_propagation, simulate_monostatic_radar_with_propagation, DeterministicRng,
    GoldenCuasScenarioOut, IsacMeasurement, RadarMeasurement, ReplayManifest, Result, Scenario,
    SigintSimulation, SimError,
};

const RADAR_RNG_DOMAIN: u64 = 0x5241_4441_525F_5631;
const ISAC_RNG_DOMAIN: u64 = 0x4953_4143_5F56_3101;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SigintRun {
    pub simulation: SigintSimulation,
    pub manifest: ReplayManifest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuasRun {
    pub radar_measurement: RadarMeasurement,
    pub recorded_scenario: GoldenCuasScenarioOut,
    pub isac_measurement: Option<IsacMeasurement>,
    pub isac_scenario: Option<GoldenCuasScenarioOut>,
    pub manifest: ReplayManifest,
}

pub fn run_sigint(scenario: &Scenario) -> Result<SigintRun> {
    scenario.validate()?;
    let job = scenario
        .sigint_job
        .as_ref()
        .ok_or_else(|| SimError::InvalidArgument("scenario has no sigint_job".into()))?;
    let emitter = scenario.emitter(&job.emitter_id)?;
    let receiver = scenario.receiver(&job.receiver_id)?;
    let simulation = simulate_iq(
        emitter,
        receiver,
        scenario.environment,
        scenario.propagation,
        scenario.timestamp_ms,
        job.samples,
        scenario.seed,
    )?;
    let manifest = replay_manifest(scenario, &simulation)?;
    Ok(SigintRun {
        simulation,
        manifest,
    })
}

pub fn run_cuas(scenario: &Scenario) -> Result<CuasRun> {
    scenario.validate()?;
    let job = scenario
        .cuas_job
        .as_ref()
        .ok_or_else(|| SimError::InvalidArgument("scenario has no cuas_job".into()))?;
    let sensor = scenario.receiver(&job.sensor_id)?;
    let target = scenario.target(&job.target_id)?;

    let mut radar_rng = DeterministicRng::new(scenario.seed ^ RADAR_RNG_DOMAIN);
    let radar_measurement = simulate_monostatic_radar_with_propagation(
        sensor.state,
        target,
        &job.radar,
        scenario.environment,
        scenario.propagation,
        &mut radar_rng,
    )?;
    let recorded_scenario =
        cuas_recorded_scenario(scenario, &sensor.id, target, &radar_measurement)?;

    let (isac_measurement, isac_scenario) = if let Some(config) = &job.isac {
        let mut isac_rng = DeterministicRng::new(scenario.seed ^ ISAC_RNG_DOMAIN);
        let measurement = simulate_isac_with_propagation(
            sensor.state,
            target,
            config,
            scenario.environment,
            scenario.propagation,
            &mut isac_rng,
        )?;
        let out = cuas_isac_scenario(
            scenario,
            &sensor.id,
            "simulated-operator",
            target,
            &measurement,
        )?;
        (Some(measurement), Some(out))
    } else {
        (None, None)
    };

    let material = (
        &radar_measurement,
        &recorded_scenario,
        &isac_measurement,
        &isac_scenario,
    );
    let manifest = replay_manifest(scenario, &material)?;
    Ok(CuasRun {
        radar_measurement,
        recorded_scenario,
        isac_measurement,
        isac_scenario,
        manifest,
    })
}
