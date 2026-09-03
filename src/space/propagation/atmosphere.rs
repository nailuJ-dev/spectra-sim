use serde::{Deserialize, Serialize};

use crate::{Result, SimError};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericLayer {
    pub path_length_km: f64,
    pub pressure_hpa: f64,
    pub water_vapour_density_g_m3: f64,
    pub temperature_k: f64,
}

impl AtmosphericLayer {
    pub fn validate(&self) -> Result<()> {
        let values = [
            self.path_length_km,
            self.pressure_hpa,
            self.water_vapour_density_g_m3,
            self.temperature_k,
        ];
        if values.iter().any(|v| !v.is_finite()) {
            return Err(SimError::NonFinite);
        }
        if self.path_length_km < 0.0
            || self.pressure_hpa <= 0.0
            || self.water_vapour_density_g_m3 < 0.0
            || self.temperature_k <= 0.0
        {
            return Err(SimError::InvalidArgument(
                "invalid atmospheric layer".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GaseousSpecificAttenuation {
    pub dry_air_db_per_km: f64,
    pub water_vapour_db_per_km: f64,
}

impl GaseousSpecificAttenuation {
    pub fn total_db_per_km(&self) -> f64 {
        self.dry_air_db_per_km + self.water_vapour_db_per_km
    }
}

/// ITU-R P.676-13 line-by-line specific attenuation for dry air and water vapour.
/// Frequency is in GHz, pressure in hPa, water-vapour density in g/m^3 and
/// temperature in kelvin. The embedded spectroscopic tables are v13.
pub fn gaseous_specific_attenuation_p676_13(
    frequency_ghz: f64,
    pressure_hpa: f64,
    water_vapour_density_g_m3: f64,
    temperature_k: f64,
) -> Result<GaseousSpecificAttenuation> {
    if [
        frequency_ghz,
        pressure_hpa,
        water_vapour_density_g_m3,
        temperature_k,
    ]
    .iter()
    .any(|v| !v.is_finite())
    {
        return Err(SimError::NonFinite);
    }
    if frequency_ghz <= 0.0
        || pressure_hpa <= 0.0
        || water_vapour_density_g_m3 < 0.0
        || temperature_k <= 0.0
    {
        return Err(SimError::InvalidArgument(
            "invalid P.676 atmospheric inputs".into(),
        ));
    }
    let oxygen = parse_lines(include_str!("p676_v13_oxygen.csv"))?;
    let water = parse_lines(include_str!("p676_v13_water.csv"))?;
    let theta = 300.0 / temperature_k;
    let e = water_vapour_density_g_m3 * temperature_k / 216.7;

    let mut n_oxygen = 0.0;
    for row in oxygen {
        let f0 = row[0];
        let a1 = row[1];
        let a2 = row[2];
        let a3 = row[3];
        let a4 = row[4];
        let a5 = row[5];
        let a6 = row[6];
        let mut df = a3 * 1.0e-4 * (pressure_hpa * theta.powf(0.8 - a4) + 1.1 * e * theta);
        df = (df * df + 2.25e-6).sqrt();
        let delta = (a5 + a6 * theta) * 1.0e-4 * (pressure_hpa + e) * theta.powf(0.8);
        let fi = frequency_ghz / f0
            * ((df - delta * (f0 - frequency_ghz)) / ((f0 - frequency_ghz).powi(2) + df.powi(2))
                + (df - delta * (f0 + frequency_ghz))
                    / ((f0 + frequency_ghz).powi(2) + df.powi(2)));
        let si = a1 * 1.0e-7 * pressure_hpa * theta.powi(3) * (a2 * (1.0 - theta)).exp();
        n_oxygen += si * fi;
    }
    let d = 5.6e-4 * (pressure_hpa + e) * theta.powf(0.8);
    let n_d = frequency_ghz
        * pressure_hpa
        * theta.powi(2)
        * (6.14e-5 / (d * (1.0 + (frequency_ghz / d).powi(2)))
            + 1.4e-12 * pressure_hpa * theta.powf(1.5) / (1.0 + 1.9e-5 * frequency_ghz.powf(1.5)));
    n_oxygen += n_d;

    let mut n_water = 0.0;
    for row in water {
        let f0 = row[0];
        let b1 = row[1];
        let b2 = row[2];
        let b3 = row[3];
        let b4 = row[4];
        let b5 = row[5];
        let b6 = row[6];
        let raw_df = b3 * 1.0e-4 * (pressure_hpa * theta.powf(b4) + b5 * e * theta.powf(b6));
        let df = 0.535 * raw_df + (0.217 * raw_df.powi(2) + 2.1316e-12 * f0.powi(2) / theta).sqrt();
        let fi = frequency_ghz / f0
            * (df / ((f0 - frequency_ghz).powi(2) + df.powi(2))
                + df / ((f0 + frequency_ghz).powi(2) + df.powi(2)));
        let si = b1 * 1.0e-1 * e * theta.powf(3.5) * (b2 * (1.0 - theta)).exp();
        n_water += si * fi;
    }

    Ok(GaseousSpecificAttenuation {
        dry_air_db_per_km: 0.1820 * frequency_ghz * n_oxygen,
        water_vapour_db_per_km: 0.1820 * frequency_ghz * n_water,
    })
}

pub fn integrate_gaseous_attenuation_p676_13(
    frequency_ghz: f64,
    layers: &[AtmosphericLayer],
) -> Result<f64> {
    if layers.is_empty() {
        return Err(SimError::InvalidArgument(
            "atmospheric profile is empty".into(),
        ));
    }
    let mut total = 0.0;
    for layer in layers {
        layer.validate()?;
        total += gaseous_specific_attenuation_p676_13(
            frequency_ghz,
            layer.pressure_hpa,
            layer.water_vapour_density_g_m3,
            layer.temperature_k,
        )?
        .total_db_per_km()
            * layer.path_length_km;
    }
    Ok(total)
}

/// Deterministic rain attenuation term when the P.838 k/alpha coefficients and
/// P.618 effective path length have already been selected for the polarization,
/// frequency, latitude and availability target of the link.
pub fn rain_attenuation_from_effective_path_db(
    rain_rate_mm_per_h: f64,
    k: f64,
    alpha: f64,
    effective_path_km: f64,
) -> Result<f64> {
    if [rain_rate_mm_per_h, k, alpha, effective_path_km]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err(SimError::NonFinite);
    }
    if rain_rate_mm_per_h < 0.0 || k < 0.0 || alpha < 0.0 || effective_path_km < 0.0 {
        return Err(SimError::InvalidArgument(
            "invalid rain attenuation input".into(),
        ));
    }
    Ok(k * rain_rate_mm_per_h.powf(alpha) * effective_path_km)
}

/// ITU-R P.840-9 cloud liquid-water mass absorption coefficient K_L(f).
///
/// Frequency is in GHz. This uses the P.840-9 frequency correction applied
/// to the Recommendation's double-Debye liquid-water model at 273.75 K.
/// The result is in dB/(kg/m²), suitable for columnar liquid-water content.
pub fn cloud_liquid_mass_absorption_coefficient_p840_9(frequency_ghz: f64) -> Result<f64> {
    if !frequency_ghz.is_finite() {
        return Err(SimError::NonFinite);
    }
    if frequency_ghz <= 0.0 || frequency_ghz > 1_000.0 {
        return Err(SimError::InvalidArgument(
            "P.840-9 frequency must be in (0, 1000] GHz".into(),
        ));
    }
    let base = cloud_specific_attenuation_p840_debye(frequency_ghz, 0.60)?;
    let a1 = 0.1522;
    let a2 = 11.51;
    let a3 = -10.4912;
    let f1 = -23.9589;
    let f2 = 219.2096;
    let sigma1 = 3.2991e3;
    let sigma2 = 2.7595e6;
    let correction = a1 * (-(frequency_ghz - f1).powi(2) / sigma1).exp()
        + a2 * (-(frequency_ghz - f2).powi(2) / sigma2).exp()
        + a3;
    let coefficient = base * correction;
    if !coefficient.is_finite() || coefficient < 0.0 {
        return Err(SimError::InvalidArgument(
            "P.840-9 produced an invalid cloud absorption coefficient".into(),
        ));
    }
    Ok(coefficient)
}

/// Cloud attenuation from reduced liquid-water column content following
/// ITU-R P.840-9: A = L_red K_L / sin(elevation).
pub fn cloud_attenuation_p840_9_db(
    frequency_ghz: f64,
    reduced_liquid_water_kg_per_m2: f64,
    elevation_rad: f64,
) -> Result<f64> {
    let coefficient = cloud_liquid_mass_absorption_coefficient_p840_9(frequency_ghz)?;
    cloud_attenuation_from_liquid_water_db(
        reduced_liquid_water_kg_per_m2,
        coefficient,
        elevation_rad,
    )
}

fn cloud_specific_attenuation_p840_debye(frequency_ghz: f64, temperature_c: f64) -> Result<f64> {
    if !temperature_c.is_finite() {
        return Err(SimError::NonFinite);
    }
    let temperature_k = temperature_c + 273.15;
    if temperature_k <= 0.0 {
        return Err(SimError::InvalidArgument(
            "invalid liquid-water temperature".into(),
        ));
    }
    let theta = 300.0 / temperature_k;
    let epsilon0 = 77.66 + 103.3 * (theta - 1.0);
    let epsilon1 = 0.0671 * epsilon0;
    let epsilon2 = 3.52;
    let fp = 20.20 - 146.0 * (theta - 1.0) + 316.0 * (theta - 1.0).powi(2);
    let fs = 39.8 * fp;
    let epsilon_p = (epsilon0 - epsilon1) / (1.0 + (frequency_ghz / fp).powi(2))
        + (epsilon1 - epsilon2) / (1.0 + (frequency_ghz / fs).powi(2))
        + epsilon2;
    let epsilon_pp = frequency_ghz * (epsilon0 - epsilon1)
        / (fp * (1.0 + (frequency_ghz / fp).powi(2)))
        + frequency_ghz * (epsilon1 - epsilon2) / (fs * (1.0 + (frequency_ghz / fs).powi(2)));
    if epsilon_pp <= 0.0 {
        return Err(SimError::InvalidArgument(
            "invalid P.840 liquid-water dielectric loss".into(),
        ));
    }
    let eta = (2.0 + epsilon_p) / epsilon_pp;
    Ok(0.819 * frequency_ghz / (epsilon_pp * (1.0 + eta.powi(2))))
}

pub fn cloud_attenuation_from_liquid_water_db(
    liquid_water_kg_per_m2: f64,
    specific_attenuation_db_per_kg_m2: f64,
    elevation_rad: f64,
) -> Result<f64> {
    if [
        liquid_water_kg_per_m2,
        specific_attenuation_db_per_kg_m2,
        elevation_rad,
    ]
    .iter()
    .any(|v| !v.is_finite())
    {
        return Err(SimError::NonFinite);
    }
    if liquid_water_kg_per_m2 < 0.0
        || specific_attenuation_db_per_kg_m2 < 0.0
        || elevation_rad <= 0.0
        || elevation_rad > std::f64::consts::FRAC_PI_2
    {
        return Err(SimError::InvalidArgument(
            "invalid cloud attenuation input".into(),
        ));
    }
    Ok(liquid_water_kg_per_m2 * specific_attenuation_db_per_kg_m2 / elevation_rad.sin())
}

fn parse_lines(data: &str) -> Result<Vec<[f64; 7]>> {
    let mut rows = Vec::new();
    for line in data.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values = line
            .split(',')
            .map(|raw| raw.trim().parse::<f64>())
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| {
                SimError::InvalidArgument("invalid embedded P.676 coefficient table".into())
            })?;
        if values.len() != 7 {
            return Err(SimError::InvalidArgument(
                "invalid embedded P.676 coefficient row".into(),
            ));
        }
        rows.push([
            values[0], values[1], values[2], values[3], values[4], values[5], values[6],
        ]);
    }
    Ok(rows)
}
