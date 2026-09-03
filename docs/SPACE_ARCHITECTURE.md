# spectra-sim 0.4 space architecture

## Data flow

```text
TLE / OMM --------------------> SGP4 --------------------> TEME AbsoluteOrbitState
                                                         |
OEM --------------------------> AbsoluteEphemeris        |
                                                         v
OCM --> typed core + raw extension preservation     EOP-backed frames
                                                         |
                              +--------------------------+--------------------+
                              |                                               |
                              v                                               v
                     Ground-station geometry                         absolute link geometry
                              |                                               |
                              v                                               v
                     antenna pointing/gain                          range/range-rate/Doppler
                              +--------------------------+--------------------+
                                                         |
                                                         v
                                           Earth-space corrections
                                           P.676 / P.840 / ionosphere
                                                         |
                                                         v
                                                   link budget
```

Orekit is deliberately outside the native computational core. It is used as a precision/reference backend for propagation, difficult frame pairs and independent CCSDS validation.

## Determinism contract

1. All external data is caller supplied.
2. EOP lookup is bounded; no silent extrapolation.
3. `orekit-data` is a local snapshot and can be tree-hashed.
4. SGP4/SOFA/XML dependency versions are pinned.
5. Unit conversion happens at parser boundaries; internal Cartesian states are SI.
6. Unsupported OCM element types are retained losslessly rather than coerced.
7. Relative simulation time and absolute astronomical time are distinct types.
