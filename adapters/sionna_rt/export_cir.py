#!/usr/bin/env python3
"""Export a reproducible Sionna RT channel impulse response as JSON.

This adapter is optional and out-of-process. It requires Sionna RT 2.0.1.
"""
from __future__ import annotations
import argparse, json
from pathlib import Path
import numpy as np


def vec3(text: str) -> list[float]:
    values=[float(x) for x in text.split(',')]
    if len(values)!=3 or not all(np.isfinite(values)):
        raise argparse.ArgumentTypeError('expected finite x,y,z')
    return values


def main() -> None:
    ap=argparse.ArgumentParser()
    ap.add_argument('--scene', required=True, help='Mitsuba XML scene path')
    ap.add_argument('--frequency-hz', type=float, required=True)
    ap.add_argument('--tx', type=vec3, required=True)
    ap.add_argument('--rx', type=vec3, required=True)
    ap.add_argument('--tx-velocity', type=vec3, default=[0.0,0.0,0.0])
    ap.add_argument('--rx-velocity', type=vec3, default=[0.0,0.0,0.0])
    ap.add_argument('--max-depth', type=int, default=4)
    ap.add_argument('--seed', type=int, default=41)
    ap.add_argument('--out', required=True)
    args=ap.parse_args()
    if not np.isfinite(args.frequency_hz) or args.frequency_hz <= 0:
        raise SystemExit('frequency must be positive')
    if not 0 <= args.max_depth <= 16:
        raise SystemExit('max-depth must be in 0..=16')

    import sionna.rt
    from sionna.rt import load_scene, PlanarArray, Transmitter, Receiver, PathSolver

    scene=load_scene(args.scene)
    scene.frequency=args.frequency_hz
    scene.tx_array=PlanarArray(num_rows=1,num_cols=1,vertical_spacing=0.5,horizontal_spacing=0.5,pattern='tr38901',polarization='V')
    scene.rx_array=PlanarArray(num_rows=1,num_cols=1,vertical_spacing=0.5,horizontal_spacing=0.5,pattern='dipole',polarization='V')
    tx=Transmitter(name='spectra_tx',position=args.tx)
    rx=Receiver(name='spectra_rx',position=args.rx)
    tx.velocity=args.tx_velocity
    rx.velocity=args.rx_velocity
    scene.add(tx); scene.add(rx)
    tx.look_at(rx)
    solver=PathSolver()
    paths=solver(scene=scene,max_depth=args.max_depth,los=True,specular_reflection=True,diffuse_reflection=False,refraction=True,synthetic_array=True,seed=args.seed)
    a,tau=paths.cir(normalize_delays=False,out_type='numpy')
    payload={
        'schema_version':'spectra-sim-sionna-cir-v1',
        'frequency_hz':args.frequency_hz,
        'seed':args.seed,
        'tx_position_m':args.tx,
        'rx_position_m':args.rx,
        'coefficients_real':np.asarray(a.real).tolist(),
        'coefficients_imag':np.asarray(a.imag).tolist(),
        'delays_s':np.asarray(tau).tolist(),
    }
    doppler=getattr(paths,'doppler',None)
    if doppler is not None:
        try: payload['doppler_hz']=np.asarray(doppler).tolist()
        except Exception: pass
    out=Path(args.out); out.parent.mkdir(parents=True,exist_ok=True)
    out.write_text(json.dumps(payload,indent=2))

if __name__=='__main__': main()
