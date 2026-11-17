//! `plot` subcommand: render SVG plots of an I/Q capture.

use std::fs;
use std::path::Path;

use spectra_sim::viz::{constellation_svg, spectrogram_svg, spectrum_svg};
use spectra_sim::{IqCaptureOut, Result};

use super::io::{ensure_dir, io_with_path, read_json};

/// Most points drawn in the constellation plot.
const MAX_CONSTELLATION_POINTS: usize = 2000;
/// Most cells drawn in the spectrogram.
const MAX_SPECTROGRAM_CELLS: usize = 4096;

/// Writes `constellation.svg`, `spectrum.svg` and `spectrogram.svg` into `out`.
/// All plots are rendered before anything is written, so an invalid `--fft`
/// leaves the output directory untouched.
pub fn run(capture_path: &Path, out: &Path, fft: usize) -> Result<()> {
    let capture: IqCaptureOut = read_json(capture_path)?;
    let plots = [
        (
            "constellation.svg",
            constellation_svg(&capture, MAX_CONSTELLATION_POINTS),
        ),
        ("spectrum.svg", spectrum_svg(&capture, fft)?),
        (
            "spectrogram.svg",
            spectrogram_svg(&capture, fft, (fft / 2).max(1), MAX_SPECTROGRAM_CELLS)?,
        ),
    ];
    ensure_dir(out)?;
    for (name, svg) in plots {
        let path = out.join(name);
        fs::write(&path, svg).map_err(|error| io_with_path(error, &path))?;
    }
    Ok(())
}
