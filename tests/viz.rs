use spectra_sim::viz::{constellation_svg, spectrogram_svg, spectrum_svg, timeline_svg};
use spectra_sim::{IqCaptureOut, IqSampleOut};
use std::path::PathBuf;
use std::process::Command;

fn capture_from(samples: Vec<(f32, f32)>) -> IqCaptureOut {
    IqCaptureOut {
        id: "test-capture".to_string(),
        timestamp_ms: 0,
        sample_rate_hz: 1_000.0,
        center_frequency_hz: 2.4e9,
        full_scale_v: 1.0,
        adc_bits: 12,
        samples: samples
            .into_iter()
            .map(|(i, q)| IqSampleOut { i, q })
            .collect(),
    }
}

fn tone(n: usize) -> IqCaptureOut {
    capture_from(
        (0..n)
            .map(|k| {
                let phase = 2.0 * std::f32::consts::PI * (k % 8) as f32 / 8.0;
                (phase.cos(), phase.sin())
            })
            .collect(),
    )
}

fn assert_svg(svg: &str) {
    assert!(
        svg.starts_with("<svg"),
        "starts with: {}",
        &svg[..20.min(svg.len())]
    );
    assert!(svg.trim_end().ends_with("</svg>"));
    assert!(!svg.contains("NaN"), "NaN in output");
    assert!(!svg.contains("inf"), "inf in output");
}

fn all_svgs(capture: &IqCaptureOut) -> Vec<String> {
    vec![
        constellation_svg(capture, 2000),
        spectrum_svg(capture, 64).unwrap_or_else(|e| panic!("spectrum: {e}")),
        spectrogram_svg(capture, 64, 32, 4096).unwrap_or_else(|e| panic!("spectrogram: {e}")),
        timeline_svg("t", "y", &[(0.0, 1.0), (1.0, 2.0)]),
    ]
}

#[test]
fn every_function_outputs_an_svg_document() {
    for svg in all_svgs(&tone(512)) {
        assert_svg(&svg);
    }
}

#[test]
fn spectrum_rejects_non_power_of_two() {
    let capture = tone(256);
    for bad in [0, 1, 3, 100, (1 << 20) + 1, 1 << 21] {
        assert!(spectrum_svg(&capture, bad).is_err(), "fft_len {bad}");
    }
    assert!(spectrum_svg(&capture, 2).is_ok());
}

#[test]
fn spectrogram_rejects_bad_parameters() {
    let capture = tone(256);
    assert!(spectrogram_svg(&capture, 100, 32, 4096).is_err());
    assert!(spectrogram_svg(&capture, 64, 0, 4096).is_err());
    assert!(spectrogram_svg(&capture, 64, 32, 0).is_err());
}

#[test]
fn bad_sample_rate_is_rejected() {
    let mut capture = tone(256);
    capture.sample_rate_hz = 0.0;
    assert!(spectrum_svg(&capture, 64).is_err());
    assert!(spectrogram_svg(&capture, 64, 32, 100).is_err());
}

#[test]
fn zero_and_one_sample_do_not_panic() {
    for n in [0, 1] {
        for svg in all_svgs(&tone(n)) {
            assert_svg(&svg);
        }
    }
    assert_svg(&timeline_svg("t", "y", &[]));
    assert_svg(&timeline_svg("t", "y", &[(1.0, 1.0)]));
}

#[test]
fn constant_and_odd_length_captures_have_no_nan() {
    let constant = capture_from(vec![(0.5, 0.5); 300]);
    let zeros = capture_from(vec![(0.0, 0.0); 300]);
    let odd = tone(1000);
    for capture in [&constant, &zeros, &odd] {
        for svg in all_svgs(capture) {
            assert_svg(&svg);
        }
    }
}

#[test]
fn timeline_skips_nan() {
    let svg = timeline_svg(
        "Doppler",
        "Hz",
        &[
            (0.0, 1.0),
            (f64::NAN, 2.0),
            (1.0, f64::INFINITY),
            (2.0, 3.0),
        ],
    );
    assert_svg(&svg);
    let points = svg
        .split("points=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_else(|| panic!("polyline present"));
    assert_eq!(points.split_whitespace().count(), 2, "points={points}");
}

#[test]
fn constellation_decimates() {
    let capture = capture_from((0..100_000).map(|k| ((k % 7) as f32 / 7.0, 0.1)).collect());
    let svg = constellation_svg(&capture, 500);
    let circles = svg.matches("<circle").count();
    assert!(circles > 0 && circles <= 500, "circles={circles}");
    let none = constellation_svg(&capture, 0);
    assert_svg(&none);
    assert_eq!(none.matches("<circle").count(), 0);
}

#[test]
fn spectrogram_respects_cell_cap() {
    let svg = spectrogram_svg(&tone(4096), 64, 32, 100).unwrap_or_else(|e| panic!("{e}"));
    let cells = svg.matches("class=\"cell\"").count();
    assert!(cells > 0 && cells <= 100, "cells={cells}");
    // No decimation needed: 4 frames x 64 bins.
    let exact = spectrogram_svg(&tone(256), 64, 64, 4096).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(exact.matches("class=\"cell\"").count(), 256);
}

#[test]
fn output_is_deterministic() {
    let capture = tone(1000);
    assert_eq!(all_svgs(&capture), all_svgs(&capture));
}

#[test]
fn titles_are_escaped() {
    let mut capture = tone(64);
    capture.id = "<a&\"b\">".to_string();
    for svg in all_svgs(&capture) {
        assert!(!svg.contains("<a&"), "unescaped id");
    }
    let svg = timeline_svg("x<y & z", "\"u\"", &[(0.0, 0.0), (1.0, 1.0)]);
    assert!(svg.contains("x&lt;y &amp; z"));
    assert!(svg.contains("&quot;u&quot;"));
}

fn unique_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("spectra-viz-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("mkdir: {e}"));
    dir
}

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_spectra-sim"))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("binary starts: {e}"))
}

fn sigint_capture(dir: &std::path::Path) -> PathBuf {
    let out = dir.join("run");
    let output = run_cli(&[
        "sigint",
        "examples/scenarios/sigint_urban.json",
        "--out",
        &out.to_string_lossy(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    out.join("iq_capture.json")
}

#[test]
fn plot_cli_writes_three_files() {
    let dir = unique_dir("three");
    let capture = sigint_capture(&dir);
    let plots = dir.join("plots");
    let output = run_cli(&[
        "plot",
        &capture.to_string_lossy(),
        "--out",
        &plots.to_string_lossy(),
        "--fft",
        "128",
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    for name in ["constellation.svg", "spectrum.svg", "spectrogram.svg"] {
        let text = std::fs::read_to_string(plots.join(name))
            .unwrap_or_else(|e| panic!("{name} written: {e}"));
        assert_svg(&text);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn plot_cli_rejects_bad_fft_and_missing_file() {
    let dir = unique_dir("badfft");
    let capture = sigint_capture(&dir);
    let plots = dir.join("plots");
    let output = run_cli(&[
        "plot",
        &capture.to_string_lossy(),
        "--out",
        &plots.to_string_lossy(),
        "--fft",
        "100",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!plots.exists(), "nothing written on invalid fft");
    let missing = run_cli(&["plot", &dir.join("nope.json").to_string_lossy()]);
    assert_eq!(missing.status.code(), Some(3));
    let _ = std::fs::remove_dir_all(&dir);
}
