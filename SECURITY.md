# Security

The Rust core is offline by design. It performs no network requests, dynamic code loading or shell execution.

Security controls:

- `#![forbid(unsafe_code)]`;
- bounded entity, file, sample and array dimensions;
- finite-number validation before simulation;
- deterministic seeded randomness;
- JSON schemas reject unknown fields on core contracts;
- high-fidelity adapters are optional, separate processes;
- simulator truth and model-consumable outputs are separated to reduce accidental label leakage.

Please report vulnerabilities privately to the repository maintainer before public disclosure.
