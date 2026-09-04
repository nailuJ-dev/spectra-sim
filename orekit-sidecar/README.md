# spectra-sim Orekit precision sidecar

Pinned backend: **Orekit 13.1.8**, Java 17+.

The sidecar communicates via one JSON object per line on stdin/stdout. It intentionally clears default Orekit data providers and installs only the directory specified by `OREKIT_DATA_DIR`; runtime HTTP/network fetching is not configured.

## Build

```bash
cd orekit-sidecar
mvn -DskipTests package
```

Resulting shaded JAR:

```text
target/spectra-orekit-sidecar-0.4.1.jar
```

## Required data snapshot

Set a local, versioned Orekit data directory:

```bash
export OREKIT_DATA_DIR=/absolute/path/to/pinned/orekit-data
```

For reproducibility, compute its digest with `spectra_sim::sha256_directory` and record it in `SpaceProvenance`.

## Supported operations

- `health`
- `propagate_tle`
- `transform_state` for GCRF/ICRF, TEME, EME2000/J2000 and ITRF
- `parse_oem` — full segment metadata and Cartesian P/V/A state histories
- `parse_ocm` — trajectory blocks converted by Orekit to Cartesian P/V/A state histories
- `parse_oem_summary` — compatibility summary
- `parse_ocm_summary` — compatibility summary

Example:

```bash
printf '%s\n' '{"op":"health"}' | \
  OREKIT_DATA_DIR=/path/to/orekit-data \
  java -jar target/spectra-orekit-sidecar-0.4.1.jar
```


## Precision-oracle role

`parse_oem` and `parse_ocm` intentionally return the states produced by Orekit rather than only parser counts. This makes the sidecar usable as an independent oracle for native CCSDS parsing and frame/state validation. Startup failures are emitted both as protocol-shaped JSON and concise stderr; no runtime network data provider is installed.
