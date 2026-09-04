use serde::{Deserialize, Serialize};

use crate::{
    db_to_linear, effective_directional_gain_dbi, link_budget, thermal_noise_dbm, Complex64,
    DeterministicRng, Emitter, Environment, PropagationModel, Receiver, Result, SimError, Waveform,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IqSampleOut {
    pub i: f32,
    pub q: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IqCaptureOut {
    pub id: String,
    pub timestamp_ms: u64,
    pub sample_rate_hz: f64,
    pub center_frequency_hz: f64,
    /// ADC full-scale voltage represented by a normalized sample magnitude of 1.
    pub full_scale_v: f64,
    /// ADC resolution used by the quantizer.
    pub adc_bits: u8,
    pub samples: Vec<IqSampleOut>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SigintTruth {
    pub emitter_id: String,
    pub receiver_id: String,
    pub distance_m: f64,
    pub path_loss_db: f64,
    pub received_power_dbm: f64,
    pub thermal_noise_dbm: f64,
    pub snr_db: f64,
    pub radial_velocity_mps: f64,
    pub doppler_hz: f64,
    pub oscillator_offset_hz: f64,
    pub tx_effective_gain_dbi: f64,
    pub rx_effective_gain_dbi: f64,
    /// Fraction of generated complex samples for which either component reached
    /// or exceeded the normalized ADC rail before quantization.
    pub clipped_sample_fraction: f32,
    pub seed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SigintSimulation {
    pub capture: IqCaptureOut,
    pub truth: SigintTruth,
}

#[allow(clippy::too_many_arguments)]
pub fn simulate_iq(
    emitter: &Emitter,
    receiver: &Receiver,
    environment: Environment,
    propagation: PropagationModel,
    timestamp_ms: u64,
    sample_count: usize,
    seed: u64,
) -> Result<SigintSimulation> {
    emitter.validate()?;
    receiver.validate()?;
    environment.validate()?;
    propagation.validate()?;
    if sample_count == 0 || sample_count > 16_384 {
        return Err(SimError::DimensionLimit {
            actual: sample_count,
            max: 16_384,
        });
    }

    let tx_effective_gain_dbi = effective_directional_gain_dbi(
        &emitter.antenna,
        emitter.state,
        receiver.state.position_enu_m,
        emitter.center_frequency_hz,
    )? - emitter.antenna.cable_loss_db
        - emitter.antenna.polarization_loss_db;
    let rx_effective_gain_dbi = effective_directional_gain_dbi(
        &receiver.antenna,
        receiver.state,
        emitter.state.position_enu_m,
        emitter.center_frequency_hz,
    )? - receiver.antenna.cable_loss_db
        - receiver.antenna.polarization_loss_db;

    let mut rng = DeterministicRng::new(seed);
    let budget = link_budget(
        emitter.tx_power_dbm,
        tx_effective_gain_dbi,
        rx_effective_gain_dbi,
        emitter.state,
        receiver.state,
        emitter.center_frequency_hz,
        environment,
        propagation,
        &mut rng,
    )?;
    let noise_dbm = thermal_noise_dbm(
        emitter.bandwidth_hz.min(receiver.sample_rate_hz),
        environment.temperature_k,
        receiver.noise_figure_db,
    )?;
    let snr_db = budget.received_power_dbm - noise_dbm;
    let signal_to_noise = db_to_linear(snr_db);
    if !signal_to_noise.is_finite() || signal_to_noise <= 0.0 {
        return Err(SimError::InvalidArgument(
            "link SNR is not representable as a positive finite ratio".into(),
        ));
    }
    let target_rms = receiver.agc_target_rms;

    // Pass 1: evaluate the deterministic baseband envelope and measure its
    // actual mean power. Duty-cycled and finite-window waveforms must not be
    // mistaken for unit-RMS signals.
    let mut envelope = Vec::with_capacity(sample_count);
    let mut mean_power = 0.0_f64;
    for index in 0..sample_count {
        let t = index as f64 / receiver.sample_rate_hz;
        let base = waveform_sample(&emitter.waveform, t, emitter.bandwidth_hz)?;
        mean_power += base.magnitude_squared();
        envelope.push(base);
    }
    mean_power /= sample_count as f64;
    if !mean_power.is_finite() {
        return Err(SimError::NonFinite);
    }
    if mean_power <= 1e-12 {
        return Err(SimError::InvalidArgument(
            "waveform produced no measurable power over the requested capture window".into(),
        ));
    }

    let agc_gain = target_rms / mean_power.sqrt();
    let noise_rms = target_rms / signal_to_noise.sqrt();
    let noise_component_std = noise_rms / 2.0_f64.sqrt();
    let oscillator_offset = emitter.center_frequency_hz * receiver.oscillator_ppm * 1e-6;
    let frequency_offset = budget.doppler_hz + oscillator_offset;
    let gain_ratio = 10.0_f64.powf(receiver.iq_gain_imbalance_db / 20.0);
    let gain_root = gain_ratio.sqrt();
    let (half_sin, half_cos) = (0.5 * receiver.iq_phase_imbalance_deg.to_radians()).sin_cos();
    let quant_max = ((1_u64 << (receiver.adc_bits - 1)) - 1) as f64;
    let mut phase_noise = 0.0_f64;
    let mut samples = Vec::with_capacity(sample_count);
    let mut clipped = 0usize;

    // Pass 2: carrier/CFO, phase noise, thermal noise, front-end IQ imbalance,
    // saturation accounting and quantization. The pseudo-random draw order is
    // stable for a fixed input and seed.
    for (index, base) in envelope.into_iter().enumerate() {
        let t = index as f64 / receiver.sample_rate_hz;
        phase_noise += rng.normal(0.0, receiver.phase_noise_rad_std)?;
        let phase = std::f64::consts::TAU * frequency_offset * t
            + phase_noise
            + budget.complex_channel.phase();
        let carrier = Complex64::from_polar(agc_gain, phase);
        let mut value = base * carrier;
        value.re += rng.normal(0.0, noise_component_std)?;
        value.im += rng.normal(0.0, noise_component_std)?;

        // Symmetric gain/quadrature skew. A phase-only defect must generate
        // cross-coupling between I and Q; a scalar cos(phi) on Q is not enough.
        let i_raw = gain_root * value.re.mul_add(half_cos, value.im * half_sin);
        let q_raw = value.im.mul_add(half_cos, value.re * half_sin) / gain_root;
        if i_raw.abs() >= 1.0 || q_raw.abs() >= 1.0 {
            clipped += 1;
        }
        let i = quantize(i_raw, quant_max);
        let q = quantize(q_raw, quant_max);
        samples.push(IqSampleOut {
            i: i as f32,
            q: q as f32,
        });
    }
    let clipped_sample_fraction = (clipped as f64 / sample_count as f64) as f32;

    Ok(SigintSimulation {
        capture: IqCaptureOut {
            id: format!("sim-{}-{}-{timestamp_ms}", emitter.id, receiver.id),
            timestamp_ms,
            sample_rate_hz: receiver.sample_rate_hz,
            center_frequency_hz: emitter.center_frequency_hz,
            full_scale_v: receiver.full_scale_v,
            adc_bits: receiver.adc_bits,
            samples,
        },
        truth: SigintTruth {
            emitter_id: emitter.id.clone(),
            receiver_id: receiver.id.clone(),
            distance_m: budget.distance_m,
            path_loss_db: budget.path_loss_db,
            received_power_dbm: budget.received_power_dbm,
            thermal_noise_dbm: noise_dbm,
            snr_db,
            radial_velocity_mps: budget.radial_velocity_mps,
            doppler_hz: budget.doppler_hz,
            oscillator_offset_hz: oscillator_offset,
            tx_effective_gain_dbi,
            rx_effective_gain_dbi,
            clipped_sample_fraction,
            seed,
        },
    })
}

fn waveform_sample(waveform: &Waveform, t: f64, bandwidth_hz: f64) -> Result<Complex64> {
    match waveform {
        Waveform::ContinuousTone { offset_hz } => Ok(Complex64::from_polar(
            1.0,
            std::f64::consts::TAU * offset_hz * t,
        )),
        Waveform::PulseTrain {
            offset_hz,
            pulse_repetition_hz,
            duty_cycle,
        } => {
            let phase = (t * pulse_repetition_hz).fract();
            if phase <= *duty_cycle {
                Ok(Complex64::from_polar(
                    1.0,
                    std::f64::consts::TAU * offset_hz * t,
                ))
            } else {
                Ok(Complex64::ZERO)
            }
        }
        Waveform::Ofdm {
            subcarriers,
            subcarrier_spacing_hz,
        } => {
            let occupied = *subcarriers as f64 * *subcarrier_spacing_hz;
            if occupied > bandwidth_hz * 1.25 {
                return Err(SimError::InvalidArgument(
                    "OFDM occupied bandwidth exceeds emitter bandwidth envelope".into(),
                ));
            }
            let mut value = Complex64::ZERO;
            let center = (*subcarriers as f64 - 1.0) / 2.0;
            for k in 0..*subcarriers {
                let freq = (k as f64 - center) * *subcarrier_spacing_hz;
                let deterministic_phase = ((k as u64).wrapping_mul(0x9E37_79B9) % 65_521) as f64
                    / 65_521.0
                    * std::f64::consts::TAU;
                value = value
                    + Complex64::from_polar(
                        1.0,
                        std::f64::consts::TAU * freq * t + deterministic_phase,
                    );
            }
            Ok(value.scale(1.0 / (*subcarriers as f64).sqrt()))
        }
    }
}

fn quantize(value: f64, quant_max: f64) -> f64 {
    let clipped = value.clamp(-0.999_999, 0.999_999);
    (clipped * quant_max).round() / quant_max
}
