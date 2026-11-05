"""The normal ownership gate must reject missing inventories and fake controls."""
import hashlib
import json
from pathlib import Path

from ci.scripts.checks import ownership_matrix as matrix_gate


ROOT = Path(__file__).resolve().parents[3]


def test_matrix_inventory_matches_authoritative_catalogs_and_is_still_red():
    document = (ROOT / matrix_gate.MATRIX).read_text()
    source = matrix_gate.source_members(ROOT)
    for category, count in (("relations", 69), ("operators", 26), ("derived", 13)):
        rows = matrix_gate.table(document, matrix_gate.HEADERS[category])
        assert {matrix_gate.row_name(row[0]) for row in rows} == source[category]
        assert len(rows) == count
    assert len(matrix_gate.table(document, matrix_gate.HEADERS["routes"])) == 21
    findings = matrix_gate.inspect(ROOT)
    assert any("ownership or enforcement remains unchecked" in item for item in findings)
    assert any("executed controls receipt required" in item for item in findings)


def test_receipt_requires_real_good_and_bad_controls(tmp_path, monkeypatch):
    base = tmp_path / matrix_gate.MATRIX.parent / "evidence/contract-controls"
    base.mkdir(parents=True)
    output = b"control checked expected behavior\n"
    (base / "output.txt").write_bytes(output)
    case = {
        "command": ["python3", "-m", "pytest", "exact_control"],
        "exit": 0,
        "output": "evidence/contract-controls/output.txt",
        "output_sha256": hashlib.sha256(output).hexdigest(),
        "forbidden_rejected": True,
        "build_failed": False,
    }
    receipt = {
        "schema": "contract-controls/v1",
        "contract": "families/1. Typed relation catalog (69 public)",
        "source": {"head": "a" * 40, "sha256": "b" * 64},
        "proof": {"Scheduling": [{"kind": kind, "good": case, "bad": case} for kind in ("tm", "rust", "runtime")]},
    }
    (base / "one.json").write_text(json.dumps(receipt))
    monkeypatch.setattr(matrix_gate.subprocess, "check_call", lambda *args: 0)
    monkeypatch.setattr(matrix_gate, "source_digest", lambda *args: "b" * 64)
    row = ["[controls](evidence/contract-controls/one.json)"]
    failures = []
    matrix_gate.controls(tmp_path, receipt["contract"], row, {"Scheduling"}, failures)
    assert failures == []
    receipt["proof"]["Scheduling"][1]["bad"] = {**case, "build_failed": True}
    (base / "one.json").write_text(json.dumps(receipt))
    failures = []
    matrix_gate.controls(tmp_path, receipt["contract"], row, {"Scheduling"}, failures)
    assert any("build failure is not" in item for item in failures)
