use spectra_sim::{ReferenceFrame, Sgp4Mode, Sgp4Propagator, Tle};

#[test]
fn vallado_case_00005_matches_reference_at_epoch() {
    let tle = Tle::parse(
        Some("VANGUARD 1".into()),
        "1 00005U 58002B   00179.78495062  .00000023  00000-0  28098-4 0  4753",
        "2 00005  34.2682 348.7242 1859667 331.7664  19.3264 10.82419157413667",
    )
    .unwrap();
    let state = Sgp4Propagator::from_tle_with_mode(&tle, Sgp4Mode::AfspcWgs72)
        .unwrap()
        .propagate_minutes(0.0)
        .unwrap();
    assert_eq!(state.frame, ReferenceFrame::Teme);
    let expected_r = [7022.46529266e3, -1400.08296755e3, 0.03995155e3];
    let expected_v = [1.893841015e3, 6.405893759e3, 4.534807250e3];
    for i in 0..3 {
        assert!((state.position_m[i] - expected_r[i]).abs() < 0.05);
        assert!((state.velocity_m_per_s[i] - expected_v[i]).abs() < 5.0e-5);
    }
}

#[test]
fn signed_propagation_time_is_not_collapsed_to_zero() {
    let tle = Tle::parse(
        None,
        "1 00005U 58002B   00179.78495062  .00000023  00000-0  28098-4 0  4753",
        "2 00005  34.2682 348.7242 1859667 331.7664  19.3264 10.82419157413667",
    )
    .unwrap();
    let prop = Sgp4Propagator::from_tle(&tle).unwrap();
    let before = prop.propagate_minutes(-60.0).unwrap();
    let after = prop.propagate_minutes(60.0).unwrap();
    assert_ne!(before.position_m, after.position_m);
}

#[test]
fn omm_json_preserves_integer_catalog_fields_and_propagates() {
    use spectra_sim::{OmmMessage, Sgp4Propagator};
    let input = r#"{
        "OBJECT_NAME": "ISS (ZARYA)",
        "OBJECT_ID": "1998-067A",
        "EPOCH": "2020-07-12T01:19:07.402656",
        "MEAN_MOTION": 15.49560532,
        "ECCENTRICITY": 0.0001771,
        "INCLINATION": 51.6435,
        "RA_OF_ASC_NODE": 225.4004,
        "ARG_OF_PERICENTER": 44.9625,
        "MEAN_ANOMALY": 5.1087,
        "EPHEMERIS_TYPE": 0,
        "CLASSIFICATION_TYPE": "U",
        "NORAD_CAT_ID": 25544,
        "ELEMENT_SET_NO": 999,
        "REV_AT_EPOCH": 23587,
        "BSTAR": 0.0049645,
        "MEAN_MOTION_DOT": 0.00289036,
        "MEAN_MOTION_DDOT": 0
    }"#;
    let omm = OmmMessage::from_json(input).unwrap();
    let elements = omm.elements().unwrap();
    assert_eq!(elements.norad_id, 25544);
    assert_eq!(elements.element_set_number, 999);
    assert_eq!(elements.revolution_number, 23587);
    let state = Sgp4Propagator::from_omm(&omm)
        .unwrap()
        .propagate_minutes(0.0)
        .unwrap();
    assert!(state.position_m.iter().all(|value| value.is_finite()));
}

#[test]
fn omm_kvn_and_xml_produce_equivalent_elements() {
    use spectra_sim::OmmMessage;
    let kvn = r#"CCSDS_OMM_VERS = 3.0
OBJECT_NAME = ISS (ZARYA)
OBJECT_ID = 1998-067A
EPOCH = 2020-07-12T01:19:07.402656
MEAN_MOTION = 15.49560532
ECCENTRICITY = 0.0001771
INCLINATION = 51.6435
RA_OF_ASC_NODE = 225.4004
ARG_OF_PERICENTER = 44.9625
MEAN_ANOMALY = 5.1087
EPHEMERIS_TYPE = 0
CLASSIFICATION_TYPE = U
NORAD_CAT_ID = 25544
ELEMENT_SET_NO = 999
REV_AT_EPOCH = 23587
BSTAR = 0.0049645
MEAN_MOTION_DOT = 0.00289036
MEAN_MOTION_DDOT = 0
"#;
    let xml = r#"<omm><body><segment><metadata>
<OBJECT_NAME>ISS (ZARYA)</OBJECT_NAME><OBJECT_ID>1998-067A</OBJECT_ID>
</metadata><data><meanElements>
<EPOCH>2020-07-12T01:19:07.402656</EPOCH><MEAN_MOTION>15.49560532</MEAN_MOTION>
<ECCENTRICITY>0.0001771</ECCENTRICITY><INCLINATION>51.6435</INCLINATION>
<RA_OF_ASC_NODE>225.4004</RA_OF_ASC_NODE><ARG_OF_PERICENTER>44.9625</ARG_OF_PERICENTER>
<MEAN_ANOMALY>5.1087</MEAN_ANOMALY>
</meanElements><tleParameters>
<EPHEMERIS_TYPE>0</EPHEMERIS_TYPE><CLASSIFICATION_TYPE>U</CLASSIFICATION_TYPE>
<NORAD_CAT_ID>25544</NORAD_CAT_ID><ELEMENT_SET_NO>999</ELEMENT_SET_NO><REV_AT_EPOCH>23587</REV_AT_EPOCH>
<BSTAR>0.0049645</BSTAR><MEAN_MOTION_DOT>0.00289036</MEAN_MOTION_DOT><MEAN_MOTION_DDOT>0</MEAN_MOTION_DDOT>
</tleParameters></data></segment></body></omm>"#;
    let a = OmmMessage::from_kvn(kvn).unwrap().elements().unwrap();
    let b = OmmMessage::from_xml(xml).unwrap().elements().unwrap();
    assert_eq!(a.norad_id, b.norad_id);
    assert!((a.mean_motion - b.mean_motion).abs() < 1e-15);
    assert!((a.eccentricity - b.eccentricity).abs() < 1e-15);
}
