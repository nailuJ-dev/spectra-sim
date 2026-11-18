//! Ground-station pass prediction from a TLE.
//!
//! [`compute_pass`] propagates a TLE with SGP4, rotates each state from TEME
//! to ITRF with IERS Earth-orientation parameters, and samples the
//! station-relative azimuth, elevation, slant range, range rate, one-way
//! Doppler and Doppler rate on a fixed time grid.
//!
//! The geometry is **light-time corrected** and computed in the SGP4 output
//! frame TEME, which is quasi-inertial over the few milliseconds of signal
//! flight (its precession/nutation drift over 0.1 s is below 1e-12 rad): for
//! each receive epoch `t` the transmit epoch `t - tau` is found by
//! iterating `tau = |r_station(t) - r_sat(t - tau)| / c`, which also accounts
//! for the Earth's rotation during the signal flight (Sagnac). The range rate
//! is `(v_station(t) - v_sat(t - tau)) . u`, `u` being the unit vector from the
//! satellite to the station; the one-way Doppler is `-range_rate * f / c`
//! (first order, no relativistic terms), the same convention as
//! [`crate::solve_absolute_one_way_link`]. Azimuth and elevation are the
//! apparent (light-time corrected) direction in the station frame.

use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

use super::{
    ground_station_az_el_range, transform_absolute_state, AbsoluteEpoch, AbsoluteOrbitState,
    EopTable, GeodeticPosition, GroundStation, ReferenceFrame, Sgp4Propagator, TimeScale, Tle,
    SPEED_OF_LIGHT_M_PER_S,
};

/// Schema identifier written into every [`PassResult`].
pub const PASS_SCHEMA_VERSION: &str = "spectra-sim-pass-v1";

/// Upper bound on the number of time samples of one pass computation.
pub const MAX_PASS_SAMPLES: usize = 100_000;

/// Half-step (seconds) of the central difference used for the Doppler rate.
const DOPPLER_RATE_HALF_STEP_S: f64 = 0.5;

/// Two-line element set of the satellite.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassTleInput {
    #[serde(default)]
    pub object_name: Option<String>,
    pub line1: String,
    pub line2: String,
}

/// Geodetic (WGS-84) location of the ground station.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassStationInput {
    pub name: String,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub height_m: f64,
    /// Elevation mask: samples below it are reported as not visible.
    #[serde(default)]
    pub minimum_elevation_deg: f64,
}

/// Format of the Earth-orientation-parameter file.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EopFormat {
    /// USNO/IERS `finals2000A.*` fixed-width Bulletin A files.
    #[serde(rename = "finals2000a")]
    Finals2000a,
    /// IERS EOP 20u24 C04 whitespace files.
    #[serde(rename = "c04_20u24")]
    C04_20u24,
}

/// Reference to the EOP file the caller must provide; nothing is downloaded.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassEopInput {
    pub format: EopFormat,
    /// Path to the EOP file; relative paths are resolved against the config file.
    pub path: String,
}

/// Input of a pass computation (`spectra-sim space-pass <config.json>`).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassConfig {
    pub tle: PassTleInput,
    pub ground_station: PassStationInput,
    /// UTC start epoch, `YYYY-MM-DDThh:mm:ss[.fff][Z]`.
    pub start_utc: String,
    pub duration_s: f64,
    pub step_s: f64,
    pub carrier_frequency_hz: f64,
    pub eop: PassEopInput,
}

/// One time sample of the pass.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassSample {
    /// Seconds since `start_utc` (SI seconds).
    pub t_s: f64,
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub range_m: f64,
    pub range_rate_m_per_s: f64,
    pub doppler_hz: f64,
    pub doppler_rate_hz_per_s: f64,
    /// One-way signal flight time from the satellite to the station.
    pub light_time_s: f64,
    pub visible: bool,
}

/// A contiguous run of visible samples.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassWindow {
    /// Time of the first visible sample (sample resolution, not interpolated).
    pub start_s: f64,
    /// Time of the last visible sample.
    pub end_s: f64,
    pub max_elevation_deg: f64,
    pub max_elevation_t_s: f64,
}

/// Output of [`compute_pass`].
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassResult {
    pub schema_version: String,
    pub object_name: Option<String>,
    pub station: String,
    pub start_utc: String,
    pub step_s: f64,
    pub carrier_frequency_hz: f64,
    pub eop_source: String,
    /// Always `"light_time_corrected"`, see module docs.
    pub geometry: String,
    pub samples: Vec<PassSample>,
    pub windows: Vec<PassWindow>,
}

/// Parses an EOP file in the given format.
pub fn parse_eop(format: EopFormat, text: &str) -> Result<EopTable> {
    match format {
        EopFormat::Finals2000a => EopTable::from_finals2000a(text),
        EopFormat::C04_20u24 => EopTable::from_c04_20u24(text),
    }
}

/// Number of samples on the `[0, duration_s]` grid with spacing `step_s`.
pub fn pass_sample_count(duration_s: f64, step_s: f64) -> Result<usize> {
    if !duration_s.is_finite() || duration_s <= 0.0 {
        return Err(SimError::InvalidArgument(
            "duration_s must be positive and finite".into(),
        ));
    }
    if !step_s.is_finite() || step_s <= 0.0 {
        return Err(SimError::InvalidArgument(
            "step_s must be positive and finite".into(),
        ));
    }
    let intervals = (duration_s / step_s).floor();
    if intervals >= MAX_PASS_SAMPLES as f64 {
        return Err(SimError::DimensionLimit {
            actual: usize::MAX.min(intervals as usize).saturating_add(1),
            max: MAX_PASS_SAMPLES,
        });
    }
    Ok(intervals as usize + 1)
}

/// Computes the station-relative pass geometry of a TLE on a fixed time grid.
pub fn compute_pass(config: &PassConfig, eop: &EopTable) -> Result<PassResult> {
    let count = pass_sample_count(config.duration_s, config.step_s)?;
    if !config.carrier_frequency_hz.is_finite() || config.carrier_frequency_hz <= 0.0 {
        return Err(SimError::InvalidArgument(
            "carrier_frequency_hz must be positive and finite".into(),
        ));
    }
    let station = build_station(&config.ground_station)?;
    let tle = Tle::parse(
        config.tle.object_name.clone(),
        &config.tle.line1,
        &config.tle.line2,
    )?;
    let propagator = Sgp4Propagator::from_tle(&tle)?;
    let start = AbsoluteEpoch::parse_ccsds(&config.start_utc, TimeScale::Utc)?;
    let f = config.carrier_frequency_hz;

    let mut samples = Vec::with_capacity(count);
    for k in 0..count {
        let t_s = k as f64 * config.step_s;
        let epoch = start.shift_si_seconds(t_s, None)?;
        let now = geometry(&propagator, &station, eop, epoch)?;
        let before = geometry(
            &propagator,
            &station,
            eop,
            epoch.shift_si_seconds(-DOPPLER_RATE_HALF_STEP_S, None)?,
        )?;
        let after = geometry(
            &propagator,
            &station,
            eop,
            epoch.shift_si_seconds(DOPPLER_RATE_HALF_STEP_S, None)?,
        )?;
        let doppler = doppler_hz(now.range_rate_m_per_s, f);
        let doppler_rate = (doppler_hz(after.range_rate_m_per_s, f)
            - doppler_hz(before.range_rate_m_per_s, f))
            / (2.0 * DOPPLER_RATE_HALF_STEP_S);
        samples.push(PassSample {
            t_s,
            azimuth_deg: now.azimuth_rad.to_degrees(),
            elevation_deg: now.elevation_rad.to_degrees(),
            range_m: now.range_m,
            range_rate_m_per_s: now.range_rate_m_per_s,
            doppler_hz: doppler,
            doppler_rate_hz_per_s: doppler_rate,
            light_time_s: now.light_time_s,
            visible: now.visible,
        });
    }
    let windows = visibility_windows(&samples);
    Ok(PassResult {
        schema_version: PASS_SCHEMA_VERSION.into(),
        object_name: config.tle.object_name.clone(),
        station: station.name.clone(),
        start_utc: config.start_utc.clone(),
        step_s: config.step_s,
        carrier_frequency_hz: f,
        eop_source: eop.source_label().into(),
        geometry: "light_time_corrected".into(),
        samples,
        windows,
    })
}

/// Groups contiguous visible samples into windows.
pub fn visibility_windows(samples: &[PassSample]) -> Vec<PassWindow> {
    let mut windows = Vec::new();
    let mut current: Option<PassWindow> = None;
    for sample in samples {
        match (&mut current, sample.visible) {
            (Some(window), true) => {
                window.end_s = sample.t_s;
                if sample.elevation_deg > window.max_elevation_deg {
                    window.max_elevation_deg = sample.elevation_deg;
                    window.max_elevation_t_s = sample.t_s;
                }
            }
            (None, true) => {
                current = Some(PassWindow {
                    start_s: sample.t_s,
                    end_s: sample.t_s,
                    max_elevation_deg: sample.elevation_deg,
                    max_elevation_t_s: sample.t_s,
                });
            }
            (Some(_), false) => {
                if let Some(window) = current.take() {
                    windows.push(window);
                }
            }
            (None, false) => {}
        }
    }
    if let Some(window) = current {
        windows.push(window);
    }
    windows
}

struct Geometry {
    azimuth_rad: f64,
    elevation_rad: f64,
    range_m: f64,
    range_rate_m_per_s: f64,
    light_time_s: f64,
    visible: bool,
}

/// Convergence tolerance of the light-time iteration (seconds).
const LIGHT_TIME_TOLERANCE_S: f64 = 1e-12;
/// Iteration cap of the light-time solution; LEO to GEO converge in < 6.
const LIGHT_TIME_MAX_ITERATIONS: usize = 20;

fn geometry(
    propagator: &Sgp4Propagator,
    station: &GroundStation,
    eop: &EopTable,
    receive_epoch: AbsoluteEpoch,
) -> Result<Geometry> {
    let rx_eop = eop.sample_at(receive_epoch)?;
    let station_itrf = AbsoluteOrbitState::new(
        receive_epoch,
        ReferenceFrame::Itrf,
        station.ecef_m(),
        [0.0; 3],
    )?;
    // Station state in TEME at receive time, including the Earth-rotation velocity.
    let station_teme = transform_absolute_state(&station_itrf, ReferenceFrame::Teme, &rx_eop)?;

    let mut light_time = 0.0_f64;
    let mut satellite = None;
    for _ in 0..LIGHT_TIME_MAX_ITERATIONS {
        let transmit_epoch = receive_epoch.shift_si_seconds(-light_time, None)?;
        let teme = propagator.propagate(transmit_epoch)?;
        let next = norm(sub(station_teme.position_m, teme.position_m)) / SPEED_OF_LIGHT_M_PER_S;
        let converged = (next - light_time).abs() <= LIGHT_TIME_TOLERANCE_S;
        light_time = next;
        if converged {
            satellite = Some(teme);
            break;
        }
    }
    let satellite = satellite.ok_or_else(|| {
        SimError::InvalidArgument("pass light-time iteration did not converge".into())
    })?;

    let delta = sub(station_teme.position_m, satellite.position_m);
    let range = norm(delta);
    if range <= f64::EPSILON {
        return Err(SimError::InvalidArgument(
            "satellite coincides with ground station".into(),
        ));
    }
    let los = [delta[0] / range, delta[1] / range, delta[2] / range];
    let range_rate = dot(
        sub(station_teme.velocity_m_per_s, satellite.velocity_m_per_s),
        los,
    );

    // Apparent direction: where the station sees the signal come from, i.e.
    // the transmit-time satellite position rotated into ITRF at receive time.
    let apparent = AbsoluteOrbitState::new(
        receive_epoch,
        ReferenceFrame::Teme,
        satellite.position_m,
        satellite.velocity_m_per_s,
    )?;
    let apparent_itrf = transform_absolute_state(&apparent, ReferenceFrame::Itrf, &rx_eop)?;
    let aer = ground_station_az_el_range(station, apparent_itrf.position_m)?;
    Ok(Geometry {
        azimuth_rad: aer.azimuth_rad,
        elevation_rad: aer.elevation_rad,
        range_m: range,
        range_rate_m_per_s: range_rate,
        light_time_s: light_time,
        visible: aer.visible,
    })
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn doppler_hz(range_rate_m_per_s: f64, carrier_frequency_hz: f64) -> f64 {
    -range_rate_m_per_s * carrier_frequency_hz / SPEED_OF_LIGHT_M_PER_S
}

fn build_station(input: &PassStationInput) -> Result<GroundStation> {
    if !(-90.0..=90.0).contains(&input.latitude_deg) {
        return Err(SimError::InvalidArgument(
            "latitude_deg must be in [-90, 90]".into(),
        ));
    }
    if !(-180.0..=360.0).contains(&input.longitude_deg) {
        return Err(SimError::InvalidArgument(
            "longitude_deg must be in [-180, 360]".into(),
        ));
    }
    if !(-90.0..=90.0).contains(&input.minimum_elevation_deg) {
        return Err(SimError::InvalidArgument(
            "minimum_elevation_deg must be in [-90, 90]".into(),
        ));
    }
    if !input.height_m.is_finite() {
        return Err(SimError::NonFinite);
    }
    GroundStation::new(
        input.name.clone(),
        GeodeticPosition {
            latitude_rad: input.latitude_deg.to_radians(),
            longitude_rad: input.longitude_deg.to_radians(),
            height_m: input.height_m,
        },
        input.minimum_elevation_deg.to_radians(),
    )
}
