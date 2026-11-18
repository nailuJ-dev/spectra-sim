use std::path::Path;
use std::process::Command;

use spectra_sim::{
    compute_pass, parse_eop, pass_sample_count, visibility_windows, EopFormat, EopTable,
    PassConfig, PassSample, SimError, MAX_PASS_SAMPLES, SPEED_OF_LIGHT_M_PER_S,
};

const EXAMPLE: &str = "examples/space/vanguard1_pass.json";

fn example() -> (PassConfig, EopTable) {
    let config: PassConfig =
        serde_json::from_str(&std::fs::read_to_string(EXAMPLE).unwrap()).unwrap();
    let eop_text =
        std::fs::read_to_string(Path::new("examples/space").join(&config.eop.path)).unwrap();
    let eop = parse_eop(config.eop.format, &eop_text).unwrap();
    (config, eop)
}

#[test]
fn example_pass_has_three_windows_over_los_angeles() {
    let (config, eop) = example();
    let result = compute_pass(&config, &eop).unwrap();
    assert_eq!(result.samples.len(), 721);
    assert_eq!(result.windows.len(), 3, "{:?}", result.windows);
    assert_eq!(result.geometry, "instantaneous");
    for window in &result.windows {
        assert!(window.start_s <= window.max_elevation_t_s);
        assert!(window.max_elevation_t_s <= window.end_s);
        assert!(window.max_elevation_deg >= 10.0);
    }
}

#[test]
fn doppler_crosses_zero_at_closest_approach_with_negative_rate() {
    let (config, eop) = example();
    let result = compute_pass(&config, &eop).unwrap();
    let window = &result.windows[0];
    let in_window: Vec<&PassSample> = result
        .samples
        .iter()
        .filter(|s| s.t_s >= window.start_s && s.t_s <= window.end_s)
        .collect();
    let closest = in_window
        .iter()
        .min_by(|a, b| a.range_m.total_cmp(&b.range_m))
        .unwrap();
    // At closest approach the range rate changes sign and the Doppler rate is
    // at its most negative (range acceleration > 0).
    let max_doppler = in_window
        .iter()
        .map(|s| s.doppler_hz.abs())
        .fold(0.0_f64, f64::max);
    assert!(
        closest.doppler_hz.abs() < 0.2 * max_doppler,
        "doppler at closest approach {} vs max {max_doppler}",
        closest.doppler_hz
    );
    assert!(closest.doppler_rate_hz_per_s < 0.0);
}

#[test]
fn doppler_rate_matches_finite_difference_of_doppler_series() {
    let (config, eop) = example();
    let result = compute_pass(&config, &eop).unwrap();
    let s = &result.samples;
    let step = result.step_s;
    for i in 1..s.len() - 1 {
        let fd = (s[i + 1].doppler_hz - s[i - 1].doppler_hz) / (2.0 * step);
        assert!(
            (fd - s[i].doppler_rate_hz_per_s).abs() < 0.05,
            "t={} fd={fd} rate={}",
            s[i].t_s,
            s[i].doppler_rate_hz_per_s
        );
    }
}

#[test]
fn doppler_is_bounded_by_orbital_speed() {
    let (config, eop) = example();
    let result = compute_pass(&config, &eop).unwrap();
    // Vanguard 1 is never faster than ~8.5 km/s relative to the ground.
    let bound = 8_500.0 * config.carrier_frequency_hz / SPEED_OF_LIGHT_M_PER_S;
    for sample in &result.samples {
        assert!(sample.doppler_hz.abs() < bound, "{sample:?}");
        assert!((-90.0..=90.0).contains(&sample.elevation_deg));
        assert!((0.0..360.0).contains(&sample.azimuth_deg));
        assert!(sample.range_m > 0.0);
    }
}

#[test]
fn pass_computation_is_deterministic() {
    let (config, eop) = example();
    let a = serde_json::to_vec(&compute_pass(&config, &eop).unwrap()).unwrap();
    let b = serde_json::to_vec(&compute_pass(&config, &eop).unwrap()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn invalid_time_grids_are_rejected() {
    assert!(matches!(
        pass_sample_count(0.0, 1.0),
        Err(SimError::InvalidArgument(_))
    ));
    assert!(matches!(
        pass_sample_count(10.0, 0.0),
        Err(SimError::InvalidArgument(_))
    ));
    assert!(matches!(
        pass_sample_count(f64::NAN, 1.0),
        Err(SimError::InvalidArgument(_))
    ));
    assert!(matches!(
        pass_sample_count(1e12, 1.0),
        Err(SimError::DimensionLimit { .. })
    ));
    assert_eq!(pass_sample_count(10.0, 3.0).unwrap(), 4);
    assert_eq!(
        pass_sample_count((MAX_PASS_SAMPLES - 1) as f64, 1.0).unwrap(),
        MAX_PASS_SAMPLES
    );
}

#[test]
fn invalid_station_carrier_and_epoch_are_rejected() {
    let (config, eop) = example();
    let mut bad = config.clone();
    bad.ground_station.latitude_deg = 91.0;
    assert!(compute_pass(&bad, &eop).is_err());
    let mut bad = config.clone();
    bad.carrier_frequency_hz = 0.0;
    assert!(compute_pass(&bad, &eop).is_err());
    let mut bad = config.clone();
    bad.start_utc = "yesterday".into();
    assert!(compute_pass(&bad, &eop).is_err());
    // EOP table does not cover 2010: refused instead of extrapolated.
    let mut bad = config;
    bad.start_utc = "2010-01-01T00:00:00Z".into();
    assert!(compute_pass(&bad, &eop).is_err());
}

#[test]
fn visibility_windows_group_contiguous_samples() {
    let sample = |t_s: f64, elevation_deg: f64, visible: bool| PassSample {
        t_s,
        azimuth_deg: 0.0,
        elevation_deg,
        range_m: 1.0,
        range_rate_m_per_s: 0.0,
        doppler_hz: 0.0,
        doppler_rate_hz_per_s: 0.0,
        visible,
    };
    let samples = vec![
        sample(0.0, 5.0, false),
        sample(1.0, 12.0, true),
        sample(2.0, 30.0, true),
        sample(3.0, 15.0, true),
        sample(4.0, 2.0, false),
        sample(5.0, 11.0, true),
    ];
    let windows = visibility_windows(&samples);
    assert_eq!(windows.len(), 2);
    assert_eq!((windows[0].start_s, windows[0].end_s), (1.0, 3.0));
    assert_eq!(windows[0].max_elevation_t_s, 2.0);
    assert_eq!((windows[1].start_s, windows[1].end_s), (5.0, 5.0));
    assert!(visibility_windows(&[]).is_empty());
}

#[test]
fn eop_format_names_are_stable() {
    assert_eq!(
        serde_json::to_string(&EopFormat::Finals2000a).unwrap(),
        "\"finals2000a\""
    );
    assert_eq!(
        serde_json::to_string(&EopFormat::C04_20u24).unwrap(),
        "\"c04_20u24\""
    );
}

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_spectra-sim"))
        .args(args)
        .output()
        .expect("spectra-sim binary starts")
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("spectra-space-pass-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn cli_space_pass_writes_json_and_plots() {
    let out = temp_dir("ok");
    let output = run_cli(&["space-pass", EXAMPLE, "--out", out.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("3 visible window(s)"), "{stdout}");
    let result: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("pass.json")).unwrap()).unwrap();
    assert_eq!(result["schema_version"], "spectra-sim-pass-v1");
    for svg in ["elevation.svg", "doppler.svg"] {
        let text = std::fs::read_to_string(out.join(svg)).unwrap();
        assert!(text.starts_with("<svg") && text.trim_end().ends_with("</svg>"));
    }
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn cli_space_pass_missing_eop_file_exits_3() {
    let dir = temp_dir("noeop");
    std::fs::create_dir_all(&dir).unwrap();
    let text = std::fs::read_to_string(EXAMPLE)
        .unwrap()
        .replace("eop_illustrative_c04.txt", "missing_eop.txt");
    let config = dir.join("pass.json");
    std::fs::write(&config, text).unwrap();
    let output = run_cli(&[
        "space-pass",
        config.to_str().unwrap(),
        "--out",
        dir.join("out").to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing_eop.txt"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cli_space_pass_invalid_config_exits_2() {
    let dir = temp_dir("bad");
    std::fs::create_dir_all(&dir).unwrap();
    let text = std::fs::read_to_string(EXAMPLE)
        .unwrap()
        .replace("\"step_s\": 30.0", "\"step_s\": 0.0");
    let config = dir.join("pass.json");
    std::fs::write(&config, text).unwrap();
    // Point the EOP path back at the example file so only step_s is wrong.
    let text = std::fs::read_to_string(&config).unwrap().replace(
        "\"eop_illustrative_c04.txt\"",
        &format!(
            "{:?}",
            std::fs::canonicalize("examples/space/eop_illustrative_c04.txt").unwrap()
        ),
    );
    std::fs::write(&config, text).unwrap();
    let output = run_cli(&[
        "space-pass",
        config.to_str().unwrap(),
        "--out",
        dir.join("out").to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("step_s"));
    assert!(!dir.join("out").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
