use sgp4::chrono::{Datelike, Timelike};

use crate::{Result, SimError};

use super::{AbsoluteEpoch, AbsoluteOrbitState, OmmMessage, ReferenceFrame, TimeScale, Tle};

/// SGP4 numerical/convention mode.
///
/// `IauWgs84` is the default high-level mode used by the Rust `sgp4` crate.
/// `AfspcWgs72` reproduces the historical AFSPC/Vallado verification convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sgp4Mode {
    IauWgs84,
    AfspcWgs72,
}

#[derive(Debug, Clone)]
pub struct Sgp4Propagator {
    elements: sgp4::Elements,
    epoch: AbsoluteEpoch,
    mode: Sgp4Mode,
    constants: sgp4::Constants,
}

impl Sgp4Propagator {
    /// Builds a propagator using the default IAU/WGS-84 convention.
    pub fn from_tle(tle: &Tle) -> Result<Self> {
        Self::from_elements_with_mode(tle.elements()?, Sgp4Mode::IauWgs84)
    }

    /// Builds a TLE propagator using AFSPC/WGS-72 compatibility mode.
    ///
    /// Use this when reproducing historical Vallado/AFSPC verification
    /// vectors or when explicit compatibility with an AFSPC implementation
    /// is required.
    pub fn from_tle_afspc(tle: &Tle) -> Result<Self> {
        Self::from_elements_with_mode(tle.elements()?, Sgp4Mode::AfspcWgs72)
    }

    pub fn from_tle_with_mode(tle: &Tle, mode: Sgp4Mode) -> Result<Self> {
        Self::from_elements_with_mode(tle.elements()?, mode)
    }

    /// Builds an OMM propagator using the default IAU/WGS-84 convention.
    pub fn from_omm(omm: &OmmMessage) -> Result<Self> {
        Self::from_elements_with_mode(omm.elements()?, Sgp4Mode::IauWgs84)
    }

    pub fn from_omm_afspc(omm: &OmmMessage) -> Result<Self> {
        Self::from_elements_with_mode(omm.elements()?, Sgp4Mode::AfspcWgs72)
    }

    pub fn from_omm_with_mode(omm: &OmmMessage, mode: Sgp4Mode) -> Result<Self> {
        Self::from_elements_with_mode(omm.elements()?, mode)
    }

    /// Builds a propagator using the default IAU/WGS-84 convention.
    pub fn from_elements(elements: sgp4::Elements) -> Result<Self> {
        Self::from_elements_with_mode(elements, Sgp4Mode::IauWgs84)
    }

    pub fn from_elements_with_mode(elements: sgp4::Elements, mode: Sgp4Mode) -> Result<Self> {
        let dt = elements.datetime;
        let seconds = f64::from(dt.second()) + f64::from(dt.nanosecond()) * 1.0e-9;

        let epoch = AbsoluteEpoch::from_calendar(
            dt.year(),
            dt.month() as i32,
            dt.day() as i32,
            dt.hour() as i32,
            dt.minute() as i32,
            seconds,
            TimeScale::Utc,
        )?;

        let constants = build_constants(&elements, mode)?;

        Ok(Self {
            elements,
            epoch,
            mode,
            constants,
        })
    }

    pub fn elements(&self) -> &sgp4::Elements {
        &self.elements
    }

    pub fn epoch(&self) -> AbsoluteEpoch {
        self.epoch
    }

    pub fn mode(&self) -> Sgp4Mode {
        self.mode
    }

    pub fn propagate_minutes(&self, minutes_since_epoch: f64) -> Result<AbsoluteOrbitState> {
        if !minutes_since_epoch.is_finite() {
            return Err(SimError::NonFinite);
        }

        let time = sgp4::MinutesSinceEpoch(minutes_since_epoch);

        let prediction = match self.mode {
            Sgp4Mode::IauWgs84 => self.constants.propagate(time),
            Sgp4Mode::AfspcWgs72 => self.constants.propagate_afspc_compatibility_mode(time),
        }
        .map_err(|error| SimError::InvalidArgument(format!("SGP4 propagation failed: {error}")))?;

        let epoch = self
            .epoch
            .shift_si_seconds(minutes_since_epoch * 60.0, None)?;

        AbsoluteOrbitState::new(
            epoch,
            ReferenceFrame::Teme,
            prediction.position.map(|value| value * 1_000.0),
            prediction.velocity.map(|value| value * 1_000.0),
        )
    }

    /// Propagates to an absolute epoch.
    ///
    /// SGP4 elapsed time conventionally ignores leap seconds; this method
    /// intentionally follows that SGP4 convention.
    pub fn propagate(&self, target: AbsoluteEpoch) -> Result<AbsoluteOrbitState> {
        let target_utc = target.utc_parts(None)?;
        let epoch_utc = self.epoch.utc_parts(None)?;

        let minutes =
            (((target_utc.0 - epoch_utc.0) + (target_utc.1 - epoch_utc.1)) * 86_400.0) / 60.0;

        let mut state = self.propagate_minutes(minutes)?;
        state.epoch = target;

        Ok(state)
    }
}

fn build_constants(elements: &sgp4::Elements, mode: Sgp4Mode) -> Result<sgp4::Constants> {
    let result = match mode {
        Sgp4Mode::IauWgs84 => sgp4::Constants::from_elements(elements),
        Sgp4Mode::AfspcWgs72 => sgp4::Constants::from_elements_afspc_compatibility_mode(elements),
    };

    result
        .map_err(|error| SimError::InvalidArgument(format!("SGP4 initialization failed: {error}")))
}
