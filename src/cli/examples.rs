//! `examples` subcommand: list and print the scenarios embedded in the binary.

use spectra_sim::{Result, SimError};

/// Scenario files shipped inside the binary, as `(name, contents)`.
pub const EMBEDDED: &[(&str, &str)] = &[
    (
        "cuas_aircraft_adsb",
        include_str!("../../examples/scenarios/cuas_aircraft_adsb.json"),
    ),
    (
        "cuas_bird",
        include_str!("../../examples/scenarios/cuas_bird.json"),
    ),
    (
        "cuas_directional_radar",
        include_str!("../../examples/scenarios/cuas_directional_radar.json"),
    ),
    (
        "cuas_drone_isac",
        include_str!("../../examples/scenarios/cuas_drone_isac.json"),
    ),
    (
        "cuas_ship_ais",
        include_str!("../../examples/scenarios/cuas_ship_ais.json"),
    ),
    (
        "randomization",
        include_str!("../../examples/scenarios/randomization.json"),
    ),
    (
        "sigint_ofdm",
        include_str!("../../examples/scenarios/sigint_ofdm.json"),
    ),
    (
        "sigint_urban",
        include_str!("../../examples/scenarios/sigint_urban.json"),
    ),
];

/// Names of the embedded examples, sorted.
pub fn list() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = EMBEDDED.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    names
}

/// Contents of the embedded example `name`.
pub fn show(name: &str) -> Result<&'static str> {
    EMBEDDED
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, contents)| *contents)
        .ok_or_else(|| {
            SimError::NotFound(format!(
                "unknown example '{name}' (run `spectra-sim examples list`)"
            ))
        })
}

/// Prints the example names, one per line.
pub fn run_list() -> Result<()> {
    for name in list() {
        println!("{name}");
    }
    Ok(())
}

/// Prints the example `name` verbatim.
pub fn run_show(name: &str) -> Result<()> {
    print!("{}", show(name)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectra_sim::{DomainRandomization, Scenario};

    #[test]
    fn embedded_examples_all_validate() {
        assert_eq!(EMBEDDED.len(), 8);
        for (name, contents) in EMBEDDED {
            if *name == "randomization" {
                let parsed: std::result::Result<DomainRandomization, _> =
                    serde_json::from_str(contents);
                assert!(parsed.is_ok(), "{name}: {:?}", parsed.err());
            } else {
                let scenario: Scenario = serde_json::from_str(contents)
                    .unwrap_or_else(|error| panic!("{name}: {error}"));
                assert!(scenario.validate().is_ok(), "{name} failed validation");
            }
        }
    }

    #[test]
    fn show_unknown_is_not_found() {
        assert!(matches!(show("nope"), Err(SimError::NotFound(_))));
    }
}
