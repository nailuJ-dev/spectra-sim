use spectra_sim::{run_cuas, run_sigint, GoldenCuasSourceOut, Scenario};

fn fixture(path: &str) -> Scenario {
    let bytes = std::fs::read(path).unwrap_or_default();
    serde_json::from_slice(&bytes).unwrap_or_else(|_| panic!("valid fixture"))
}

#[test]
fn sigint_export_matches_public_iq_contract() {
    let run = run_sigint(&fixture("examples/scenarios/sigint_urban.json"))
        .unwrap_or_else(|_| panic!("sigint simulation"));
    let value = serde_json::to_value(&run.simulation.capture).unwrap_or_default();
    assert!(value.get("sample_rate_hz").is_some());
    assert!(value.get("center_frequency_hz").is_some());
    assert!(value.get("full_scale_v").is_some());
    assert!(value.get("adc_bits").is_some());
    assert!(value
        .get("samples")
        .and_then(|v| v.as_array())
        .is_some_and(|v| !v.is_empty()));
}

#[test]
fn cuas_export_has_thirteen_isac_features() {
    let run = run_cuas(&fixture("examples/scenarios/cuas_drone_isac.json"))
        .unwrap_or_else(|_| panic!("cuas simulation"));
    let scenario = run.isac_scenario.unwrap_or_else(|| panic!("isac scenario"));
    match scenario.source {
        GoldenCuasSourceOut::IsacRecorded { input } => assert_eq!(input.features.len(), 13),
        _ => panic!("expected isac source"),
    }
}

// ---- .npy and SigMF export ----

mod formats_export {
    use spectra_sim::formats::{iq_to_npy_bytes, sigmf_data_bytes, sigmf_meta_json};
    use spectra_sim::{IqCaptureOut, IqSampleOut};
    use std::process::Command;

    fn capture(n: usize) -> IqCaptureOut {
        IqCaptureOut {
            id: "cap".to_string(),
            timestamp_ms: 7,
            sample_rate_hz: 2_000_000.0,
            center_frequency_hz: 915_000_000.0,
            full_scale_v: 1.0,
            adc_bits: 12,
            samples: (0..n)
                .map(|k| IqSampleOut {
                    i: k as f32 * 0.5 + 0.25,
                    q: -(k as f32) - 1.5,
                })
                .collect(),
        }
    }

    fn header_len(bytes: &[u8]) -> usize {
        u16::from_le_bytes([bytes[8], bytes[9]]) as usize
    }

    fn header_text(bytes: &[u8]) -> String {
        String::from_utf8(bytes[10..10 + header_len(bytes)].to_vec()).expect("ascii header")
    }

    #[test]
    fn npy_header_is_valid() {
        let bytes = iq_to_npy_bytes(&capture(3));
        assert_eq!(&bytes[..8], b"\x93NUMPY\x01\x00");
        assert_eq!((10 + header_len(&bytes)) % 64, 0);
        let header = header_text(&bytes);
        assert_eq!(
            header.trim_end(),
            "{'descr': '<c8', 'fortran_order': False, 'shape': (3,), }"
        );
        assert!(header.contains("(3,)"));
        assert!(header.ends_with('\n'));
        assert_eq!(bytes.len(), 10 + header_len(&bytes) + 3 * 8);
    }

    #[test]
    fn npy_payload_round_trips() {
        let cap = capture(5);
        let bytes = iq_to_npy_bytes(&cap);
        let payload = &bytes[10 + header_len(&bytes)..];
        assert_eq!(payload.len(), cap.samples.len() * 8);
        for (k, sample) in cap.samples.iter().enumerate() {
            let at = k * 8;
            let i = f32::from_le_bytes(payload[at..at + 4].try_into().expect("4 bytes"));
            let q = f32::from_le_bytes(payload[at + 4..at + 8].try_into().expect("4 bytes"));
            assert_eq!(i, sample.i);
            assert_eq!(q, sample.q);
        }
        assert_eq!(payload, sigmf_data_bytes(&cap).as_slice());
    }

    #[test]
    fn npy_empty_and_single_sample() {
        let empty = iq_to_npy_bytes(&capture(0));
        assert!(header_text(&empty).contains("(0,)"));
        assert_eq!(empty.len(), 10 + header_len(&empty));
        assert_eq!((10 + header_len(&empty)) % 64, 0);
        assert!(sigmf_data_bytes(&capture(0)).is_empty());

        let one = iq_to_npy_bytes(&capture(1));
        assert!(header_text(&one).contains("(1,)"));
        assert_eq!(one.len(), 10 + header_len(&one) + 8);
    }

    #[test]
    fn npy_header_n_is_exact_for_large_counts() {
        let bytes = iq_to_npy_bytes(&capture(1000));
        assert!(header_text(&bytes).contains("(1000,)"));
        assert_eq!((10 + header_len(&bytes)) % 64, 0);
        assert_eq!(bytes.len() - 10 - header_len(&bytes), 8000);
    }

    #[test]
    fn sigmf_meta_fields() {
        let meta = sigmf_meta_json(&capture(2), "iq_capture.sigmf-data");
        let expected = serde_json::json!({
            "global": {
                "core:datatype": "cf32_le",
                "core:sample_rate": 2_000_000.0,
                "core:version": "1.0.0"
            },
            "captures": [{"core:sample_start": 0, "core:frequency": 915_000_000.0}],
            "annotations": []
        });
        assert_eq!(meta, expected);
        assert!(meta["global"].get("core:sha512").is_none());
    }

    fn run_cli(args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_spectra-sim"))
            .args(args)
            .output()
            .expect("run binary")
    }

    fn scenario_path() -> String {
        format!(
            "{}/examples/scenarios/sigint_urban.json",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    fn temp_dir(test: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("spectra-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn cli_also_flag_writes_files() {
        let dir = temp_dir("also-both");
        let out = run_cli(&[
            "sigint",
            &scenario_path(),
            "--out",
            dir.to_str().expect("utf8"),
            "--also",
            "npy,sigmf",
        ]);
        assert!(out.status.success(), "{out:?}");
        for name in [
            "iq_capture.json",
            "iq_capture.npy",
            "iq_capture.sigmf-data",
            "iq_capture.sigmf-meta",
        ] {
            assert!(dir.join(name).is_file(), "missing {name}");
        }
        let json: IqCaptureOut =
            serde_json::from_slice(&std::fs::read(dir.join("iq_capture.json")).expect("json"))
                .expect("parse capture");
        assert_eq!(
            std::fs::read(dir.join("iq_capture.npy")).expect("npy"),
            iq_to_npy_bytes(&json)
        );
        assert_eq!(
            std::fs::read(dir.join("iq_capture.sigmf-data")).expect("data"),
            sigmf_data_bytes(&json)
        );
        let meta: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("iq_capture.sigmf-meta")).expect("m"))
                .expect("meta json");
        assert_eq!(meta, sigmf_meta_json(&json, "iq_capture.sigmf-data"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cli_also_default_and_single_format() {
        let dir = temp_dir("also-none");
        let out = run_cli(&[
            "sigint",
            &scenario_path(),
            "--out",
            dir.to_str().expect("utf8"),
        ]);
        assert!(out.status.success());
        assert!(!dir.join("iq_capture.npy").exists());
        assert!(!dir.join("iq_capture.sigmf-meta").exists());
        let _ = std::fs::remove_dir_all(&dir);

        let dir = temp_dir("also-npy-dup");
        let out = run_cli(&[
            "sigint",
            &scenario_path(),
            "--out",
            dir.to_str().expect("utf8"),
            "--also",
            "npy,npy",
            "--also",
            "npy",
        ]);
        assert!(out.status.success(), "{out:?}");
        assert!(dir.join("iq_capture.npy").is_file());
        assert!(!dir.join("iq_capture.sigmf-data").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cli_also_rejects_unknown_value() {
        let dir = temp_dir("also-bad");
        let out = run_cli(&[
            "sigint",
            &scenario_path(),
            "--out",
            dir.to_str().expect("utf8"),
            "--also",
            "wav",
        ]);
        assert_eq!(out.status.code(), Some(2));
        assert!(!dir.exists());
    }
}
