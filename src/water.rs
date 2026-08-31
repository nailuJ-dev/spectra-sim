use serde::{Deserialize, Serialize};

use crate::{Complex64, MaterialProperties, ModelMaturity, Result, SimError, ValidityIssue, ValidityLedger};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaterPermittivityModel {
    ConductiveLowFrequency,
    MeissnerWentzDoubleDebye,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WaterState {
    pub temperature_c: f64,
    pub salinity_psu: f64,
}

impl WaterState {
    pub fn new(temperature_c: f64, salinity_psu: f64) -> Result<Self> {
        if !temperature_c.is_finite() || !salinity_psu.is_finite() || salinity_psu < 0.0 {
            return Err(SimError::InvalidArgument("water temperature and salinity must be finite; salinity must be non-negative".into()));
        }
        Ok(Self { temperature_c, salinity_psu })
    }
}

pub fn water_properties(
    frequency_hz: f64,
    state: &WaterState,
    model: WaterPermittivityModel,
) -> Result<MaterialProperties> {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err(SimError::InvalidArgument("frequency_hz must be finite and positive".into()));
    }
    let mut validity = ValidityLedger::default();
    let conductivity = seawater_conductivity_s_per_m(state.temperature_c, state.salinity_psu);
    let epsilon = match model {
        WaterPermittivityModel::ConductiveLowFrequency => {
            validity.record_model(
                "conductive-low-frequency-water",
                "1",
                "Maxwell lossy-medium model with Meissner-Wentz conductivity regression",
                ModelMaturity::ValidatedWithWarnings,
            );
            if frequency_hz > 1.0e8 {
                validity.push(ValidityIssue::extrapolation("water", "conductive low-frequency approximation requested above 100 MHz"));
            }
            Complex64::new(static_permittivity_pure(state.temperature_c), 0.0)
        }
        WaterPermittivityModel::MeissnerWentzDoubleDebye => {
            validity.record_model(
                "meissner-wentz-double-debye",
                "2004",
                "DOI:10.1109/TGRS.2004.831888",
                ModelMaturity::ValidatedReference,
            );
            if state.salinity_psu > 0.0 && (!(-2.0..=29.0).contains(&state.temperature_c) || state.salinity_psu > 40.0 || frequency_hz > 90.0e9) {
                validity.push(ValidityIssue::extrapolation("water", "sea-water state is outside the published validation domain (-2..29 C, 0..40 PSU, <=90 GHz)"));
            }
            if state.salinity_psu == 0.0 && (!(-20.0..=40.0).contains(&state.temperature_c) || frequency_hz > 500.0e9) {
                validity.push(ValidityIssue::extrapolation("water", "pure-water state is outside the published validation domain (-20..40 C, <=500 GHz)"));
            }
            meissner_wentz_relative_permittivity(frequency_hz, state.temperature_c, state.salinity_psu, conductivity)
        }
    };
    Ok(MaterialProperties {
        relative_permittivity: epsilon,
        relative_permeability: 1.0,
        conductivity_s_per_m: if model == WaterPermittivityModel::ConductiveLowFrequency { conductivity } else { 0.0 },
        validity,
    })
}

fn static_permittivity_pure(t: f64) -> f64 {
    (3.708_86e4 - 8.216_8e1 * t) / (4.218_54e2 + t)
}

fn pure_water_parameters(t: f64) -> (f64, f64, f64, f64, f64) {
    const A: [f64; 11] = [
        5.7230, 2.2379e-2, -7.1237e-4, 5.0478, -7.0315e-2, 6.0059e-4,
        3.6143, 2.8841e-2, 1.3652e-1, 1.4825e-3, 2.4166e-4,
    ];
    let eps_s = static_permittivity_pure(t);
    let eps_1 = A[0] + A[1] * t + A[2] * t * t;
    let nu_1_ghz = (45.0 + t) / (A[3] + A[4] * t + A[5] * t * t);
    let eps_inf = A[6] + A[7] * t;
    let nu_2_ghz = (45.0 + t) / (A[8] + A[9] * t + A[10] * t * t);
    (eps_s, eps_1, eps_inf, nu_1_ghz, nu_2_ghz)
}

fn meissner_wentz_relative_permittivity(frequency_hz: f64, t: f64, s: f64, conductivity: f64) -> Complex64 {
    const B: [f64; 13] = [
        -3.56417e-3, 4.74868e-6, 1.15574e-5, 2.39357e-3, -3.13530e-5,
        2.52477e-7, -6.28908e-3, 1.76032e-4, -9.22144e-5, -1.99723e-2,
        1.81176e-4, -2.04265e-3, 1.57883e-4,
    ];
    let (eps_s0, eps_10, eps_inf0, nu_10, nu_20) = pure_water_parameters(t);
    let eps_s = eps_s0 * (B[0] * s + B[1] * s * s + B[2] * t * s).exp();
    let nu_1 = nu_10 * (1.0 + s * (B[3] + B[4] * t + B[5] * t * t));
    let eps_1 = eps_10 * (B[6] * s + B[7] * s * s + B[8] * t * s).exp();
    let nu_2 = nu_20 * (1.0 + s * (B[9] + B[10] * t));
    let eps_inf = eps_inf0 * (1.0 + s * (B[11] + B[12] * t));
    let nu_ghz = frequency_hz / 1.0e9;
    let term1 = Complex64::new(eps_s - eps_1, 0.0).div(Complex64::new(1.0, nu_ghz / nu_1));
    let term2 = Complex64::new(eps_1 - eps_inf, 0.0).div(Complex64::new(1.0, nu_ghz / nu_2));
    let conduction_imag = if frequency_hz > 0.0 {
        -conductivity / (std::f64::consts::TAU * 8.854_187_812_8e-12 * frequency_hz)
    } else { 0.0 };
    term1.add(term2).add(Complex64::new(eps_inf, conduction_imag))
}

fn seawater_conductivity_s_per_m(t: f64, s: f64) -> f64 {
    if s <= f64::EPSILON { return 0.0; }
    let sigma_35 = 2.903_602 + 8.607e-2 * t + 4.738_817e-4 * t.powi(2)
        - 2.991e-6 * t.powi(3) + 4.304_7e-9 * t.powi(4);
    let r15 = s * (37.5109 + 5.45216 * s + 1.4409e-2 * s * s)
        / (1004.75 + 182.283 * s + s * s);
    let alpha0 = (6.9431 + 3.2841 * s - 9.9486e-2 * s * s)
        / (84.850 + 69.024 * s + s * s);
    let alpha1 = 49.843 - 0.2276 * s + 0.198e-2 * s * s;
    let rt_over_r15 = 1.0 + alpha0 * (t - 15.0) / (alpha1 + t);
    let r15_35 = 35.0 * (37.5109 + 5.45216 * 35.0 + 1.4409e-2 * 35.0 * 35.0)
        / (1004.75 + 182.283 * 35.0 + 35.0 * 35.0);
    sigma_35 * r15 * rt_over_r15 / r15_35
}
