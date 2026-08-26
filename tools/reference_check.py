#!/usr/bin/env python3
"""Dependency-free numerical smoke checks for the documented analytical equations."""
from __future__ import annotations
import json, math
from pathlib import Path

C=299_792_458.0
K=1.380_649e-23

def fspl(f,d): return 20*math.log10(4*math.pi*d/(C/f))
def noise_dbm(bw,t=290.0,nf=0.0): return 10*math.log10(K*t*bw*1000)+nf

def main():
    assert abs(fspl(1e9,1)-32.4478) < 0.02
    assert abs(noise_dbm(1e6)+113.98) < 0.2
    scenario=json.loads(Path('examples/scenarios/cuas_drone_isac.json').read_text())
    radar=scenario['cuas_job']['radar']
    target=scenario['targets'][0]
    sensor=scenario['receivers'][0]
    p=target['state']['position_enu_m']; s=sensor['state']['position_enu_m']
    r=math.sqrt(sum((p[k]-s[k])**2 for k in ('x','y','z')))
    range_res=C/(2*radar['bandwidth_hz'])
    assert 100 < r < 500
    assert 1.0 < range_res < 2.0
    print(json.dumps({'status':'ok','range_m':r,'range_resolution_m':range_res,'fspl_1ghz_1m_db':fspl(1e9,1)},indent=2))

if __name__=='__main__': main()
