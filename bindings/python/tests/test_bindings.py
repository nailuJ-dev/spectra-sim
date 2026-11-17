"""Tests for the spectra_sim Python bindings."""

import json
from pathlib import Path

import numpy as np
import pytest

import spectra_sim

ROOT = Path(__file__).resolve().parents[3]
SCENARIOS = ROOT / "examples" / "scenarios"

# Pinned in tests/sdk_contracts.rs (`output_hash_is_unchanged`) for sigint_urban.json.
URBAN_OUTPUT_SHA256 = "a40a02e982b9d8223132993f27274fc26cc86516b0cff5c507b94a34f6766cc2"


def load(name):
    return (SCENARIOS / name).read_text()


def test_version_matches_crate():
    cargo = (ROOT / "Cargo.toml").read_text()
    version = next(
        line.split('"')[1] for line in cargo.splitlines() if line.startswith("version")
    )
    assert spectra_sim.__version__ == version


def test_run_sigint_shapes():
    text = load("sigint_urban.json")
    scenario = json.loads(text)
    result = spectra_sim.run_sigint(text)
    assert result.iq.dtype == np.complex64
    assert result.iq.ndim == 1
    assert len(result.iq) == scenario["sigint_job"]["samples"]
    assert result.sample_rate_hz > 0.0
    assert result.center_frequency_hz > 0.0
    assert json.loads(result.truth_json)
    assert "output_sha256" in json.loads(result.manifest_json)


def test_run_sigint_matches_cli_hash():
    """The manifest hash equals the Rust CLI/library value.

    The expected hash is the one pinned in tests/sdk_contracts.rs
    (`output_hash_is_unchanged`) for examples/scenarios/sigint_urban.json, so the
    test does not need to build or run the CLI.
    """
    result = spectra_sim.run_sigint(load("sigint_urban.json"))
    assert json.loads(result.manifest_json)["output_sha256"] == URBAN_OUTPUT_SHA256


def test_validate_rejects_bad_json():
    with pytest.raises(ValueError):
        spectra_sim.validate("{not json")


def test_validate_rejects_invalid_scenario():
    scenario = json.loads(load("sigint_urban.json"))
    scenario["schema_version"] = "bogus"
    with pytest.raises(ValueError, match="schema_version"):
        spectra_sim.validate(json.dumps(scenario))


def test_run_cuas_returns_json():
    run = json.loads(spectra_sim.run_cuas(load("cuas_drone_isac.json")))
    assert "radar_measurement" in run
    assert "manifest" in run


def test_run_cuas_without_job_is_value_error():
    with pytest.raises(ValueError):
        spectra_sim.run_cuas(load("sigint_urban.json"))


def test_verify_replay_roundtrip():
    text = load("sigint_urban.json")
    result = spectra_sim.run_sigint(text)
    assert spectra_sim.verify_replay(text, result.manifest_json, "sigint") == "match"

    tampered = json.loads(result.manifest_json)
    tampered["output_sha256"] = "0" * 64
    assert (
        spectra_sim.verify_replay(text, json.dumps(tampered), "sigint")
        == "output_mismatch"
    )

    cuas_text = load("cuas_drone_isac.json")
    cuas = json.loads(spectra_sim.run_cuas(cuas_text))
    assert (
        spectra_sim.verify_replay(cuas_text, json.dumps(cuas["manifest"]), "cuas")
        == "match"
    )


def test_verify_replay_rejects_unknown_kind():
    text = load("sigint_urban.json")
    result = spectra_sim.run_sigint(text)
    with pytest.raises(ValueError, match="kind"):
        spectra_sim.verify_replay(text, result.manifest_json, "other")


@pytest.mark.parametrize(
    "path",
    sorted(p for p in SCENARIOS.glob("*.json") if p.name != "randomization.json"),
    ids=lambda p: p.name,
)
def test_validate_accepts_examples(path):
    assert spectra_sim.validate(path.read_text()) is None


def test_oversized_scenario_rejected():
    with pytest.raises(ValueError, match="dimension limit exceeded"):
        spectra_sim.validate(" " * (32 * 1024 * 1024 + 1))
