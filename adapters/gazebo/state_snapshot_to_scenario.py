#!/usr/bin/env python3
"""Apply a bounded Gazebo/ROS-exported ENU state snapshot to a spectra-sim scenario."""
from __future__ import annotations
import argparse,json,math
from pathlib import Path


def finite3(v):
    return isinstance(v,list) and len(v)==3 and all(isinstance(x,(int,float)) and math.isfinite(x) for x in v)


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--scenario',required=True); ap.add_argument('--snapshot',required=True); ap.add_argument('--out',required=True); args=ap.parse_args()
    scenario=json.loads(Path(args.scenario).read_text()); snapshot=json.loads(Path(args.snapshot).read_text())
    if snapshot.get('schema_version')!='spectra-gazebo-state-v1': raise SystemExit('unsupported snapshot schema')
    entities=snapshot.get('entities',[])
    if not isinstance(entities,list) or len(entities)>4096: raise SystemExit('invalid entity list')
    by_id={e['id']:e for e in entities if isinstance(e,dict) and isinstance(e.get('id'),str)}
    for collection in ('emitters','receivers','targets'):
        for entity in scenario.get(collection,[]):
            state=by_id.get(entity.get('id'))
            if not state: continue
            p=state.get('position_enu_m'); v=state.get('velocity_enu_mps')
            if not finite3(p) or not finite3(v): raise SystemExit(f'invalid state for {entity.get("id")}')
            entity['state']['position_enu_m']={'x':p[0],'y':p[1],'z':p[2]}
            entity['state']['velocity_enu_mps']={'x':v[0],'y':v[1],'z':v[2]}
            for key in ('roll_deg','pitch_deg','yaw_deg'):
                if key in state:
                    if not isinstance(state[key],(int,float)) or not math.isfinite(state[key]): raise SystemExit(f'invalid {key}')
                    entity['state'][key]=state[key]
    out=Path(args.out); out.parent.mkdir(parents=True,exist_ok=True); out.write_text(json.dumps(scenario,indent=2))

if __name__=='__main__': main()
