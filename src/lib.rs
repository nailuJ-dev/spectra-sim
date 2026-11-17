#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod error;
pub mod export;
pub mod formats;
pub mod interfaces;
pub mod isac;
pub mod layers;
pub mod materials;
pub mod math;
pub mod physics;
pub mod radar;
pub mod randomization;
pub mod replay;
pub mod rf;
pub mod rng;
#[cfg(feature = "schema")]
pub mod schema;
pub mod simulator;
pub mod space;
pub mod validity;
pub mod viz;
pub mod water;
pub mod world;

pub use error::{Result, SimError};
pub use export::*;
pub use interfaces::*;
pub use isac::*;
pub use layers::*;
pub use materials::{
    propagation_constant, Complex64 as MaterialComplex64, MaterialProperties, PropagationConstant,
    VACUUM_PERMEABILITY_H_PER_M, VACUUM_PERMITTIVITY_F_PER_M,
};
pub use math::*;
pub use physics::*;
pub use radar::*;
pub use randomization::*;
pub use replay::*;
pub use rf::*;
pub use rng::*;
#[cfg(feature = "schema")]
pub use schema::{cuas_run_schema, scenario_schema, sigint_run_schema};
pub use simulator::*;
pub use space::*;
pub use validity::*;
pub use water::*;
pub use world::*;
