"""Validate source-bound target-version fixture evidence for the inventory renderer."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
import tarfile
import tomllib

from authoring import case_observations, reference_discrepancies


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def observed_row(observed: dict[str, object], rule: str) -> dict[str, object]:
    """The control's one executed row. The collector reports the whole pinned inventory: every other row is a rule the
    registration did not select, which fails as not selected and carries no cases."""
    rows = observed.get("rules", [])
    selected = [row for row in rows if row.get("rule_id") == rule]
    others = [row for row in rows if row.get("rule_id") != rule]
    if (len(selected) != 1 or observed.get("counts") != {"PASS": 1, "FAIL": len(others)}
        or any(row.get("status") != "FAIL" or row.get("reason") != "not selected for this invocation"
               or row.get("case_count") != 0 for row in others)):
        raise ValueError("target fixture execution is incomplete or failed")
    return selected[0]


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
            for logical, relative in profile.get("query-modules", {}).items():
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
            counts = case_observations.case_expectation_summary(registered["rules"][0]["cases"], records)
            raw, differences = item["raw_exact"], item["documented_discrepancies"]
            contract_only = item.get("contract_only", 0)
            if (item.get("case_count") != count or type(raw) is not int or type(differences) is not int
                or type(contract_only) is not int or min(raw, differences, contract_only) < 0
                or raw + differences + contract_only != count or len(records) != 2 * count
                or counts.get("contract_only_passes", 0) != contract_only
                or counts["complete_case_pairs"] != count or counts["intended_behavior_passes"] != count
                or counts["raw_parity_matches"] != raw or counts["documented_reference_discrepancies"] != differences
                or counts["unexplained_mismatches"] or any(record.get("case_passed") is not True for record in records)
                or observed.get("run_state") != "complete"
                or observed_row(observed, rule).get("status") != "PASS"
                or observed_row(observed, rule).get("case_results_sha256") != item["case_results_sha256"]):
                raise ValueError("target fixture execution is incomplete or failed")
        direct = controls.get("direct_negative", [])
        required_direct = proof.get("direct_negative_requirements", {})
        if (not isinstance(direct, list) or not isinstance(required_direct, dict)
            or [(item.get("rule"), item.get("target")) for item in direct] != [
                (rule, target) for rule, targets in required_direct.items() for target in targets]
            or any(rule not in required or not isinstance(targets, list) or not targets
                   or len(targets) != len(set(targets)) for rule, targets in required_direct.items())):
            raise ValueError("missing, duplicate or unknown direct negative target control")
        for item in direct:
            rule = item["rule"]
            canonical = tomllib.loads((ruff / "rules" / rule / "cases.toml").read_text())["cases"]
            counter = tomllib.loads(read(item["cases_path"], item["cases_sha256"]).decode())["cases"]
            profile = tomllib.loads(read(item["profile_path"], item["profile_sha256"]).decode())
            observed = json.loads(read(item["summary_path"], item["summary_sha256"]))
            if (item["query_sha256"] != sha(ruff / "rules" / rule / "query.ryoql")
                or item["canonical_cases_sha256"] != sha(ruff / "rules" / rule / "cases.toml")
                or hashlib.sha256(read(item["query_path"])).hexdigest() != item["query_sha256"]
                or profile.get("parser-configuration") != item["parser_configuration"]
                or profile.get("defaults") != [rule]
                or observed.get("worker_sha256") != proof["identity"]["worker_sha256"]
                or observed.get("query_sha256") != item["query_sha256"]
                or observed.get("cases_sha256") != item["cases_sha256"]
                or observed.get("cases") != len(canonical) or observed.get("passed") != len(canonical)
                or len(counter) != len(canonical) or len(observed.get("results", [])) != len(canonical)):
                raise ValueError("direct negative target source or execution identity differs")
            positive_settings = next(control for control in actual if control["rule"] == rule)
            source_settings = tomllib.loads((ruff / positive_settings["settings_source"]).read_text())
            if any(profile.get(key) != source_settings.get(key) for key in ("tool", "version", "sha256")):
                raise ValueError("direct negative target changes the pinned reference")
            profile_prefix = PurePosixPath(item["profile_path"]).parent
            executed_rules = str(Path(item["executed_stage_path"]) / "rules")
            for relative in profile.get("query-modules", {}).values():
                if read(str(profile_prefix / relative)) != (ruff / relative).read_bytes():
                    raise ValueError("direct negative target has stale helper bytes")
            for index, (source_case, counter_case, result) in enumerate(zip(canonical, counter, observed["results"])):
                if (len(source_case["files"]) != 1
                    or source_case["name"] != counter_case["name"] or source_case["files"] != counter_case["files"]
                    or counter_case["expected"] != [] or result.get("case") != source_case["name"]
                    or result.get("passed") is not True or result.get("exit") != 0
                    or result.get("reference_exit") != 0 or result.get("reference_findings") != 0):
                    raise ValueError("direct negative target changes a source or failed execution")
                file, source = next(iter(source_case["files"].items()))
                prefix = f'{item["archive_prefix"]}/{index:02d}'
                request = json.loads(read(f"{prefix}/request.json"))
                candidate = json.loads(read(f"{prefix}/stdout.bin", result["stdout_sha256"]))
                reference = json.loads(read(f"{prefix}/ruff-stdout.bin", result["reference_stdout_sha256"]))
                coverage = candidate.get("coverage", {})
                if (read(f"{prefix}/{file}") != source.encode()
                    or request.get("parser_configuration") != item["parser_configuration"]
                    or request.get("rules") != executed_rules
                    or request.get("files") != [file]
                    or request.get("query_modules") != profile.get("query-modules", {})
                    or request.get("rule_applicability") != profile.get("rule-applicability", {})
                    or candidate.get("diagnostics") != [] or candidate.get("skipped") != []
                    or candidate.get("files") != [file] or candidate.get("execution_metadata", {}).get("availability") != "complete"
                    or (coverage.get("selected"), coverage.get("evaluated"), coverage.get("unknown_or_unread")) != (1, 1, 0)
                    or reference != []):
                    raise ValueError("direct negative target has findings or incomplete execution")
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
            or row.get("contract_only", 0) != canonical.get("contract_only", 0)
            or row.get("case_results_sha256") != canonical["case_results_sha256"]
            or row.get("instance", {}).get("profile_sha256") != canonical["profile_sha256"]):
            raise ValueError("default target summary is not its executed fixture evidence")
    return rows


def verified_failed_rows(ruff: Path, previous: dict[str, object], directory: Path) -> list[dict[str, object]]:
    """Refresh FAIL reasons from source-bound attempts; this path cannot promote a rule."""
    if not directory.is_dir():
        return []
    proof = json.loads((directory / "proof.json").read_text())
    failed = proof.get("failed_controls", [])
    if len({item["rule"] for item in failed}) != len(failed):
        raise ValueError("duplicate failed fixture observation")
    if proof["raw_receipts_sha256"] != sha(directory / "raw-receipts.tar.gz"):
        raise ValueError("changed failed fixture archive")
    rows = []
    with tarfile.open(directory / "raw-receipts.tar.gz", "r:gz") as archive:
        for item in failed:
            rule = item["rule"]
            instance = item["instance"]
            if (rule not in previous or previous[rule]["status"] != "FAIL"
                or sha(ruff / instance["query_file"]) != instance["query_sha256"]
                or sha(ruff / instance["cases_file"]) != instance["cases_sha256"]
                or sha(ruff / item["settings_source"]) != instance["settings_source_sha256"]):
                raise ValueError("changed failed fixture source/settings")
            cases = tomllib.loads((ruff / instance["cases_file"]).read_text())["cases"]
            registration = archive.extractfile(item["registration_path"])
            if registration is None:
                raise ValueError("missing failed fixture registration")
            declared = json.loads(registration.read())["rules"][0]["cases"]
            hashes = {case["name"]: hashlib.sha256(json.dumps(case, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest() for case in cases}
            if len(hashes) != len(cases) or {case["name"]: case["case_sha256"] for case in declared} != hashes:
                raise ValueError("changed failed fixture cases")
            stream = archive.extractfile(item["case_results_path"])
            if stream is None:
                raise ValueError("missing failed fixture receipt")
            raw = stream.read()
            if hashlib.sha256(raw).hexdigest() != item["case_results_sha256"]:
                raise ValueError("changed failed fixture receipt")
            records = [json.loads(line) for line in raw.splitlines()]
            counts = case_observations.case_expectation_summary(declared, records)
            if (len(records) != 2 * len(cases) or counts != item["native_counts"]
                or counts["intended_behavior_passes"] == len(cases)):
                raise ValueError("invalid failed fixture observation")
            rows.append({"rule_id": rule, "status": "FAIL", "instance": instance,
                         "reason": "fixture mismatch or incomplete candidate",
                         "failure_receipt": str((directory / "provenance.json").relative_to(ruff))})
    return rows
