//! `space-pass` subcommand: ground-station pass prediction from a TLE.

use std::fs;
use std::path::{Path, PathBuf};

use spectra_sim::viz::timeline_svg;
use spectra_sim::{compute_pass, parse_eop, PassConfig, PassResult, Result, SimError};

use super::io::{ensure_dir, io_with_path, read_json, read_text};

/// Runs the pass computation and writes `pass.json` plus SVG timelines.
pub fn run(config_path: &Path, out: &Path) -> Result<()> {
    let config: PassConfig = read_json(config_path)?;
    let eop_path = resolve_relative(config_path, &config.eop.path);
    let eop_text = read_text(&eop_path)?;
    let eop = parse_eop(config.eop.format, &eop_text).map_err(|error| match error {
        SimError::InvalidArgument(message) => {
            SimError::InvalidArgument(format!("{}: {message}", eop_path.display()))
        }
        other => other,
    })?;
    let result = compute_pass(&config, &eop)?;
    ensure_dir(out)?;
    super::io::write_json(&out.join("pass.json"), &result)?;
    write_svg(&out.join("elevation.svg"), &elevation_svg(&result))?;
    write_svg(&out.join("doppler.svg"), &doppler_svg(&result))?;
    println!(
        "{} samples, {} visible window(s)",
        result.samples.len(),
        result.windows.len()
    );
    for window in &result.windows {
        println!(
            "  t = {:.0} s .. {:.0} s, max elevation {:.1} deg at t = {:.0} s",
            window.start_s, window.end_s, window.max_elevation_deg, window.max_elevation_t_s
        );
    }
    Ok(())
}

/// Resolves `path` against the directory of `config_path` when it is relative.
fn resolve_relative(config_path: &Path, path: &str) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return candidate.to_path_buf();
    }
    config_path
        .parent()
        .map_or_else(|| candidate.to_path_buf(), |dir| dir.join(candidate))
}

fn elevation_svg(result: &PassResult) -> String {
    let series: Vec<(f64, f64)> = result
        .samples
        .iter()
        .map(|s| (s.t_s, s.elevation_deg))
        .collect();
    timeline_svg(
        &format!("Elevation — {}", result.station),
        "elevation (deg)",
        &series,
    )
}

fn doppler_svg(result: &PassResult) -> String {
    let series: Vec<(f64, f64)> = result
        .samples
        .iter()
        .map(|s| (s.t_s, s.doppler_hz))
        .collect();
    timeline_svg(
        &format!(
            "One-way Doppler (all samples; windows in pass.json) — {:.3} MHz",
            result.carrier_frequency_hz / 1e6
        ),
        "Doppler (Hz)",
        &series,
    )
}

fn write_svg(path: &Path, svg: &str) -> Result<()> {
    fs::write(path, svg).map_err(|error| io_with_path(error, path))
}
