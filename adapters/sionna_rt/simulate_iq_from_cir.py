#!/usr/bin/env python3
"""Turn a Sionna RT CIR artifact plus a spectra-sim scenario into bounded raw I/Q JSON."""
from __future__ import annotations
import argparse,json,math
from pathlib import Path
import numpy as np
K=1.380_649e-23
R=50.0


def find(items, key):
    for item in items:
        if item.get('id')==key: return item
    raise SystemExit(f'missing entity {key}')


def waveform(kind, t, w, bandwidth):
    typ=w['kind']
    if typ=='continuous_tone': return np.exp(1j*2*np.pi*w['offset_hz']*t)
    if typ=='pulse_train':
        gate=((t*w['pulse_repetition_hz'])%1.0)<=w['duty_cycle']
        return gate.astype(np.float64)*np.exp(1j*2*np.pi*w['offset_hz']*t)
    if typ=='ofdm':
        n=int(w['subcarriers']); spacing=float(w['subcarrier_spacing_hz'])
        if n<=0 or n>4096 or n*spacing>bandwidth*1.25: raise SystemExit('invalid OFDM dimensions')
        center=(n-1)/2
        out=np.zeros_like(t,dtype=np.complex128)
        for k in range(n):
            freq=(k-center)*spacing
            phase=((k*0x9E3779B9)%65521)/65521*2*np.pi
            out += np.exp(1j*(2*np.pi*freq*t+phase))
        return out/math.sqrt(n)
    raise SystemExit(f'unsupported waveform {typ}')


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--scenario',required=True); ap.add_argument('--cir',required=True); ap.add_argument('--out',required=True); args=ap.parse_args()
    scenario=json.loads(Path(args.scenario).read_text()); cir=json.loads(Path(args.cir).read_text())
    job=scenario.get('sigint_job') or {}; emitter=find(scenario['emitters'],job.get('emitter_id')); receiver=find(scenario['receivers'],job.get('receiver_id'))
    count=int(job.get('samples',0));
    if not 1<=count<=16384: raise SystemExit('sample count out of range')
    fs=float(receiver['sample_rate_hz']); t=np.arange(count,dtype=np.float64)/fs
    x=waveform(emitter['waveform']['kind'],t,emitter['waveform'],float(emitter['bandwidth_hz']))
    real=np.asarray(cir['coefficients_real'],dtype=np.float64); imag=np.asarray(cir['coefficients_imag'],dtype=np.float64); tau=np.asarray(cir['delays_s'],dtype=np.float64)
    a=(real+1j*imag)[0,0,0,0,:,0]; delays=tau[0,0,0,0,:]
    valid=np.isfinite(delays)&np.isfinite(a.real)&np.isfinite(a.imag)
    a=a[valid]; delays=delays[valid]
    if len(a)==0: raise SystemExit('CIR contains no finite path')
    y=np.zeros(count,dtype=np.complex128)
    for coeff,delay in zip(a,delays):
        shift=max(0,int(round(delay*fs)))
        if shift<count: y[shift:] += coeff*x[:count-shift]
    tx_w=10**((float(emitter['tx_power_dbm'])-30)/10)
    path_gain=float(np.sum(np.abs(a)**2)); signal_w=max(tx_w*path_gain,1e-30)
    env=scenario['environment']; bw=min(float(emitter['bandwidth_hz']),fs)
    nf=10**(float(receiver['noise_figure_db'])/10); noise_w=K*float(env['temperature_k'])*bw*nf
    snr=max(signal_w/max(noise_w,1e-30),1e-12)
    rms=float(np.sqrt(np.mean(np.abs(y)**2)))
    target=float(receiver['agc_target_rms'])
    if rms>1e-15: y*=target/rms
    rng=np.random.default_rng(int(scenario['seed']))
    noise_rms=target/math.sqrt(snr)
    noise=(rng.normal(0,noise_rms/math.sqrt(2),count)+1j*rng.normal(0,noise_rms/math.sqrt(2),count))
    ppm=float(receiver['oscillator_ppm']); cfo=float(emitter['center_frequency_hz'])*ppm*1e-6
    phase_walk=np.cumsum(rng.normal(0,float(receiver['phase_noise_rad_std']),count))
    # Match the Rust core ordering: carrier/CFO and phase noise act on the
    # signal, then receiver thermal noise is added, then IQ imbalance acts on
    # the complete complex baseband.
    y=y*np.exp(1j*(2*np.pi*cfo*t+phase_walk))+noise
    gain_root=math.sqrt(10**(float(receiver['iq_gain_imbalance_db'])/20))
    half=math.radians(float(receiver.get('iq_phase_imbalance_deg',0.0)))/2
    ci,si=math.cos(half),math.sin(half)
    y=gain_root*(y.real*ci+y.imag*si)+1j*((y.imag*ci+y.real*si)/gain_root)
    bits=int(receiver['adc_bits']); qmax=(1<<(bits-1))-1
    i=np.round(np.clip(y.real,-0.999999,0.999999)*qmax)/qmax; q=np.round(np.clip(y.imag,-0.999999,0.999999)*qmax)/qmax
    payload={
        'id':f"sionna-{emitter['id']}-{receiver['id']}-{scenario['timestamp_ms']}",
        'timestamp_ms':int(scenario['timestamp_ms']),
        'sample_rate_hz':fs,
        'center_frequency_hz':float(emitter['center_frequency_hz']),
        'full_scale_v':float(receiver['full_scale_v']),
        'adc_bits':bits,
        'samples':[{'i':float(a),'q':float(b)} for a,b in zip(i,q)],
    }
    out=Path(args.out); out.parent.mkdir(parents=True,exist_ok=True); out.write_text(json.dumps(payload,indent=2))

if __name__=='__main__': main()
