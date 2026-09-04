use serde::{Deserialize, Serialize};
use sofars::ts;

use crate::{Result, SimError};

use super::{EopSample, TimeScale};

const SECONDS_PER_DAY: f64 = 86_400.0;
const GPS_MINUS_TAI_SECONDS: f64 = -19.0;

/// Absolute astronomical epoch stored as a two-part Julian Date.
///
/// This type is intentionally separate from [`crate::Epoch`], whose `seconds`
/// field remains a relative simulation coordinate for backwards compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AbsoluteEpoch {
    jd1: f64,
    jd2: f64,
    scale: TimeScale,
}

impl AbsoluteEpoch {
    pub fn new(jd1: f64, jd2: f64, scale: TimeScale) -> Result<Self> {
        if !jd1.is_finite() || !jd2.is_finite() {
            return Err(SimError::NonFinite);
        }
        let (jd1, jd2) = normalize_parts(jd1, jd2);
        Ok(Self { jd1, jd2, scale })
    }

    pub fn from_calendar(
        year: i32,
        month: i32,
        day: i32,
        hour: i32,
        minute: i32,
        second: f64,
        scale: TimeScale,
    ) -> Result<Self> {
        if !second.is_finite() {
            return Err(SimError::NonFinite);
        }
        let label = scale_label(scale)?;
        let (jd1, jd2) =
            ts::dtf2d(label, year, month, day, hour, minute, second).map_err(|code| {
                SimError::InvalidArgument(format!("SOFA dtf2d rejected epoch, status={code}"))
            })?;
        Self::new(jd1, jd2, scale)
    }

    /// Parses CCSDS-style calendar epochs (`YYYY-MM-DDThh:mm:ss[.fff][Z]`) or
    /// day-of-year epochs (`YYYY-DDDThh:mm:ss[.fff][Z]`).
    pub fn parse_ccsds(value: &str, scale: TimeScale) -> Result<Self> {
        let raw = value.trim().trim_end_matches('Z');
        let (date, time) = raw
            .split_once('T')
            .ok_or_else(|| SimError::InvalidArgument("epoch must contain 'T'".into()))?;
        let (hour, minute, second) = parse_time(time)?;
        let date_parts: Vec<&str> = date.split('-').collect();
        match date_parts.as_slice() {
            [year, month, day] => Self::from_calendar(
                parse_i32(year, "year")?,
                parse_i32(month, "month")?,
                parse_i32(day, "day")?,
                hour,
                minute,
                second,
                scale,
            ),
            [year, doy] => {
                let year = parse_i32(year, "year")?;
                let doy = parse_i32(doy, "day of year")?;
                let (month, day) = month_day_from_doy(year, doy)?;
                Self::from_calendar(year, month, day, hour, minute, second, scale)
            }
            _ => Err(SimError::InvalidArgument(
                "unsupported CCSDS epoch date representation".into(),
            )),
        }
    }

    pub fn jd1(&self) -> f64 {
        self.jd1
    }

    pub fn jd2(&self) -> f64 {
        self.jd2
    }

    pub fn scale(&self) -> TimeScale {
        self.scale
    }

    pub fn julian_date(&self) -> f64 {
        self.jd1 + self.jd2
    }

    pub fn modified_julian_date(&self) -> f64 {
        self.julian_date() - 2_400_000.5
    }

    /// Converts this instant to UTC quasi-JD using the supplied UT1-UTC when
    /// the source scale is UT1.
    pub fn utc_parts(&self, eop: Option<&EopSample>) -> Result<(f64, f64)> {
        match self.scale {
            TimeScale::Utc => Ok((self.jd1, self.jd2)),
            TimeScale::Tai => sofa_pair(ts::taiutc(self.jd1, self.jd2), "TAI->UTC"),
            TimeScale::Tt => {
                let (tai1, tai2) = sofa_pair(ts::tttai(self.jd1, self.jd2), "TT->TAI")?;
                sofa_pair(ts::taiutc(tai1, tai2), "TAI->UTC")
            }
            TimeScale::Ut1 => {
                let eop = require_eop(eop)?;
                sofa_pair(
                    ts::ut1utc(self.jd1, self.jd2, eop.ut1_minus_utc_s),
                    "UT1->UTC",
                )
            }
            TimeScale::Gps => {
                let (tai1, tai2) = add_seconds(self.jd1, self.jd2, -GPS_MINUS_TAI_SECONDS);
                sofa_pair(ts::taiutc(tai1, tai2), "GPS->TAI->UTC")
            }
        }
    }

    pub fn tai_parts(&self, eop: Option<&EopSample>) -> Result<(f64, f64)> {
        match self.scale {
            TimeScale::Tai => Ok((self.jd1, self.jd2)),
            TimeScale::Utc => sofa_pair(ts::utctai(self.jd1, self.jd2), "UTC->TAI"),
            TimeScale::Tt => sofa_pair(ts::tttai(self.jd1, self.jd2), "TT->TAI"),
            TimeScale::Ut1 => {
                let (u1, u2) = self.utc_parts(eop)?;
                sofa_pair(ts::utctai(u1, u2), "UT1->UTC->TAI")
            }
            TimeScale::Gps => Ok(add_seconds(self.jd1, self.jd2, -GPS_MINUS_TAI_SECONDS)),
        }
    }

    pub fn tt_parts(&self, eop: Option<&EopSample>) -> Result<(f64, f64)> {
        match self.scale {
            TimeScale::Tt => Ok((self.jd1, self.jd2)),
            _ => {
                let (tai1, tai2) = self.tai_parts(eop)?;
                sofa_pair(ts::taitt(tai1, tai2), "TAI->TT")
            }
        }
    }

    pub fn ut1_parts(&self, eop: &EopSample) -> Result<(f64, f64)> {
        if self.scale == TimeScale::Ut1 {
            return Ok((self.jd1, self.jd2));
        }
        let (utc1, utc2) = self.utc_parts(Some(eop))?;
        sofa_pair(ts::utcut1(utc1, utc2, eop.ut1_minus_utc_s), "UTC->UT1")
    }

    /// Shifts an instant by SI seconds through the continuous TAI scale.
    pub fn shift_si_seconds(&self, seconds: f64, eop: Option<&EopSample>) -> Result<Self> {
        if !seconds.is_finite() {
            return Err(SimError::NonFinite);
        }
        let (tai1, tai2) = self.tai_parts(eop)?;
        let (shifted1, shifted2) = add_seconds(tai1, tai2, seconds);
        match self.scale {
            TimeScale::Tai => Self::new(shifted1, shifted2, TimeScale::Tai),
            TimeScale::Tt => {
                let (a, b) = sofa_pair(ts::taitt(shifted1, shifted2), "TAI->TT")?;
                Self::new(a, b, TimeScale::Tt)
            }
            TimeScale::Utc => {
                let (a, b) = sofa_pair(ts::taiutc(shifted1, shifted2), "TAI->UTC")?;
                Self::new(a, b, TimeScale::Utc)
            }
            TimeScale::Gps => {
                let (a, b) = add_seconds(shifted1, shifted2, GPS_MINUS_TAI_SECONDS);
                Self::new(a, b, TimeScale::Gps)
            }
            TimeScale::Ut1 => {
                let eop = require_eop(eop)?;
                let (utc1, utc2) = sofa_pair(ts::taiutc(shifted1, shifted2), "TAI->UTC")?;
                let (a, b) = sofa_pair(ts::utcut1(utc1, utc2, eop.ut1_minus_utc_s), "UTC->UT1")?;
                Self::new(a, b, TimeScale::Ut1)
            }
        }
    }

    /// Difference in SI seconds, evaluated in TAI to preserve leap-second semantics.
    pub fn seconds_since(&self, earlier: &Self, eop: Option<&EopSample>) -> Result<f64> {
        let (a1, a2) = self.tai_parts(eop)?;
        let (b1, b2) = earlier.tai_parts(eop)?;
        Ok(((a1 - b1) + (a2 - b2)) * SECONDS_PER_DAY)
    }
}

fn sofa_pair(result: std::result::Result<(f64, f64), i32>, operation: &str) -> Result<(f64, f64)> {
    result.map_err(|code| {
        SimError::InvalidArgument(format!("SOFA {operation} failed, status={code}"))
    })
}

fn require_eop(value: Option<&EopSample>) -> Result<&EopSample> {
    value.ok_or_else(|| SimError::InvalidArgument("EOP data is required for UT1 conversion".into()))
}

fn add_seconds(jd1: f64, jd2: f64, seconds: f64) -> (f64, f64) {
    normalize_parts(jd1, jd2 + seconds / SECONDS_PER_DAY)
}

fn normalize_parts(jd1: f64, jd2: f64) -> (f64, f64) {
    let whole = jd2.floor();
    (jd1 + whole, jd2 - whole)
}

fn scale_label(scale: TimeScale) -> Result<&'static str> {
    match scale {
        TimeScale::Utc => Ok("UTC"),
        TimeScale::Tai => Ok("TAI"),
        TimeScale::Tt => Ok("TT"),
        TimeScale::Ut1 => Ok("UT1"),
        TimeScale::Gps => Ok("GPS"),
    }
}

fn parse_time(value: &str) -> Result<(i32, i32, f64)> {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 3 {
        return Err(SimError::InvalidArgument("invalid CCSDS clock time".into()));
    }
    Ok((
        parse_i32(parts[0], "hour")?,
        parse_i32(parts[1], "minute")?,
        parts[2]
            .parse::<f64>()
            .map_err(|_| SimError::InvalidArgument("invalid second field".into()))?,
    ))
}

fn parse_i32(value: &str, label: &str) -> Result<i32> {
    value
        .parse::<i32>()
        .map_err(|_| SimError::InvalidArgument(format!("invalid {label}")))
}

fn month_day_from_doy(year: i32, doy: i32) -> Result<(i32, i32)> {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let max = if leap { 366 } else { 365 };
    if doy < 1 || doy > max {
        return Err(SimError::InvalidArgument("day of year out of range".into()));
    }
    let mut remaining = doy;
    for (index, days) in month_days.iter().enumerate() {
        if remaining <= *days {
            return Ok(((index + 1) as i32, remaining));
        }
        remaining -= *days;
    }
    Err(SimError::InvalidArgument(
        "day of year conversion failed".into(),
    ))
}
