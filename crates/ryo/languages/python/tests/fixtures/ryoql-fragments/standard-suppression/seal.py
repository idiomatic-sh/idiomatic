"""Validate every immutable side record and seal migration equivalence separately from Ruff parity."""

import argparse
import collections
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def seal(root, receipt_path=None):
    receipt_path = receipt_path or Path(__file__).with_name("source-receipt.json")
    manifest = json.loads((root / "manifest.json").read_text())
    validator = (
        root
        / "validator/crates/ryo/languages/python/benchmarks/linters/ruff/verify_profiles.py"
    )
    validator.parent.mkdir(parents=True, exist_ok=True)
    validator.write_bytes((root / "validator-source.py").read_bytes())
    shutil.copytree(
        root / "candidate/fragments", validator.parent / "fragments", dirs_exist_ok=True
    )
    phases = None
    if "rule_phase_registry_sha256" in manifest:
        assert (
            digest(root / "pinned-rule-phases.json")
            == manifest["rule_phase_registry_sha256"]
        )
        phases = json.loads((root / "pinned-rule-phases.json").read_text())
        (validator.parent / "profiles").mkdir(exist_ok=True)
        shutil.copyfile(
            root / "pinned-rule-phases.json",
            validator.parent / "profiles/pinned-rule-phases.json",
        )
    spec = importlib.util.spec_from_file_location("captured_validator", validator)
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    assert digest(root / "validator-source.py") == manifest["validator_sha256"]
    guard.self_test_standard_suppression()
    source = json.loads(receipt_path.read_text())
    assert digest(receipt_path) == manifest["source_receipt_sha256"]
    records = [
        json.loads(line) for line in (root / "records.jsonl").read_text().splitlines()
    ]
    counts = collections.Counter((r["rule"], r["case"], r["side"]) for r in records)
    assert all(count == 1 for count in counts.values()), "duplicate side records"
    index = {(r["rule"], r["case"], r["side"]): r for r in records}
    expected_keys = set()
    differences = []
    skipped = []
    total = 0
    cases_hashes = {}
    for row in source["rows"]:
        rule = row["rule"]
        settings = None
        if phases is not None:
            captured = root / "canonical-profiles" / (rule + ".toml")
            expected = manifest["canonical_settings"][rule]
            assert digest(captured) == expected["profile_sha256"]
            settings = guard.canonical_run_ryo_settings(
                tomllib.loads(captured.read_text()), [rule], phases
            )
            assert settings == expected["effective"]
        single = root / "candidate/single" / rule
        cases_path = single / "rules" / rule / "cases.toml"
        cases_hashes[rule] = digest(cases_path)
        cases = tomllib.loads(cases_path.read_text())["cases"]
        inventory = guard.profile_case_inventory(single)
        assert len(inventory) == len(cases)
        normalized = {case["name"]: case for case in inventory}
        for side in ("baseline", "candidate"):
            profile = root / side / "single" / rule
            query = profile / "rules" / rule / "query.ryoql"
            assert (
                digest(query)
                == row["before_sha256" if side == "baseline" else "after_sha256"]
            )
            for name, path in manifest["profiles"][side]["module_registry"].items():
                assert (
                    digest(profile / path)
                    == manifest["profiles"][side]["module_sha256"][name]
                )
        for case in cases:
            total += 1
            reports = {}
            for side in ("baseline", "candidate"):
                key = (rule, case["name"], side)
                expected_keys.add(key)
                actual = index[key]
                assert actual["case_file_sha256"] == cases_hashes[rule]
                # Bind the existing raw receipt encoding, without rewriting it.
                assert (
                    actual["source_sha256"]
                    == hashlib.sha256(
                        json.dumps(case["files"], sort_keys=True).encode()
                    ).hexdigest()
                )
                assert (
                    actual["request_sha256"]
                    == hashlib.sha256(
                        json.dumps(actual["request"], sort_keys=True).encode()
                    ).hexdigest()
                )
                assert (
                    actual["request"]["query_modules"]
                    == manifest["profiles"][side]["module_registry"]
                )
                if settings is not None:
                    guard.check_run_ryo_settings(actual["request"], settings)
                else:
                    # This is the recorded legacy author configuration, not a
                    # claim that EXE005/PYI033 had canonical token-phase coverage.
                    assert (
                        actual["request"]["parser_configuration"]
                        == "python-target=3.10"
                    )
                    assert actual["request"]["rule_applicability"] == {
                        rule: "accepted-syntax"
                    }
                report = guard.complete_run_ryo_report(
                    actual["exit"],
                    actual["stdout"],
                    actual["stderr"],
                    list(case["files"]),
                    [rule],
                    **(
                        {"request": actual["request"], "settings": settings}
                        if settings is not None
                        else {}
                    ),
                )
                assert report == actual["report"]
                reports[side] = report
                if report["skipped"]:
                    skipped.append(
                        {
                            "rule": rule,
                            "case": case["name"],
                            "side": side,
                            "receipts": report["skipped"],
                        }
                    )
            assert reports["baseline"] == reports["candidate"], (
                rule,
                case["name"],
                "changed full report",
            )
            if guard.diagnostic_multiset(
                reports["candidate"]["diagnostics"]
            ) != guard.diagnostic_multiset(normalized[case["name"]]["expected"]):
                differences.append(
                    {
                        "rule": rule,
                        "case": case["name"],
                        "expected": normalized[case["name"]]["expected"],
                        "actual": reports["candidate"]["diagnostics"],
                    }
                )
    assert (
        set(index) == expected_keys and len(records) == len(expected_keys) == 2 * total
    )
    proof = {
        "schema": "standard-suppression-migration-proof/v2"
        if phases is not None
        else "standard-suppression-migration-proof/v1",
        "claim": "All complete baseline/candidate reports exactly equal; existing Ruff expectation differences remain separate.",
        "rules": len(source["rows"]),
        "cases": total,
        "side_records": len(records),
        "all_process_statuses_zero": True,
        "unknown_or_unread": 0,
        "full_report_differences": 0,
        "existing_expected_differences": differences,
        "accepted_syntax_skips": skipped,
        "case_sha256": cases_hashes,
        "records_sha256": digest(root / "records.jsonl"),
        "manifest_sha256": digest(root / "manifest.json"),
        "validator_sha256": manifest["validator_sha256"],
        "source_receipt_sha256": manifest["source_receipt_sha256"],
        "worker_sha256": manifest["worker_sha256"],
        "helper_sha256": manifest["suppress_sha256"],
        "no_ruff_invocations": True,
        "phase_settings_policy": "explicit-pinned-canonical"
        if phases is not None
        else "legacy all-accepted-syntax; EXE005/PYI033 canonical coverage pending",
        "rule_phase_registry_sha256": manifest.get("rule_phase_registry_sha256"),
        "rejected_prior_run": "/tmp/suppress-migration-20261101-rejected-v1/REVOKED.txt",
        "benchmark_measurement": False,
    }
    destination = root / "sealed-proof.json"
    with destination.open("x") as output:
        output.write(json.dumps(proof, indent=2) + "\n")
    print(
        json.dumps(
            {
                k: proof[k]
                for k in (
                    "rules",
                    "cases",
                    "side_records",
                    "all_process_statuses_zero",
                    "full_report_differences",
                    "unknown_or_unread",
                )
            }
        )
    )
    return proof


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    seal(parser.parse_args().run)
