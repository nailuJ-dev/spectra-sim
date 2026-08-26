# 5G-LENA / ns-3 bridge

This adapter treats 5G-LENA as a source of realistic NR network configuration and timing, not as an electromagnetic radar solver.

A bounded CSV trace can carry the carrier, bandwidth, numerology, coherent symbol count, array geometry and radio power budget. Convert one row with:

```bash
python adapters/5g_lena/trace_to_isac_config.py \
  --trace adapters/5g_lena/example_nr_trace.csv \
  --row -1 \
  --out artifacts/nr/isac_config.json
```

The resulting object uses the same fields as `IsacConfig` in a spectra-sim scenario. The electromagnetic sensing response is still produced by spectra-sim or a ray-traced backend.
