use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

use super::{AbsoluteEpoch, TimeScale};

pub const ARCSEC_TO_RAD: f64 = std::f64::consts::PI / (180.0 * 3_600.0);
pub const MAS_TO_RAD: f64 = ARCSEC_TO_RAD / 1_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EopSample {
    pub mjd_utc: f64,
    pub polar_motion_x_rad: f64,
    pub polar_motion_y_rad: f64,
    pub ut1_minus_utc_s: f64,
    pub lod_s: f64,
    pub dx_rad: f64,
    pub dy_rad: f64,
}

impl EopSample {
    pub fn validate(&self) -> Result<()> {
        if [
            self.mjd_utc,
            self.polar_motion_x_rad,
            self.polar_motion_y_rad,
            self.ut1_minus_utc_s,
            self.lod_s,
            self.dx_rad,
            self.dy_rad,
        ]
        .iter()
        .any(|v| !v.is_finite())
        {
            return Err(SimError::NonFinite);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EopTable {
    samples: Vec<EopSample>,
    source_label: String,
}

impl EopTable {
    pub fn new(mut samples: Vec<EopSample>, source_label: impl Into<String>) -> Result<Self> {
        if samples.len() < 2 {
            return Err(SimError::InvalidArgument(
                "EOP table requires at least two samples".into(),
            ));
        }
        for sample in &samples {
            sample.validate()?;
        }
        samples.sort_by(|a, b| a.mjd_utc.total_cmp(&b.mjd_utc));
        if samples.windows(2).any(|p| p[0].mjd_utc >= p[1].mjd_utc) {
            return Err(SimError::InvalidArgument(
                "EOP sample MJDs must be strictly increasing".into(),
            ));
        }
        Ok(Self {
            samples,
            source_label: source_label.into(),
        })
    }

    pub fn samples(&self) -> &[EopSample] {
        &self.samples
    }

    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    pub fn sample_at_mjd(&self, mjd_utc: f64) -> Result<EopSample> {
        if !mjd_utc.is_finite() {
            return Err(SimError::NonFinite);
        }
        let first = self
            .samples
            .first()
            .ok_or_else(|| SimError::InvalidArgument("empty EOP table".into()))?;
        let last = self
            .samples
            .last()
            .ok_or_else(|| SimError::InvalidArgument("empty EOP table".into()))?;
        if mjd_utc < first.mjd_utc || mjd_utc > last.mjd_utc {
            return Err(SimError::InvalidArgument(format!(
                "EOP epoch {mjd_utc} is outside [{}, {}]",
                first.mjd_utc, last.mjd_utc
            )));
        }
        let upper = self
            .samples
            .partition_point(|sample| sample.mjd_utc < mjd_utc);
        if upper < self.samples.len()
            && self.samples[upper].mjd_utc.total_cmp(&mjd_utc) == Ordering::Equal
        {
            return Ok(self.samples[upper]);
        }
        let b = self.samples[upper.min(self.samples.len() - 1)];
        let a = self.samples[upper.saturating_sub(1)];
        let t = (mjd_utc - a.mjd_utc) / (b.mjd_utc - a.mjd_utc);
        Ok(EopSample {
            mjd_utc,
            polar_motion_x_rad: lerp(a.polar_motion_x_rad, b.polar_motion_x_rad, t),
            polar_motion_y_rad: lerp(a.polar_motion_y_rad, b.polar_motion_y_rad, t),
            ut1_minus_utc_s: lerp(a.ut1_minus_utc_s, b.ut1_minus_utc_s, t),
            lod_s: lerp(a.lod_s, b.lod_s, t),
            dx_rad: lerp(a.dx_rad, b.dx_rad, t),
            dy_rad: lerp(a.dy_rad, b.dy_rad, t),
        })
    }

    pub fn sample_at(&self, epoch: AbsoluteEpoch) -> Result<EopSample> {
        let (utc1, utc2) = epoch.utc_parts(None)?;
        self.sample_at_mjd(utc1 + utc2 - 2_400_000.5)
    }

    /// Parses the IERS 20u24 C04 whitespace format:
    /// `YR MM DD HH MJD x y UT1-UTC dX dY xrt yrt LOD ...`.
    pub fn from_c04_20u24(input: &str) -> Result<Self> {
        let mut samples = Vec::new();
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with('#')
                || !line.chars().next().is_some_and(|c| c.is_ascii_digit())
            {
                continue;
            }
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 13 {
                continue;
            }
            let parse = |index: usize, label: &str| -> Result<f64> {
                cols[index]
                    .parse::<f64>()
                    .map_err(|_| SimError::InvalidArgument(format!("invalid C04 {label} field")))
            };
            samples.push(EopSample {
                mjd_utc: parse(4, "MJD")?,
                polar_motion_x_rad: parse(5, "x")? * ARCSEC_TO_RAD,
                polar_motion_y_rad: parse(6, "y")? * ARCSEC_TO_RAD,
                ut1_minus_utc_s: parse(7, "UT1-UTC")?,
                dx_rad: parse(8, "dX")? * ARCSEC_TO_RAD,
                dy_rad: parse(9, "dY")? * ARCSEC_TO_RAD,
                lod_s: parse(12, "LOD")?,
            });
        }
        Self::new(samples, "IERS EOP 20u24 C04")
    }

    /// Parses the fixed-width USNO/IERS `finals2000A.*` format documented by
    /// `readme.finals2000A`, using Bulletin A fields. Incomplete prediction-tail
    /// rows are skipped rather than zero-filled; present-but-invalid numeric
    /// fields remain hard errors.
    pub fn from_finals2000a(input: &str) -> Result<Self> {
        let mut samples = Vec::new();
        let mut skipped = 0usize;
        for line in input.lines() {
            if line.len() < 68 {
                continue;
            }
            let fields = [
                parse_slice_optional(line, 7, 15)?,
                parse_slice_optional(line, 18, 27)?,
                parse_slice_optional(line, 37, 46)?,
                parse_slice_optional(line, 58, 68)?,
                parse_slice_optional(line, 79, 86)?,
                parse_slice_optional(line, 97, 106)?,
                parse_slice_optional(line, 116, 125)?,
            ];
            let Some([mjd, x, y, dut1, lod_ms, dx_mas, dy_mas]) = all_present(fields) else {
                skipped += 1;
                continue;
            };
            samples.push(EopSample {
                mjd_utc: mjd,
                polar_motion_x_rad: x * ARCSEC_TO_RAD,
                polar_motion_y_rad: y * ARCSEC_TO_RAD,
                ut1_minus_utc_s: dut1,
                lod_s: lod_ms * 1.0e-3,
                dx_rad: dx_mas * MAS_TO_RAD,
                dy_rad: dy_mas * MAS_TO_RAD,
            });
        }
        if samples.len() < 2 {
            return Err(SimError::InvalidArgument(format!(
                "finals2000A contains fewer than two complete rows ({skipped} incomplete rows skipped)"
            )));
        }
        Self::new(samples, "IERS/USNO finals2000A")
    }
}

pub fn epoch_from_mjd_utc(mjd: f64) -> Result<AbsoluteEpoch> {
    AbsoluteEpoch::new(2_400_000.5, mjd, TimeScale::Utc)
}

fn all_present(fields: [Option<f64>; 7]) -> Option<[f64; 7]> {
    let mut output = [0.0_f64; 7];
    for (slot, field) in output.iter_mut().zip(fields) {
        *slot = field?;
    }
    Some(output)
}

fn parse_slice_optional(line: &str, start: usize, end: usize) -> Result<Option<f64>> {
    let raw = line.get(start..end).unwrap_or("").trim();
    if raw.is_empty() {
        return Ok(None);
    }
    raw.parse::<f64>().map(Some).map_err(|_| {
        SimError::InvalidArgument(format!("invalid finals2000A numeric field {start}..{end}"))
    })
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
