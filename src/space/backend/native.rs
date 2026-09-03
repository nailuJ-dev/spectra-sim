use crate::Result;

use super::super::{
    transform_absolute_state, AbsoluteEpoch, AbsoluteOrbitState, EopSample, OmmMessage,
    ReferenceFrame, Sgp4Propagator, Tle,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct NativeSpaceBackend;

impl NativeSpaceBackend {
    pub fn propagate_tle(&self, tle: &Tle, epoch: AbsoluteEpoch) -> Result<AbsoluteOrbitState> {
        Sgp4Propagator::from_tle(tle)?.propagate(epoch)
    }

    pub fn propagate_omm(
        &self,
        omm: &OmmMessage,
        epoch: AbsoluteEpoch,
    ) -> Result<AbsoluteOrbitState> {
        Sgp4Propagator::from_omm(omm)?.propagate(epoch)
    }

    pub fn transform(
        &self,
        state: &AbsoluteOrbitState,
        frame: ReferenceFrame,
        eop: &EopSample,
    ) -> Result<AbsoluteOrbitState> {
        transform_absolute_state(state, frame, eop)
    }
}
