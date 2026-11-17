#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(scenario) = serde_json::from_slice::<spectra_sim::Scenario>(data) {
        let _ = scenario.validate();
    }
});
