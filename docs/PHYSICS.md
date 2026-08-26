# Physics models and assumptions

## RF propagation

The analytical engine implements Friis free-space loss and an optional two-ray ground-reflection approximation. A seeded Rician mode adds a specular component, complex Gaussian scattering and log-normal shadowing.

The Rician and two-ray modes are useful for fast statistical coverage, training and regression. They are not substitutes for site-specific ray tracing.

## Noise and receiver chain

Thermal noise is computed from Boltzmann's constant, absolute temperature and occupied bandwidth, with receiver noise figure added in dB. The reference receiver then applies:

1. deterministic AGC preserving the simulated SNR;
2. oscillator ppm error;
3. phase-noise random walk;
4. I/Q gain and phase imbalance;
5. additive complex Gaussian noise;
6. bounded ADC quantization.

## Doppler

One-way RF links use relative radial velocity divided by wavelength. Monostatic radar uses the round-trip factor of two.

## Radar

The monostatic radar power estimate follows the classical radar range equation with explicit transmit power, transmit / receive gain, wavelength, target RCS, range to the fourth power, and system losses.

Range resolution is approximated by `c/(2B)`. Radial-velocity resolution uses wavelength and coherent processing time. ULA angular resolution is an aperture approximation and should not be interpreted as a calibrated estimator covariance.

## Micro-Doppler

For rotating targets, the reference model derives rotor-tip speed from rotor radius and RPM, applies a simple rotor-plane aspect weighting, and converts the result to a round-trip Doppler span. This is deliberately a compact model: blade scattering, blade flex, harmonics, body/rotor occlusion and calibrated aspect-dependent RCS require a high-fidelity electromagnetic backend or measurement calibration.

## 5G / OFDM ISAC

The ISAC layer derives sensing observables from carrier frequency, occupied bandwidth, subcarrier spacing, coherent OFDM symbol count and array aperture. The Rust implementation is intentionally waveform-level analytical physics, not a complete 3GPP RAN simulator.

5G-LENA can be used to provide realistic NR configuration and network dynamics, while Sionna RT can provide site-specific radio propagation. Neither is required by the core.
