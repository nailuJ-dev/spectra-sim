use sofars::{erst, pnp};

use crate::{AbsoluteOrbitState, EopSample, ReferenceFrame, Result, SimError};

const NOMINAL_EARTH_ROTATION_RAD_PER_S: f64 = 7.292_115_146_706_98e-5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameTransformModel {
    Iau2006_2000A,
    ValladoTeme,
}

pub fn transform_absolute_state(
    state: &AbsoluteOrbitState,
    target: ReferenceFrame,
    eop: &EopSample,
) -> Result<AbsoluteOrbitState> {
    eop.validate()?;
    if state.frame == target {
        return Ok(state.clone());
    }
    match (state.frame, target) {
        (ReferenceFrame::Gcrf, ReferenceFrame::Itrf) => gcrf_to_itrf(state, eop),
        (ReferenceFrame::Itrf, ReferenceFrame::Gcrf) => itrf_to_gcrf(state, eop),
        (ReferenceFrame::Teme, ReferenceFrame::Itrf) => teme_to_itrf(state, eop),
        (ReferenceFrame::Itrf, ReferenceFrame::Teme) => itrf_to_teme(state, eop),
        (ReferenceFrame::Teme, ReferenceFrame::Gcrf) => {
            let itrf = teme_to_itrf(state, eop)?;
            itrf_to_gcrf(&itrf, eop)
        }
        (ReferenceFrame::Gcrf, ReferenceFrame::Teme) => {
            let itrf = gcrf_to_itrf(state, eop)?;
            itrf_to_teme(&itrf, eop)
        }
        (ReferenceFrame::Icrf, ReferenceFrame::Gcrf)
        | (ReferenceFrame::Gcrf, ReferenceFrame::Icrf) => AbsoluteOrbitState::new(
            state.epoch,
            target,
            state.position_m,
            state.velocity_m_per_s,
        ),
        (ReferenceFrame::Eme2000, ReferenceFrame::Gcrf)
        | (ReferenceFrame::Gcrf, ReferenceFrame::Eme2000) => Err(SimError::InvalidArgument(
            "EME2000/GCRF bias transform is intentionally not approximated; use Orekit backend for this frame pair".into(),
        )),
        _ => Err(SimError::InvalidArgument(format!(
            "unsupported frame transform {:?} -> {:?}",
            state.frame, target
        ))),
    }
}

pub fn gcrf_to_itrf(state: &AbsoluteOrbitState, eop: &EopSample) -> Result<AbsoluteOrbitState> {
    require_frame(state, ReferenceFrame::Gcrf)?;
    let (tt1, tt2) = state.epoch.tt_parts(Some(eop))?;
    let (ut11, ut12) = state.epoch.ut1_parts(eop)?;
    let matrix = gcrf_to_itrf_matrix(tt1, tt2, ut11, ut12, eop);
    let r = mat_vec(&matrix, state.position_m);
    let inertial_v_rotated = mat_vec(&matrix, state.velocity_m_per_s);
    let omega = earth_rotation_rate(eop);
    let v = sub(inertial_v_rotated, cross([0.0, 0.0, omega], r));
    AbsoluteOrbitState::new(state.epoch, ReferenceFrame::Itrf, r, v)
}

pub fn itrf_to_gcrf(state: &AbsoluteOrbitState, eop: &EopSample) -> Result<AbsoluteOrbitState> {
    require_frame(state, ReferenceFrame::Itrf)?;
    let (tt1, tt2) = state.epoch.tt_parts(Some(eop))?;
    let (ut11, ut12) = state.epoch.ut1_parts(eop)?;
    let matrix = gcrf_to_itrf_matrix(tt1, tt2, ut11, ut12, eop);
    let transpose = transpose(matrix);
    let r = mat_vec(&transpose, state.position_m);
    let omega = earth_rotation_rate(eop);
    let corrected = add(
        state.velocity_m_per_s,
        cross([0.0, 0.0, omega], state.position_m),
    );
    let v = mat_vec(&transpose, corrected);
    AbsoluteOrbitState::new(state.epoch, ReferenceFrame::Gcrf, r, v)
}

/// Vallado-compatible TEME -> ITRF transform: IAU-1982 GMST to PEF followed by
/// polar motion. UT1 and polar motion are supplied by EOP; velocity includes
/// Earth rotation and LOD correction.
pub fn teme_to_itrf(state: &AbsoluteOrbitState, eop: &EopSample) -> Result<AbsoluteOrbitState> {
    require_frame(state, ReferenceFrame::Teme)?;
    let (ut11, ut12) = state.epoch.ut1_parts(eop)?;
    let gmst = erst::gmst82(ut11, ut12);
    let r_pef = rot_z(gmst, state.position_m);
    let v_rot = rot_z(gmst, state.velocity_m_per_s);
    let omega = earth_rotation_rate(eop);
    let v_pef = sub(v_rot, cross([0.0, 0.0, omega], r_pef));
    let pm = polar_motion_matrix(eop.polar_motion_x_rad, eop.polar_motion_y_rad);
    let r = mat_vec(&pm, r_pef);
    let v = mat_vec(&pm, v_pef);
    AbsoluteOrbitState::new(state.epoch, ReferenceFrame::Itrf, r, v)
}

pub fn itrf_to_teme(state: &AbsoluteOrbitState, eop: &EopSample) -> Result<AbsoluteOrbitState> {
    require_frame(state, ReferenceFrame::Itrf)?;
    let (ut11, ut12) = state.epoch.ut1_parts(eop)?;
    let gmst = erst::gmst82(ut11, ut12);
    let pm_t = transpose(polar_motion_matrix(
        eop.polar_motion_x_rad,
        eop.polar_motion_y_rad,
    ));
    let r_pef = mat_vec(&pm_t, state.position_m);
    let v_pef = mat_vec(&pm_t, state.velocity_m_per_s);
    let omega = earth_rotation_rate(eop);
    let v_inertial_pef = add(v_pef, cross([0.0, 0.0, omega], r_pef));
    let r = rot_z(-gmst, r_pef);
    let v = rot_z(-gmst, v_inertial_pef);
    AbsoluteOrbitState::new(state.epoch, ReferenceFrame::Teme, r, v)
}

fn gcrf_to_itrf_matrix(tt1: f64, tt2: f64, ut11: f64, ut12: f64, eop: &EopSample) -> [[f64; 3]; 3] {
    // IAU 2006/2000A CIP from SOFA plus observed IERS dX/dY celestial-pole offsets.
    let (x_model, y_model, _) = pnp::xys06a(tt1, tt2);
    let x = x_model + eop.dx_rad;
    let y = y_model + eop.dy_rad;
    let s = pnp::s06(tt1, tt2, x, y);
    let rc2i = pnp::c2ixys(x, y, s);
    let era = erst::era00(ut11, ut12);
    let sp = pnp::sp00(tt1, tt2);
    let rpom = pnp::pom00(eop.polar_motion_x_rad, eop.polar_motion_y_rad, sp);
    pnp::c2tcio(&rc2i, era, &rpom)
}

fn polar_motion_matrix(xp: f64, yp: f64) -> [[f64; 3]; 3] {
    // Vallado Appendix C: W = ROT1(yp) ROT2(xp), while the PEF->ITRF
    // position transform is W^T. Expanded explicitly to avoid rotation-sign ambiguity.
    let (sxp, cxp) = xp.sin_cos();
    let (syp, cyp) = yp.sin_cos();
    [
        [cxp, sxp * syp, sxp * cyp],
        [0.0, cyp, -syp],
        [-sxp, cxp * syp, cxp * cyp],
    ]
}

fn earth_rotation_rate(eop: &EopSample) -> f64 {
    NOMINAL_EARTH_ROTATION_RAD_PER_S * (1.0 - eop.lod_s / 86_400.0)
}

fn require_frame(state: &AbsoluteOrbitState, frame: ReferenceFrame) -> Result<()> {
    if state.frame != frame {
        return Err(SimError::InvalidArgument(format!(
            "expected {:?} state, received {:?}",
            frame, state.frame
        )));
    }
    Ok(())
}

fn rot_z(angle: f64, v: [f64; 3]) -> [f64; 3] {
    mat_vec(&rot_z_matrix(angle), v)
}
fn rot_z_matrix(a: f64) -> [[f64; 3]; 3] {
    let (s, c) = a.sin_cos();
    [[c, s, 0.0], [-s, c, 0.0], [0.0, 0.0, 1.0]]
}
fn transpose(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ]
}
fn mat_vec(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2] * b[2]))
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
