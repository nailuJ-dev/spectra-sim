# Validation strategy

A credible simulation program uses a validation ladder:

1. **closed-form unit checks** — Friis, kTB, Doppler and radar equation;
2. **deterministic replay** — same scenario + seed produces identical bytes and digests;
3. **cross-model comparison** — analytical versus ray-traced path loss / delay / Doppler;
4. **hardware-in-the-loop** — inject generated I/Q / measurement frames through representative acquisition software;
5. **measurement calibration** — fit materials, RCS, receiver impairments and clutter to controlled measurements;
6. **field holdout** — assess models on recordings never used for simulator calibration.

Synthetic-only metrics must always be labeled synthetic. A simulator may demonstrate software correctness, robustness trends and controlled failure modes; it cannot by itself establish field detection/classification accuracy.
