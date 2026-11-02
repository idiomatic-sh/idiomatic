"""Validate source-bound target-version fixture evidence for the inventory renderer."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
import tarfile
import tomllib

from authoring import case_records, reference_discrepancies


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verified_rows(ruff: Path, current: Path, previous: dict[str, object], directory: Path) -> list[dict[str, object]]:
    if not directory.is_dir():
        return []
    proof = json.loads((directory / "proof.json").read_text())
    summary = json.loads((directory / "summary.json").read_text())
    controls = json.loads((directory / "controls.json").read_text())
    required = proof.get("requirements")
    if (not isinstance(required, dict) or not required
        or any(rule not in previous or not isinstance(targets, list) or not targets
               or len(targets) != len(set(targets)) for rule, targets in required.items())):
        raise ValueError("invalid manifest target requirements")
    rows = summary.get("rules", [])
    if (proof.get("schema") != "benchmark-inventory-target-supplement/v1"
        or proof.get("rule_ids") != list(required) or proof.get("requirements") != required
        or [row.get("rule_id") for row in rows] != list(required)
        or summary.get("schema") != "benchmark-inventory/v1" or summary.get("run_state") != "complete"
        or summary.get("counts") != {"PASS": len(required), "FAIL": 0}
        or summary.get("identity") != proof.get("identity") or controls.get("identity") != proof.get("identity")):
        raise ValueError("invalid target supplement")
    for filename, key in [("summary.json", "summary_sha256"), ("controls.json", "controls_sha256"),
                          ("provenance.json", "provenance_sha256"), ("raw-receipts.tar.gz", "raw_receipts_sha256")]:
        if proof.get(key) != sha(directory / filename):
            raise ValueError(f"stale target supplement {filename}")
    prior = proof.get("prior_summaries_sha256")
    if (not isinstance(prior, dict) or "summary.json" not in prior
        or any(PurePosixPath(name).is_absolute() or ".." in PurePosixPath(name).parts for name in prior)
        or any(digest != sha(current / name) for name, digest in prior.items())):
        raise ValueError("target supplement changes an earlier receipt")
    actual = controls.get("controls", [])
    if [(item.get("rule"), item.get("target")) for item in actual] != [
        (rule, target) for rule, targets in required.items() for target in targets]:
        raise ValueError("missing, duplicate or unknown target control")
    with tarfile.open(directory / "raw-receipts.tar.gz", "r:gz") as archive:
        def read(name: str, expected: str | None = None) -> bytes:
            value = archive.extractfile(name)
            if value is None:
                raise ValueError("missing archived target evidence")
            data = value.read()
            if expected is not None and hashlib.sha256(data).hexdigest() != expected:
                raise ValueError("stale archived target evidence")
            return data
        for item in actual:
            rule, target = item["rule"], item["target"]
            query = ruff / "rules" / rule / "query.ryoql"
            settings = ruff / item["settings_source"]
            cases = ruff / item["cases_file"]
            if (item["query_sha256"] != sha(query) or item["cases_sha256"] != sha(cases)
                or item["settings_source_sha256"] != sha(settings)):
                raise ValueError("target control has changed current source/settings/cases")
            source_settings = tomllib.loads(settings.read_text())
            source_cases = tomllib.loads(cases.read_text())["cases"]
            for case in source_cases:
                reference_discrepancies.validate_reference_discrepancy(settings.parent, case, rule)
            registered = json.loads(read(item["registration_path"], item["registration_sha256"]))
            observed = json.loads(read(item["summary_path"], item["summary_sha256"]))
            declared = registered["rules"][0]["cases"]
            case_hashes = {case["name"]: hashlib.sha256(json.dumps(case, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest() for case in source_cases}
            count = len(source_cases)
            if (count == 0 or len(case_hashes) != count or len(declared) != count
                or {case["name"]: case["case_sha256"] for case in declared} != case_hashes):
                raise ValueError("archived registration differs from current exact cases")
            instance = registered["rules"][0]["instance"]
            profile = tomllib.loads(read(item["profile_path"], item["profile_sha256"]).decode())
            read(item["query_path"], item["query_sha256"])
            prefix = PurePosixPath(item["profile_path"]).parent
            for logical, relative in profile["query-modules"].items():
                digest = instance["module_sha256"][logical]
                read(str(prefix / relative), digest)
                if sha(ruff / relative) != digest:
                    raise ValueError("target control helper has changed")
            if (any(registered["identity"].get(key) != digest for key, digest in proof["identity"].items())
                or observed.get("identity") != registered["identity"]
                or profile.get("parser-configuration") != source_settings.get("parser-configuration")
                or instance.get("query_sha256") != item["query_sha256"]
                or instance.get("cases_sha256") != item["cases_sha256"]):
                raise ValueError("target control invocation/source identity mismatch")
            records = [json.loads(line) for line in read(item["case_results_path"], item["case_results_sha256"]).splitlines()]
            counts = case_records.case_expectation_summary(registered["rules"][0]["cases"], records)
            raw, differences = item["raw_exact"], item["documented_discrepancies"]
            if (item.get("case_count") != count or type(raw) is not int or type(differences) is not int
                or min(raw, differences) < 0 or raw + differences != count or len(records) != 2 * count
                or counts["complete_case_pairs"] != count or counts["intended_behavior_passes"] != count
                or counts["raw_parity_matches"] != raw or counts["documented_reference_discrepancies"] != differences
                or counts["unexplained_mismatches"] or any(record.get("case_passed") is not True for record in records)
                or observed.get("run_state") != "complete"
                or observed.get("counts") != {"PASS": 1, "FAIL": 0}
                or observed["rules"][0].get("status") != "PASS"
                or observed["rules"][0].get("case_results_sha256") != item["case_results_sha256"]):
                raise ValueError("target fixture execution is incomplete or failed")
    for row in rows:
        rule = row["rule_id"]
        canonical = next((item for item in actual if item["rule"] == rule
                          and item["case_results_sha256"] == proof.get("default_case_results_sha256", {}).get(rule)
                          and item["profile_sha256"] == row.get("instance", {}).get("profile_sha256")), None)
        if canonical is None:
            raise ValueError("default summary lacks its declared executed control")
        if (previous[rule]["status"] != "FAIL" or row.get("status") != "PASS"
            or row.get("case_count") != canonical["case_count"] or row.get("raw_exact") != canonical["raw_exact"]
            or row.get("documented_discrepancies") != canonical["documented_discrepancies"]
            or row.get("case_results_sha256") != canonical["case_results_sha256"]
            or row.get("instance", {}).get("profile_sha256") != canonical["profile_sha256"]):
            raise ValueError("default target summary is not its executed fixture evidence")
    return rows
