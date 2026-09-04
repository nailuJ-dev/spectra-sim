# Space implementation audit — 0.4.1 remediation

## Requirement coverage

| Requirement | Native implementation | Precision/reference path |
|---|---|---|
| TLE parsing | `Tle` via sgp4 2.4.0 | Orekit TLE |
| OMM parsing | JSON/KVN/XML `OmmMessage` | Orekit when required |
| SGP4/SDP4 | `Sgp4Propagator` | Orekit TLEPropagator |
| CCSDS OEM | KVN/XML, segments, P/V/A states, covariance | Orekit full Cartesian state oracle |
| CCSDS OCM | KVN/XML TRAJ blocks, typed Cartesian + raw non-Cartesian preservation | Orekit Cartesian trajectory oracle |
| EOP | C04 20u24 + strict finals2000A | orekit-data |
| GCRF/ITRF | SOFA IAU 2006/2000A + dX/dY + polar motion/UT1 | Orekit IERS 2010 |
| TEME/ITRF | Vallado GMST82 + polar motion + LOD | Orekit |
| EME2000/GCRF | intentionally rejected natively | Orekit |
| Atmospheric gas | P.676-13 line-by-line | external comparison vectors |
| Clouds | P.840-9 coefficient/slant model | external comparison vectors |
| Rain/scintillation | explicit P.838/P.618 inputs, no invented climatology | authoritative external datasets |
| Ionosphere | first-order TEC f^-2 + Faraday utility | caller/oracle comparison |
| Antenna/pointing | WGS-84/ENU + Gaussian/tabulated/dish | Orekit transforms as reference |
| Link budget | FSPL/EIRP/G/T/CN0/CN/EbN0/margin | analytical reference |
| Orekit backend | JSON-lines Java sidecar, local data only | Orekit 13.1.8 |

## Regression risks explicitly covered

- Canonical Vallado SGP4 state vector.
- Negative versus positive SGP4 propagation time.
- OMM integer-field deserialization.
- OEM KVN multi-segment and OEM XML.
- OCM standard XML `trajLine` structure and non-Cartesian preservation.
- C04 interpolation bounds.
- finals2000A fixed columns, prediction-tail row skipping without zero-fill, and malformed-field rejection.
- GCRF↔ITRF and TEME↔ITRF round trips.
- dX/dY actually affecting the celestial-to-terrestrial transform.
- Ionospheric f^-2 scaling.
- P.676 positive finite attenuation.
- P.840-9 cloud coefficient/slant relation.
- Signed Doppler for approaching/receding geometry in relative and absolute solvers.
- WGS-84 visibility, ECEF/geodetic conversion, attitude-driven directional antenna gain and radar/ISAC propagation behavior.

## Known deliberate boundaries

- Native frame velocity transformation treats EOP as constant at the requested instant and applies the dominant Earth-rotation term. Use Orekit for the highest precision dynamical frame-rate treatment.
- Full global P.618 rain/scintillation availability requires climatological maps and site statistics. The native API does not fabricate them.
- OCM contains many optional orbit-element representations. All trajectory lines are retained; only CARTPV/CARTPVA are converted into SI Cartesian states natively.
- This generation environment has no Rust/Cargo and no Maven. Therefore compile/test claims must come from the user's local gates, not from this audit.
