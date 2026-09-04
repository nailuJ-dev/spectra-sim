#!/usr/bin/env python3
"""Convert a bounded NR trace row (for example from ns-3/5G-LENA) into IsacConfig JSON."""
from __future__ import annotations
import argparse,csv,json,math
from pathlib import Path
FIELDS=['carrier_frequency_hz','bandwidth_hz','subcarrier_spacing_hz','symbols','array_elements','element_spacing_lambda','tx_power_dbm','tx_gain_dbi','rx_gain_dbi','noise_figure_db','system_loss_db']

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--trace',required=True); ap.add_argument('--row',type=int,default=-1); ap.add_argument('--out',required=True); args=ap.parse_args()
    rows=list(csv.DictReader(Path(args.trace).open(newline='')))
    if not rows or len(rows)>1_000_000: raise SystemExit('trace is empty or too large')
    if not -len(rows)<=args.row<len(rows): raise SystemExit(f'--row {args.row} is outside 0..{len(rows)-1}')
    row=rows[args.row]
    out={}
    for name in FIELDS:
        if name not in row: raise SystemExit(f'missing {name}')
        value=float(row[name])
        if not math.isfinite(value): raise SystemExit(f'non-finite {name}')
        out[name]=int(value) if name in ('symbols','array_elements') else value
    p=Path(args.out); p.parent.mkdir(parents=True,exist_ok=True); p.write_text(json.dumps(out,indent=2))

if __name__=='__main__': main()
