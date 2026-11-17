//! Hand-written exporters for common I/Q interchange formats: NumPy `.npy`
//! (format 1.0, `complex64`) and SigMF (`cf32_le`).
//!
//! All formats store each sample as the little-endian `f32` pair `i`, `q`.

use serde_json::{json, Value};

use crate::rf::IqCaptureOut;

const NPY_MAGIC: &[u8] = b"\x93NUMPY\x01\x00";
/// NPY 1.0 pads magic (8) + header length field (2) + header to a multiple of this.
const NPY_ALIGNMENT: usize = 64;

/// Interleaved little-endian `f32` payload shared by `.npy` and `.sigmf-data`.
fn iq_payload(capture: &IqCaptureOut) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(capture.samples.len().saturating_mul(8));
    for sample in &capture.samples {
        bytes.extend_from_slice(&sample.i.to_le_bytes());
        bytes.extend_from_slice(&sample.q.to_le_bytes());
    }
    bytes
}

/// Serializes the capture as a NumPy `.npy` file (format 1.0, `<c8`, shape `(N,)`).
pub fn iq_to_npy_bytes(capture: &IqCaptureOut) -> Vec<u8> {
    let mut header = format!(
        "{{'descr': '<c8', 'fortran_order': False, 'shape': ({},), }}",
        capture.samples.len()
    );
    // magic + length field + header + trailing newline must be 64-aligned.
    let unpadded = NPY_MAGIC.len() + 2 + header.len() + 1;
    let padding = (NPY_ALIGNMENT - unpadded % NPY_ALIGNMENT) % NPY_ALIGNMENT;
    header.extend(std::iter::repeat_n(' ', padding));
    header.push('\n');

    let mut bytes =
        Vec::with_capacity(NPY_MAGIC.len() + 2 + header.len() + capture.samples.len() * 8);
    bytes.extend_from_slice(NPY_MAGIC);
    // The header is a few dozen bytes; saturate rather than panic if that ever changes.
    let header_len = u16::try_from(header.len()).unwrap_or(u16::MAX);
    bytes.extend_from_slice(&header_len.to_le_bytes());
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(&iq_payload(capture));
    bytes
}

/// Serializes the capture as a SigMF `cf32_le` dataset (`.sigmf-data`).
pub fn sigmf_data_bytes(capture: &IqCaptureOut) -> Vec<u8> {
    iq_payload(capture)
}

/// Builds the SigMF metadata (`.sigmf-meta`) for the dataset stored in
/// `data_file_name`. SigMF pairs the two files by shared basename, so the name
/// is not embedded (no `core:dataset`); it is part of the signature so callers
/// state which file the metadata describes. The checksum field is omitted.
pub fn sigmf_meta_json(capture: &IqCaptureOut, _data_file_name: &str) -> Value {
    json!({
        "global": {
            "core:datatype": "cf32_le",
            "core:sample_rate": capture.sample_rate_hz,
            "core:version": "1.0.0"
        },
        "captures": [{
            "core:sample_start": 0,
            "core:frequency": capture.center_frequency_hz
        }],
        "annotations": []
    })
}
