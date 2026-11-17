//! Dependency-free SVG plots of I/Q captures and time series.
//!
//! Every function builds the document as a string with fixed-precision number
//! formatting, so output is byte-identical across runs. Non-finite values never
//! reach the document: degenerate input yields an empty plot instead.

use std::f64::consts::PI;
use std::fmt::Write as _;

use crate::{IqCaptureOut, IqSampleOut, Result, SimError};

/// Largest accepted FFT length (2^20 bins).
const MAX_FFT_LEN: usize = 1 << 20;
/// Power reported for empty bins, in dB.
const DB_FLOOR: f64 = -200.0;
/// Spectrum polylines are peak-decimated to at most this many points.
const MAX_SPECTRUM_POINTS: usize = 2048;

const MARGIN_LEFT: f64 = 70.0;
const MARGIN_RIGHT: f64 = 20.0;
const MARGIN_TOP: f64 = 36.0;
const MARGIN_BOTTOM: f64 = 50.0;
const WIDTH: f64 = 640.0;
const HEIGHT: f64 = 400.0;

/// Escapes text for use in XML character data and double-quoted attributes.
pub(crate) fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // Characters outside the XML 1.0 `Char` production would make the
            // document ill-formed; show the replacement character instead.
            '\u{0}'..='\u{8}'
            | '\u{b}'
            | '\u{c}'
            | '\u{e}'..='\u{1f}'
            | '\u{fffe}'
            | '\u{ffff}' => out.push('\u{fffd}'),
            _ => out.push(c),
        }
    }
    out
}

/// Fixed two-decimal number; non-finite values and negative zero print as `0.00`.
fn f2(value: f64) -> String {
    let text = if value.is_finite() {
        format!("{value:.2}")
    } else {
        "0.00".to_string()
    };
    if text == "-0.00" {
        "0.00".to_string()
    } else {
        text
    }
}

/// Returns a finite, strictly increasing range, widening degenerate input.
fn sane_range(lo: f64, hi: f64) -> (f64, f64) {
    if !lo.is_finite() || !hi.is_finite() {
        return (0.0, 1.0);
    }
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let scale = lo.abs().max(hi.abs()).max(1.0);
    if hi - lo > 1e-12 * scale {
        (lo, hi)
    } else {
        let pad = (scale * 0.05).max(1.0);
        (lo - pad, hi + pad)
    }
}

/// Tick positions and the label format shared by all ticks of one axis.
struct Ticks {
    values: Vec<f64>,
    divisor: f64,
    suffix: &'static str,
    decimals: usize,
}

impl Ticks {
    fn new(lo: f64, hi: f64) -> Self {
        let (lo, hi) = sane_range(lo, hi);
        let raw = (hi - lo) / 5.0;
        let magnitude = 10f64.powf(raw.log10().floor());
        let normalized = raw / magnitude;
        let step = magnitude
            * if normalized <= 1.0 {
                1.0
            } else if normalized <= 2.0 {
                2.0
            } else if normalized <= 5.0 {
                5.0
            } else {
                10.0
            };
        let mut values = Vec::new();
        let mut k = (lo / step).ceil();
        while k * step <= hi + step * 1e-9 && values.len() < 50 {
            values.push(k * step);
            k += 1.0;
        }
        let biggest = lo.abs().max(hi.abs());
        let (divisor, suffix) = if biggest >= 1e9 {
            (1e9, "G")
        } else if biggest >= 1e6 {
            (1e6, "M")
        } else if biggest >= 1e3 {
            (1e3, "k")
        } else {
            (1.0, "")
        };
        let scaled_step = step / divisor;
        let decimals = if scaled_step >= 1.0 {
            0
        } else {
            (-scaled_step.log10()).ceil().clamp(0.0, 9.0) as usize
        };
        Self {
            values,
            divisor,
            suffix,
            decimals,
        }
    }

    fn label(&self, value: f64) -> String {
        let text = format!("{:.*}", self.decimals, value / self.divisor);
        let text = if text
            .trim_start_matches('-')
            .chars()
            .all(|c| c == '0' || c == '.')
        {
            text.trim_start_matches('-').to_string()
        } else {
            text
        };
        format!("{text}{}", self.suffix)
    }
}

/// An SVG document with titled axes; data is added through [`Plot::mark`].
struct Plot {
    out: String,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    x_range: (f64, f64),
    y_range: (f64, f64),
    y_down: bool,
}

impl Plot {
    #[allow(clippy::too_many_arguments)]
    fn new(
        total_w: f64,
        total_h: f64,
        title: &str,
        x_label: &str,
        y_label: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        y_down: bool,
    ) -> Self {
        let x_range = sane_range(x_range.0, x_range.1);
        let y_range = sane_range(y_range.0, y_range.1);
        let plot = Self {
            out: String::new(),
            left: MARGIN_LEFT,
            top: MARGIN_TOP,
            width: total_w - MARGIN_LEFT - MARGIN_RIGHT,
            height: total_h - MARGIN_TOP - MARGIN_BOTTOM,
            x_range,
            y_range,
            y_down,
        };
        plot.start(total_w, total_h, title, x_label, y_label)
    }

    fn start(
        mut self,
        total_w: f64,
        total_h: f64,
        title: &str,
        x_label: &str,
        y_label: &str,
    ) -> Self {
        let _ = write!(
            self.out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
             viewBox=\"0 0 {w} {h}\" font-family=\"sans-serif\" font-size=\"11\">\n\
             <rect width=\"{w}\" height=\"{h}\" fill=\"#ffffff\"/>\n\
             <text x=\"{tx}\" y=\"22\" font-size=\"14\" text-anchor=\"middle\" fill=\"#111111\">{title}</text>\n",
            w = f2(total_w),
            h = f2(total_h),
            tx = f2(total_w / 2.0),
            title = escape_xml(title),
        );
        let x_ticks = Ticks::new(self.x_range.0, self.x_range.1);
        let y_ticks = Ticks::new(self.y_range.0, self.y_range.1);
        for &value in &x_ticks.values {
            let x = self.px(value);
            let _ = write!(
                self.out,
                "<line x1=\"{x}\" y1=\"{t}\" x2=\"{x}\" y2=\"{b}\" stroke=\"#e0e0e0\"/>\n\
                 <text x=\"{x}\" y=\"{ly}\" text-anchor=\"middle\" fill=\"#333333\">{label}</text>\n",
                x = f2(x),
                t = f2(self.top),
                b = f2(self.top + self.height),
                ly = f2(self.top + self.height + 16.0),
                label = x_ticks.label(value),
            );
        }
        for &value in &y_ticks.values {
            let y = self.py(value);
            let _ = write!(
                self.out,
                "<line x1=\"{l}\" y1=\"{y}\" x2=\"{r}\" y2=\"{y}\" stroke=\"#e0e0e0\"/>\n\
                 <text x=\"{lx}\" y=\"{ty}\" text-anchor=\"end\" fill=\"#333333\">{label}</text>\n",
                l = f2(self.left),
                r = f2(self.left + self.width),
                y = f2(y),
                lx = f2(self.left - 6.0),
                ty = f2(y + 4.0),
                label = y_ticks.label(value),
            );
        }
        let _ = write!(
            self.out,
            "<text x=\"{cx}\" y=\"{by}\" text-anchor=\"middle\" fill=\"#111111\">{xl}</text>\n\
             <text x=\"14\" y=\"{cy}\" text-anchor=\"middle\" fill=\"#111111\" \
             transform=\"rotate(-90 14 {cy})\">{yl}</text>\n",
            cx = f2(self.left + self.width / 2.0),
            by = f2(total_h - 10.0),
            cy = f2(self.top + self.height / 2.0),
            xl = escape_xml(x_label),
            yl = escape_xml(y_label),
        );
        self
    }

    fn px(&self, x: f64) -> f64 {
        self.left + (x - self.x_range.0) / (self.x_range.1 - self.x_range.0) * self.width
    }

    fn py(&self, y: f64) -> f64 {
        let fraction = (y - self.y_range.0) / (self.y_range.1 - self.y_range.0);
        if self.y_down {
            self.top + fraction * self.height
        } else {
            self.top + (1.0 - fraction) * self.height
        }
    }

    /// Appends raw, already-escaped SVG markup.
    fn mark(&mut self, markup: &str) {
        self.out.push_str(markup);
    }

    fn note(&mut self, text: &str) {
        let _ = writeln!(
            self.out,
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"end\" fill=\"#666666\">{t}</text>",
            x = f2(self.left + self.width),
            y = f2(self.top - 6.0),
            t = escape_xml(text),
        );
    }

    fn no_data(&mut self) {
        let _ = writeln!(
            self.out,
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" fill=\"#888888\">no data</text>",
            x = f2(self.left + self.width / 2.0),
            y = f2(self.top + self.height / 2.0),
        );
    }

    fn finish(mut self) -> String {
        let _ = writeln!(
            self.out,
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"none\" stroke=\"#444444\"/>\n</svg>",
            x = f2(self.left),
            y = f2(self.top),
            w = f2(self.width),
            h = f2(self.height),
        );
        self.out
    }
}

/// I/Q scatter plot with equal axes, drawn from at most `max_points` samples
/// picked at a uniform stride. `max_points == 0` yields an empty plot.
pub fn constellation_svg(capture: &IqCaptureOut, max_points: usize) -> String {
    let step = capture.samples.len().div_ceil(max_points.max(1)).max(1);
    let points: Vec<(f64, f64)> = if max_points == 0 {
        Vec::new()
    } else {
        capture
            .samples
            .iter()
            .step_by(step)
            .map(|s| (f64::from(s.i), f64::from(s.q)))
            .filter(|(i, q)| i.is_finite() && q.is_finite())
            .collect()
    };
    let extent = points
        .iter()
        .fold(0.0_f64, |acc, (i, q)| acc.max(i.abs()).max(q.abs()));
    let extent = if extent > 0.0 { extent * 1.1 } else { 1.0 };
    // Square plot area (410 x 410) so the I and Q axes have equal scale.
    let side = 410.0;
    let mut plot = Plot::new(
        side + MARGIN_LEFT + MARGIN_RIGHT,
        side + MARGIN_TOP + MARGIN_BOTTOM,
        &format!("Constellation - {}", capture.id),
        "I",
        "Q",
        (-extent, extent),
        (-extent, extent),
        false,
    );
    if points.is_empty() {
        plot.no_data();
    }
    for (i, q) in points {
        let markup = format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"#1f77b4\" fill-opacity=\"0.6\"/>\n",
            f2(plot.px(i)),
            f2(plot.py(q)),
        );
        plot.mark(&markup);
    }
    plot.finish()
}

/// Precomputed radix-2 FFT plan.
struct Fft {
    n: usize,
    twiddle_re: Vec<f64>,
    twiddle_im: Vec<f64>,
    reverse: Vec<usize>,
}

impl Fft {
    /// `n` must be a power of two.
    fn new(n: usize) -> Self {
        let bits = n.trailing_zeros();
        let reverse = (0..n)
            .map(|k| {
                if bits == 0 {
                    0
                } else {
                    k.reverse_bits() >> (usize::BITS - bits)
                }
            })
            .collect();
        let angle = |k: usize| -2.0 * PI * k as f64 / n as f64;
        Self {
            n,
            twiddle_re: (0..n / 2).map(|k| angle(k).cos()).collect(),
            twiddle_im: (0..n / 2).map(|k| angle(k).sin()).collect(),
            reverse,
        }
    }

    fn process(&self, re: &mut [f64], im: &mut [f64]) {
        for k in 0..self.n {
            let j = self.reverse[k];
            if j > k {
                re.swap(k, j);
                im.swap(k, j);
            }
        }
        let mut len = 2;
        while len <= self.n {
            let half = len / 2;
            let stride = self.n / len;
            for start in (0..self.n).step_by(len) {
                for k in 0..half {
                    let (wr, wi) = (self.twiddle_re[k * stride], self.twiddle_im[k * stride]);
                    let (a, b) = (start + k, start + k + half);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            len *= 2;
        }
    }
}

/// Hann-windowed periodogram engine for one FFT length.
struct Periodogram {
    fft: Fft,
    window: Vec<f64>,
    /// Sum of squared window weights, the power normalisation.
    window_power: f64,
    re: Vec<f64>,
    im: Vec<f64>,
}

impl Periodogram {
    fn new(fft_len: usize) -> Self {
        let window: Vec<f64> = (0..fft_len)
            .map(|k| 0.5 - 0.5 * (2.0 * PI * k as f64 / fft_len as f64).cos())
            .collect();
        let window_power = window.iter().map(|w| w * w).sum();
        Self {
            fft: Fft::new(fft_len),
            window,
            window_power,
            re: vec![0.0; fft_len],
            im: vec![0.0; fft_len],
        }
    }

    /// Adds the windowed power spectrum of the frame starting at `start`
    /// (zero-padded past the end of `samples`) to `acc`, in natural FFT order.
    fn accumulate(&mut self, samples: &[IqSampleOut], start: usize, acc: &mut [f64]) {
        for k in 0..self.window.len() {
            let (i, q) = match samples.get(start.saturating_add(k)) {
                Some(s) if s.i.is_finite() && s.q.is_finite() => (f64::from(s.i), f64::from(s.q)),
                _ => (0.0, 0.0),
            };
            self.re[k] = i * self.window[k];
            self.im[k] = q * self.window[k];
        }
        self.fft.process(&mut self.re, &mut self.im);
        for (k, slot) in acc.iter_mut().enumerate() {
            *slot += self.re[k] * self.re[k] + self.im[k] * self.im[k];
        }
    }

    /// Converts accumulated power (summed over `frames` frames) to dB.
    fn to_db(&self, power: f64, frames: usize) -> f64 {
        let mean = power / (self.window_power * frames as f64);
        if mean.is_finite() && mean > 0.0 {
            (10.0 * mean.log10()).max(DB_FLOOR)
        } else {
            DB_FLOOR
        }
    }
}

fn check_fft_len(fft_len: usize) -> Result<()> {
    if fft_len < 2 || !fft_len.is_power_of_two() || fft_len > MAX_FFT_LEN {
        return Err(SimError::InvalidArgument(format!(
            "fft length must be a power of two between 2 and {MAX_FFT_LEN}, got {fft_len}"
        )));
    }
    Ok(())
}

/// Checks that the capture's sample rate is positive and finite and its centre
/// frequency is finite, the preconditions of the spectral plots.
pub fn check_rates(capture: &IqCaptureOut) -> Result<()> {
    if !(capture.sample_rate_hz.is_finite() && capture.sample_rate_hz > 0.0) {
        return Err(SimError::InvalidArgument(format!(
            "sample_rate_hz must be positive and finite, got {}",
            capture.sample_rate_hz
        )));
    }
    if !capture.center_frequency_hz.is_finite() {
        return Err(SimError::InvalidArgument(
            "center_frequency_hz must be finite".into(),
        ));
    }
    Ok(())
}

/// Averaged Hann periodogram in dB, natural FFT bin order (bin 0 is DC).
/// Averages over non-overlapping frames of `fft_len` (at least one, zero-padded).
/// Returns an empty vector for no samples or an invalid `fft_len`.
pub(crate) fn power_spectrum_db(samples: &[IqSampleOut], fft_len: usize) -> Vec<f64> {
    if samples.is_empty() || check_fft_len(fft_len).is_err() {
        return Vec::new();
    }
    let frames = (samples.len() / fft_len).max(1);
    let mut engine = Periodogram::new(fft_len);
    let mut acc = vec![0.0; fft_len];
    for frame in 0..frames {
        engine.accumulate(samples, frame * fft_len, &mut acc);
    }
    acc.iter().map(|&p| engine.to_db(p, frames)).collect()
}

/// Index into natural-order bins for position `j` of the FFT-shifted layout.
fn shifted(j: usize, n: usize) -> usize {
    (j + n / 2) % n
}

/// Averaged-periodogram power spectrum (dB) over the capture's bandwidth.
///
/// `fft_len` must be a power of two in `2..=2^20`. Peaks are preserved when the
/// line is decimated for very long FFTs.
pub fn spectrum_svg(capture: &IqCaptureOut, fft_len: usize) -> Result<String> {
    check_fft_len(fft_len)?;
    check_rates(capture)?;
    let fs = capture.sample_rate_hz;
    let center = capture.center_frequency_hz;
    let x_range = (center - fs / 2.0, center + fs / 2.0);
    let db = power_spectrum_db(&capture.samples, fft_len);
    let title = format!("Spectrum - {}", capture.id);
    if db.is_empty() {
        let mut plot = Plot::new(
            WIDTH,
            HEIGHT,
            &title,
            "frequency (Hz)",
            "power (dB)",
            x_range,
            (DB_FLOOR, 0.0),
            false,
        );
        plot.no_data();
        return Ok(plot.finish());
    }
    let bin_hz = fs / fft_len as f64;
    let freq = |j: usize| center + (j as f64 - (fft_len / 2) as f64) * bin_hz;
    let shifted_db: Vec<f64> = (0..fft_len).map(|j| db[shifted(j, fft_len)]).collect();
    let group = fft_len.div_ceil(MAX_SPECTRUM_POINTS);
    let mut line: Vec<(f64, f64)> = Vec::new();
    for (g, chunk) in shifted_db.chunks(group).enumerate() {
        let peak = chunk.iter().copied().fold(DB_FLOOR, f64::max);
        line.push((freq(g * group + chunk.len() / 2), peak));
    }
    let lo = line.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let hi = line.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let mut plot = Plot::new(
        WIDTH,
        HEIGHT,
        &title,
        "frequency (Hz)",
        "power (dB)",
        x_range,
        (lo, hi + (hi - lo) * 0.05),
        false,
    );
    let points: Vec<String> = line
        .iter()
        .map(|&(x, y)| format!("{},{}", f2(plot.px(x)), f2(plot.py(y))))
        .collect();
    let markup = format!(
        "<polyline fill=\"none\" stroke=\"#1f77b4\" stroke-width=\"1\" points=\"{}\"/>\n",
        points.join(" ")
    );
    plot.mark(&markup);
    Ok(plot.finish())
}

/// Viridis-like colour ramp for `t` in `[0, 1]`, as `#rrggbb`.
fn ramp_color(t: f64) -> String {
    const STOPS: [(f64, f64, f64); 5] = [
        (68.0, 1.0, 84.0),
        (59.0, 82.0, 139.0),
        (33.0, 145.0, 140.0),
        (94.0, 201.0, 98.0),
        (253.0, 231.0, 37.0),
    ];
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let scaled = t * (STOPS.len() - 1) as f64;
    let lower = (scaled.floor() as usize).min(STOPS.len() - 2);
    let frac = scaled - lower as f64;
    let (a, b) = (STOPS[lower], STOPS[lower + 1]);
    let channel = |x: f64, y: f64| (x + (y - x) * frac).round().clamp(0.0, 255.0) as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(a.0, b.0),
        channel(a.1, b.1),
        channel(a.2, b.2)
    )
}

/// Smallest decimation factors (frames, bins) that bring the cell count to at
/// most `max_cells`, always coarsening the currently larger dimension.
fn decimation(frames: usize, bins: usize, max_cells: usize) -> (usize, usize) {
    let (mut frame_step, mut bin_step) = (1, 1);
    loop {
        let rows = frames.div_ceil(frame_step);
        let cols = bins.div_ceil(bin_step);
        if rows.saturating_mul(cols) <= max_cells {
            return (frame_step, bin_step);
        }
        if rows > cols {
            frame_step += 1;
        } else {
            bin_step += 1;
        }
    }
}

/// Spectrogram heatmap: one `<rect class="cell">` per (time, frequency) cell.
///
/// Frames are `fft_len` samples every `hop` samples. When there would be more
/// than `max_cells` cells, frames are skipped and bins are merged (peak power)
/// by the smallest integer factors that fit. Colour is normalised to the
/// plot's own dB range.
pub fn spectrogram_svg(
    capture: &IqCaptureOut,
    fft_len: usize,
    hop: usize,
    max_cells: usize,
) -> Result<String> {
    check_fft_len(fft_len)?;
    check_rates(capture)?;
    if hop == 0 {
        return Err(SimError::InvalidArgument("hop must be at least 1".into()));
    }
    if max_cells == 0 {
        return Err(SimError::InvalidArgument(
            "max_cells must be at least 1".into(),
        ));
    }
    let fs = capture.sample_rate_hz;
    let center = capture.center_frequency_hz;
    let x_range = (center - fs / 2.0, center + fs / 2.0);
    let title = format!("Spectrogram - {}", capture.id);
    let samples = &capture.samples;
    if samples.is_empty() {
        let mut plot = Plot::new(
            WIDTH,
            HEIGHT,
            &title,
            "frequency (Hz)",
            "time (s)",
            x_range,
            (0.0, 1.0),
            true,
        );
        plot.no_data();
        return Ok(plot.finish());
    }
    let frames = if samples.len() <= fft_len {
        1
    } else {
        (samples.len() - fft_len).div_ceil(hop) + 1
    };
    let (frame_step, bin_step) = decimation(frames, fft_len, max_cells);
    let rows = frames.div_ceil(frame_step);
    let cols = fft_len.div_ceil(bin_step);

    let mut engine = Periodogram::new(fft_len);
    let mut power = vec![0.0; fft_len];
    let mut grid: Vec<f64> = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        power.iter_mut().for_each(|p| *p = 0.0);
        let start = row.saturating_mul(frame_step).saturating_mul(hop);
        engine.accumulate(samples, start, &mut power);
        let db: Vec<f64> = (0..fft_len)
            .map(|j| engine.to_db(power[shifted(j, fft_len)], 1))
            .collect();
        grid.extend(
            db.chunks(bin_step)
                .map(|c| c.iter().copied().fold(DB_FLOOR, f64::max)),
        );
    }
    let lo = grid.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = grid.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = hi - lo;

    let row_samples = if frames == 1 {
        fft_len
    } else {
        frame_step * hop
    };
    let duration = rows as f64 * row_samples as f64 / fs;
    let mut plot = Plot::new(
        WIDTH,
        HEIGHT,
        &title,
        "frequency (Hz)",
        "time (s)",
        x_range,
        (0.0, duration),
        true,
    );
    plot.note(&format!("{} to {} dB", f2(lo), f2(hi)));
    let (cell_w, cell_h) = (plot.width / cols as f64, plot.height / rows as f64);
    let mut markup = String::new();
    for row in 0..rows {
        for col in 0..cols {
            let t = if span > 1e-9 {
                (grid[row * cols + col] - lo) / span
            } else {
                0.0
            };
            let _ = writeln!(
                markup,
                "<rect class=\"cell\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" shape-rendering=\"crispEdges\"/>",
                f2(plot.left + col as f64 * cell_w),
                f2(plot.top + row as f64 * cell_h),
                f2(cell_w),
                f2(cell_h),
                ramp_color(t),
            );
        }
    }
    plot.mark(&markup);
    Ok(plot.finish())
}

/// Line plot of `(time_s, value)` pairs; points with a non-finite coordinate
/// are skipped. Intended for Doppler, elevation and similar series.
pub fn timeline_svg(title: &str, y_label: &str, series: &[(f64, f64)]) -> String {
    let points: Vec<(f64, f64)> = series
        .iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    let fold = |pick: fn((f64, f64)) -> f64, init: f64, op: fn(f64, f64) -> f64| {
        points.iter().fold(init, |acc, &p| op(acc, pick(p)))
    };
    let x_range = (
        fold(|p| p.0, f64::INFINITY, f64::min),
        fold(|p| p.0, f64::NEG_INFINITY, f64::max),
    );
    let y_range = (
        fold(|p| p.1, f64::INFINITY, f64::min),
        fold(|p| p.1, f64::NEG_INFINITY, f64::max),
    );
    let mut plot = Plot::new(
        WIDTH, HEIGHT, title, "time (s)", y_label, x_range, y_range, false,
    );
    match points.len() {
        0 => plot.no_data(),
        1 => {
            let markup = format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#1f77b4\"/>\n",
                f2(plot.px(points[0].0)),
                f2(plot.py(points[0].1)),
            );
            plot.mark(&markup);
        }
        _ => {
            let coords: Vec<String> = points
                .iter()
                .map(|&(x, y)| format!("{},{}", f2(plot.px(x)), f2(plot.py(y))))
                .collect();
            let markup = format!(
                "<polyline fill=\"none\" stroke=\"#1f77b4\" stroke-width=\"1.5\" points=\"{}\"/>\n",
                coords.join(" ")
            );
            plot.mark(&markup);
        }
    }
    plot.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrum_peak_at_tone() {
        // Complex tone at fs/8 with a 64-point FFT lands exactly in bin 8.
        let samples: Vec<IqSampleOut> = (0..256)
            .map(|k| {
                let phase = 2.0 * PI * k as f64 / 8.0;
                IqSampleOut {
                    i: phase.cos() as f32,
                    q: phase.sin() as f32,
                }
            })
            .collect();
        let db = power_spectrum_db(&samples, 64);
        assert_eq!(db.len(), 64);
        let peak = db
            .iter()
            .enumerate()
            .fold((0, f64::NEG_INFINITY), |best, (k, &v)| {
                if v > best.1 {
                    (k, v)
                } else {
                    best
                }
            });
        assert_eq!(peak.0, 8);
    }

    #[test]
    fn power_spectrum_handles_short_and_empty_input() {
        assert!(power_spectrum_db(&[], 64).is_empty());
        assert!(power_spectrum_db(&[IqSampleOut { i: 1.0, q: 0.0 }], 100).is_empty());
        let one = power_spectrum_db(&[IqSampleOut { i: 1.0, q: 0.0 }], 64);
        assert_eq!(one.len(), 64);
        assert!(one.iter().all(|v| v.is_finite() && *v >= DB_FLOOR));
    }

    #[test]
    fn escape_xml_replaces_xml_illegal_characters() {
        assert_eq!(
            escape_xml("a\u{1}b\u{fffe}c\u{ffff}d"),
            "a\u{fffd}b\u{fffd}c\u{fffd}d"
        );
        // Tab, newline and carriage return are legal XML and kept.
        assert_eq!(escape_xml("a\tb\nc\rd"), "a\tb\nc\rd");
    }

    #[test]
    fn escape_xml_escapes_all_special_characters() {
        assert_eq!(
            escape_xml("a&b<c>d\"e'f"),
            "a&amp;b&lt;c&gt;d&quot;e&apos;f"
        );
        assert_eq!(escape_xml("plain"), "plain");
        assert_eq!(escape_xml(""), "");
    }

    #[test]
    fn ramp_and_decimation_are_total() {
        assert_eq!(ramp_color(f64::NAN), ramp_color(0.0));
        assert_ne!(ramp_color(0.0), ramp_color(1.0));
        assert_eq!(ramp_color(7.0), ramp_color(1.0));
        let (fs, bs) = decimation(1000, 1024, 4096);
        assert!(1000usize.div_ceil(fs) * 1024usize.div_ceil(bs) <= 4096);
        assert_eq!(decimation(3, 4, 12), (1, 1));
    }
}
