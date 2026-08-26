use serde::{Deserialize, Serialize};

use crate::{DeterministicRng, Result, Scenario, SimError};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomizationRange {
    pub min: f64,
    pub max: f64,
}

impl RandomizationRange {
    pub fn validate(&self) -> Result<()> {
        if self.min.is_finite() && self.max.is_finite() && self.max >= self.min {
            Ok(())
        } else {
            Err(SimError::InvalidArgument(
                "invalid randomization range".into(),
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainRandomization {
    pub tx_power_delta_db: RandomizationRange,
    pub receiver_noise_figure_delta_db: RandomizationRange,
    pub position_jitter_m: RandomizationRange,
    pub velocity_jitter_mps: RandomizationRange,
    pub rcs_scale: RandomizationRange,
    pub temperature_delta_k: RandomizationRange,
}

impl DomainRandomization {
    pub fn validate(&self) -> Result<()> {
        self.tx_power_delta_db.validate()?;
        self.receiver_noise_figure_delta_db.validate()?;
        self.position_jitter_m.validate()?;
        self.velocity_jitter_mps.validate()?;
        self.rcs_scale.validate()?;
        self.temperature_delta_k.validate()
    }

    pub fn apply(&self, scenario: &Scenario, seed: u64) -> Result<Scenario> {
        self.validate()?;
        scenario.validate()?;
        let mut rng = DeterministicRng::new(seed);
        let mut out = scenario.clone();
        for emitter in &mut out.emitters {
            emitter.tx_power_dbm +=
                rng.uniform(self.tx_power_delta_db.min, self.tx_power_delta_db.max)?;
            jitter_state(&mut emitter.state, &mut rng, self)?;
        }
        for receiver in &mut out.receivers {
            receiver.noise_figure_db = (receiver.noise_figure_db
                + rng.uniform(
                    self.receiver_noise_figure_delta_db.min,
                    self.receiver_noise_figure_delta_db.max,
                )?)
            .max(0.0);
            jitter_state(&mut receiver.state, &mut rng, self)?;
        }
        for target in &mut out.targets {
            jitter_state(&mut target.state, &mut rng, self)?;
            target.rcs_m2 =
                (target.rcs_m2 * rng.uniform(self.rcs_scale.min, self.rcs_scale.max)?).max(0.0);
        }
        out.environment.temperature_k = (out.environment.temperature_k
            + rng.uniform(self.temperature_delta_k.min, self.temperature_delta_k.max)?)
        .max(1.0);
        out.seed = seed;
        out.validate()?;
        Ok(out)
    }
}

fn jitter_state(
    state: &mut crate::KinematicState,
    rng: &mut DeterministicRng,
    cfg: &DomainRandomization,
) -> Result<()> {
    state.position_enu_m.x += rng.uniform(cfg.position_jitter_m.min, cfg.position_jitter_m.max)?;
    state.position_enu_m.y += rng.uniform(cfg.position_jitter_m.min, cfg.position_jitter_m.max)?;
    state.position_enu_m.z += rng.uniform(cfg.position_jitter_m.min, cfg.position_jitter_m.max)?;
    state.velocity_enu_mps.x +=
        rng.uniform(cfg.velocity_jitter_mps.min, cfg.velocity_jitter_mps.max)?;
    state.velocity_enu_mps.y +=
        rng.uniform(cfg.velocity_jitter_mps.min, cfg.velocity_jitter_mps.max)?;
    state.velocity_enu_mps.z +=
        rng.uniform(cfg.velocity_jitter_mps.min, cfg.velocity_jitter_mps.max)?;
    Ok(())
}
