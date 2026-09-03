use spectra_sim::{OcmMessage, OemMessage, ReferenceFrame};

#[test]
fn parses_multisegment_oem_kvn() {
    let text = r#"CCSDS_OEM_VERS = 3.0
CREATION_DATE = 2026-09-02T00:00:00
ORIGINATOR = TEST
META_START
OBJECT_NAME = SAT-A
OBJECT_ID = 2026-001A
CENTER_NAME = EARTH
REF_FRAME = GCRF
TIME_SYSTEM = UTC
START_TIME = 2026-09-02T00:00:00
STOP_TIME = 2026-09-02T00:01:00
META_STOP
2026-09-02T00:00:00 7000 0 0 0 7.5 0
2026-09-02T00:01:00 6990 450 0 -0.48 7.49 0
META_START
OBJECT_NAME = SAT-A
OBJECT_ID = 2026-001A
CENTER_NAME = EARTH
REF_FRAME = GCRF
TIME_SYSTEM = UTC
START_TIME = 2026-09-02T00:01:00
STOP_TIME = 2026-09-02T00:02:00
META_STOP
2026-09-02T00:01:00 6990 450 0 -0.48 7.49 0
2026-09-02T00:02:00 6960 900 0 -0.96 7.46 0
"#;
    let oem = OemMessage::from_kvn(text).unwrap();
    assert_eq!(oem.segments.len(), 2);
    assert_eq!(oem.segments[0].states[0].frame, ReferenceFrame::Gcrf);
    assert_eq!(oem.segments[0].states[0].position_m[0], 7_000_000.0);
}

#[test]
fn parses_oem_xml_state_vectors() {
    let xml = r#"<?xml version="1.0"?>
<oem><header><CCSDS_OEM_VERS>3.0</CCSDS_OEM_VERS></header><body><segment>
<metadata><OBJECT_NAME>SAT-A</OBJECT_NAME><OBJECT_ID>2026-001A</OBJECT_ID><CENTER_NAME>EARTH</CENTER_NAME><REF_FRAME>GCRF</REF_FRAME><TIME_SYSTEM>UTC</TIME_SYSTEM></metadata>
<data>
<stateVector><EPOCH>2026-09-02T00:00:00</EPOCH><X>7000</X><Y>0</Y><Z>0</Z><X_DOT>0</X_DOT><Y_DOT>7.5</Y_DOT><Z_DOT>0</Z_DOT></stateVector>
<stateVector><EPOCH>2026-09-02T00:01:00</EPOCH><X>6990</X><Y>450</Y><Z>0</Z><X_DOT>-0.48</X_DOT><Y_DOT>7.49</Y_DOT><Z_DOT>0</Z_DOT></stateVector>
</data></segment></body></oem>"#;
    let oem = OemMessage::from_xml(xml).unwrap();
    assert_eq!(oem.segments.len(), 1);
    assert_eq!(oem.segments[0].states.len(), 2);
}

#[test]
fn ocm_preserves_typed_and_extension_sections() {
    let text = r#"CCSDS_OCM_VERS = 3.0
META_START
OBJECT_NAME = SAT-A
TIME_SYSTEM = UTC
REF_FRAME = GCRF
META_STOP
PHYS_START
MASS = 125.0 [kg]
PHYS_STOP
TRAJ_START
2026-09-02T00:00:00 7000 0 0 0 7.5 0
2026-09-02T00:01:00 6990 450 0 -0.48 7.49 0
TRAJ_STOP
CUSTOM_START
VENDOR_FIELD = retained
CUSTOM_STOP
"#;
    let ocm = OcmMessage::from_kvn(text).unwrap();
    assert_eq!(ocm.trajectory_states.len(), 2);
    assert_eq!(ocm.physical_properties.get("MASS").unwrap(), "125.0");
    assert!(ocm.extensions.contains_key("CUSTOM"));
}

#[test]
fn parses_standard_ocm_xml_traj_lines_and_units() {
    let xml = r#"<?xml version="1.0"?>
<ocm><header><CCSDS_OCM_VERS>3.0</CCSDS_OCM_VERS></header><body><segment>
<metadata><OBJECT_NAME>SAT-A</OBJECT_NAME><TIME_SYSTEM>UTC</TIME_SYSTEM></metadata>
<data><traj>
<TRAJ_ID>primary</TRAJ_ID><TRAJ_REF_FRAME>GCRF</TRAJ_REF_FRAME><TRAJ_TYPE>CARTPV</TRAJ_TYPE>
<TRAJ_UNITS>km km km km/s km/s km/s</TRAJ_UNITS>
<trajLine>2026-09-02T00:00:00 7000 0 0 0 7.5 0</trajLine>
<trajLine>2026-09-02T00:01:00 6990 450 0 -0.48 7.49 0</trajLine>
</traj></data></segment></body></ocm>"#;
    let ocm = OcmMessage::from_xml(xml).unwrap();
    assert_eq!(ocm.trajectory_blocks.len(), 1);
    assert_eq!(ocm.trajectory_blocks[0].records.len(), 2);
    assert_eq!(ocm.trajectory_states.len(), 2);
    assert_eq!(ocm.trajectory_states[0].state.position_m[0], 7_000_000.0);
}

#[test]
fn ocm_non_cartesian_trajectory_is_preserved_without_fake_cartesian_state() {
    let text = r#"CCSDS_OCM_VERS = 3.0
META_START
OBJECT_NAME = SAT-A
TIME_SYSTEM = UTC
META_STOP
TRAJ_START
TRAJ_ID = kep
TRAJ_REF_FRAME = GCRF
TRAJ_TYPE = KEPLERIAN
2026-09-02T00:00:00 7000 0.01 98.0 20.0 30.0 40.0
2026-09-02T00:01:00 7001 0.01 98.0 20.1 30.0 40.1
TRAJ_STOP
"#;
    let ocm = OcmMessage::from_kvn(text).unwrap();
    assert_eq!(ocm.trajectory_blocks.len(), 1);
    assert_eq!(ocm.trajectory_blocks[0].records.len(), 2);
    assert!(ocm.trajectory_blocks[0].records[0]
        .cartesian_state
        .is_none());
    assert!(ocm.trajectory_states.is_empty());
}

#[test]
fn oem_xml_honors_explicit_si_units() {
    let xml = r#"<oem><body><segment>
<metadata><OBJECT_NAME>SAT-A</OBJECT_NAME><OBJECT_ID>2026-001A</OBJECT_ID><CENTER_NAME>EARTH</CENTER_NAME><REF_FRAME>GCRF</REF_FRAME><TIME_SYSTEM>UTC</TIME_SYSTEM></metadata>
<data>
<stateVector><EPOCH>2026-09-02T00:00:00</EPOCH><X units="m">7000000</X><Y units="m">0</Y><Z units="m">0</Z><X_DOT units="m/s">0</X_DOT><Y_DOT units="m/s">7500</Y_DOT><Z_DOT units="m/s">0</Z_DOT></stateVector>
<stateVector><EPOCH>2026-09-02T00:01:00</EPOCH><X units="m">6990000</X><Y units="m">450000</Y><Z units="m">0</Z><X_DOT units="m/s">-480</X_DOT><Y_DOT units="m/s">7490</Y_DOT><Z_DOT units="m/s">0</Z_DOT></stateVector>
</data></segment></body></oem>"#;
    let oem = OemMessage::from_xml(xml).unwrap();
    assert_eq!(oem.segments[0].states[0].position_m[0], 7_000_000.0);
    assert_eq!(oem.segments[0].states[0].velocity_m_per_s[1], 7_500.0);
}

#[test]
fn oem_segment_builds_absolute_ephemeris() {
    use spectra_sim::AbsoluteEphemeris;
    let text = r#"CCSDS_OEM_VERS = 3.0
META_START
OBJECT_NAME = SAT-A
OBJECT_ID = 2026-001A
CENTER_NAME = EARTH
REF_FRAME = GCRF
TIME_SYSTEM = UTC
META_STOP
2026-09-02T00:00:00 7000 0 0 0 7.5 0
2026-09-02T00:01:00 6990 450 0 -0.48 7.49 0
"#;
    let oem = OemMessage::from_kvn(text).unwrap();
    let ephemeris = AbsoluteEphemeris::from_oem_segment(&oem.segments[0]).unwrap();
    let midpoint = oem.segments[0].states[0]
        .epoch
        .shift_si_seconds(30.0, None)
        .unwrap();
    let state = ephemeris.state_at(midpoint, None).unwrap();
    assert!(state.position_m.iter().all(|value| value.is_finite()));
}
