use crate::{Result, SimError};

#[derive(Debug, Clone, Copy)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn unit_f64(&mut self) -> f64 {
        let value = self.next_u64() >> 11;
        value as f64 / ((1_u64 << 53) as f64)
    }

    pub fn uniform(&mut self, min: f64, max: f64) -> Result<f64> {
        if !min.is_finite() || !max.is_finite() || max < min {
            return Err(SimError::InvalidArgument("invalid uniform range".into()));
        }
        Ok(min + (max - min) * self.unit_f64())
    }

    pub fn normal(&mut self, mean: f64, stddev: f64) -> Result<f64> {
        if !mean.is_finite() || !stddev.is_finite() || stddev < 0.0 {
            return Err(SimError::InvalidArgument(
                "invalid normal distribution parameters".into(),
            ));
        }
        if stddev == 0.0 {
            return Ok(mean);
        }
        let u1 = self.unit_f64().max(f64::MIN_POSITIVE);
        let u2 = self.unit_f64();
        let radius = (-2.0 * u1.ln()).sqrt();
        let z = radius * (std::f64::consts::TAU * u2).cos();
        Ok(mean + stddev * z)
    }
}
