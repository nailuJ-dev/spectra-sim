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
target/spectra-orekit-sidecar-0.4.0.jar
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
- `parse_oem_summary`
- `parse_ocm_summary`

Example:

```bash
printf '%s\n' '{"op":"health"}' | \
  OREKIT_DATA_DIR=/path/to/orekit-data \
  java -jar target/spectra-orekit-sidecar-0.4.0.jar
```
