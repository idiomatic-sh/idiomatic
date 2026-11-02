#!/usr/bin/env python3
"""Render the pinned Ruff rule inventory from source and sealed proof summaries."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
import tomllib

RUFF = Path(__file__).resolve().parents[1]
RESULTS = RUFF / "results"
REPOSITORY = RUFF.parents[6]


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source(rule: str) -> tuple[Path | None, str]:
    path = RUFF / "rules" / rule / "query.ryoql"
    return (path, "query") if path.is_file() else (None, "missing")


def current_source_matches(instance: dict[str, object]) -> bool:
    """A passing row remains current only while its registered settings and helpers match."""
    relative = instance.get("settings_source")
    if not isinstance(relative, str):
        return False
    profile = REPOSITORY / relative
    if not profile.is_file() or instance.get("settings_source_sha256") != sha(profile):
        return False
    settings = tomllib.loads(profile.read_text(encoding="utf-8"))
    base = tomllib.loads((RUFF / "benchmark.toml").read_text(encoding="utf-8"))
    registry = {**base.get("query-modules", {}), **settings.get("query-modules", {})}
    modules = instance.get("module_sha256")
    if not isinstance(modules, dict):
        return False
    return all(logical in registry and (RUFF / registry[logical]).is_file()
               and sha(RUFF / registry[logical]) == digest for logical, digest in modules.items())


def render_full_inventory(path: Path) -> str:
    """Render the runner's complete pinned inventory, including every failure."""
    summary = json.loads(path.read_text(encoding="utf-8"))
    defaults = tomllib.loads((RUFF / "benchmark.toml").read_text(encoding="utf-8"))["defaults"]
    phases = json.loads((RUFF / "profiles/pinned-rule-phases.json").read_text(encoding="utf-8"))
    worker_sha256 = summary.get("worker_sha256") or summary.get("identity", {}).get("worker_sha256")
    rows = summary.get("rules", [])
    ids = [row.get("rule_id") for row in rows]
    if (summary.get("schema") not in {"ruff-full-inventory/v1", "benchmark-inventory/v1"} or len(defaults) != 413
        or len(set(defaults)) != 413 or set(defaults) != set(phases["shipped_default_rules"])
        or len(ids) != 413 or len(set(ids)) != 413 or set(ids) != set(defaults)
        or any(row.get("status") not in {"PASS", "FAIL"} for row in rows)):
        raise ValueError("full inventory result omitted, duplicated, or changed a pinned rule ID")
    by_rule = {row["rule_id"]: row for row in rows}
    supplement_label = ""
    current = RESULTS / "full-inventory-current"
    if path.resolve() == (current / "summary.json").resolve():
        supplement = current / "sim222-sim223-supplement"
        proof_file = supplement / "proof.json"
        if proof_file.is_file():
            proof = json.loads(proof_file.read_text(encoding="utf-8"))
            extra_file = supplement / "summary.json"
            extra = json.loads(extra_file.read_text(encoding="utf-8"))
            extra_rows = extra.get("rules", [])
            if (proof.get("schema") != "benchmark-inventory-supplement/v1"
                or proof.get("base_summary_sha256") != sha(path)
                or proof.get("supplement_summary_sha256") != sha(extra_file)
                or proof.get("raw_receipts_sha256") != sha(supplement / "raw-receipts.tar.gz")
                or proof.get("rule_ids") != [row.get("rule_id") for row in extra_rows]
                or proof.get("rule_ids") != ["SIM222", "SIM223"]
                or extra.get("schema") != "benchmark-inventory/v1"
                or extra.get("run_state") != "complete"
                or extra.get("counts") != {"PASS": 2, "FAIL": 0}
                or extra.get("identity", {}).get("worker_sha256") != worker_sha256
                or any(row.get("status") != "PASS" or row.get("case_count") != 17
                       or row.get("case_count") != row.get("raw_exact")
                       or row.get("documented_discrepancies") != 0 or not row.get("case_results_sha256")
                       or by_rule[row["rule_id"]]["status"] != "FAIL" for row in extra_rows)):
                raise ValueError("SIM supplement is stale or changes the complete base receipt")
            for row in extra_rows:
                by_rule[row["rule_id"]] = {**row, "reason": "[separate 17/17 raw-exact proof](results/full-inventory-current/sim222-sim223-supplement/summary.json)"}
            supplement_label = " The count combines the complete 413-rule run with two later singleton replays on the same worker; it is not one 364-rule execution."
    rendered = []
    passed = 0
    for rule in defaults:
        row = by_rule[rule]
        query, label = source(rule)
        case_path = RUFF / "rules" / rule / "cases.toml"
        status = row["status"]
        reason = row.get("reason") or "verified intended expectations"
        if status == "PASS":
            instance = row.get("instance", {})
            if (query is None or not case_path.is_file()
                or instance.get("query_sha256") != sha(query)
                or instance.get("cases_sha256") != sha(case_path)
                or not current_source_matches(instance)):
                status, reason = "FAIL", "recorded PASS has changed or missing current source/cases"
            elif not worker_sha256 or not row.get("case_results_sha256"):
                raise ValueError("PASS lacks worker identity or executed case receipts")
        passed += status == "PASS"
        if status == "FAIL" and query is not None:
            reason += f"; [query source]({query.relative_to(RUFF).as_posix()})"
        reason = str(reason).replace("|", "\\|").replace("\n", " ")
        rendered.append(f"| {rule} | {status} | {reason} |")
    return "\n".join([
        "# Ruff 0.16.5 complete pinned rule inventory", "",
        "Generated from canonical source and the existing full-inventory runner's results. "
        "Every pinned default is preloaded; missing, unrun, invalid and failed implementations remain FAIL.", "",
        f"**{passed} PASS / {413 - passed} FAIL / 413 pinned defaults**; run state `{summary.get('run_state')}`. "
        f"Worker `{worker_sha256 or 'not run'}`. "
        "PASS means executed intended expectations, including only independently validated explicit reference discrepancies. "
        "Raw parity and documented differences remain explicit in result receipts. This table is not an admission subset." + supplement_label, "",
        "| Rule | Result | Reason / evidence |",
        "|---|---|---|", *rendered, "",
    ])



def render() -> str:
    """Show one current PASS or FAIL for every pinned default."""
    base = tomllib.loads((RUFF / "benchmark.toml").read_text(encoding="utf-8"))
    defaults = base["defaults"]
    assert len(defaults) == len(set(defaults)) == 413
    descriptor = json.loads((RUFF / "profiles/native-standalone.json").read_text(encoding="utf-8"))
    first = json.loads((RESULTS / "native143-split-2026-11-02/summary.json").read_text(encoding="utf-8"))
    supplement = json.loads((RESULTS / "native151-supplement-2026-11-02/summary.json").read_text(encoding="utf-8"))
    assert first["worker_sha256"] == supplement["worker_sha256"]
    assert first["native138_archive_sha256"] == sha(RESULTS / "native143-split-2026-11-02/native138-9348-raw.tar.gz")
    assert first["five_singletons_archive_sha256"] == sha(RESULTS / "native143-split-2026-11-02/five-singletons-9348-raw.tar.gz")
    assert len(first["profiles"]) == 143 and len(supplement["profiles"]) == 8
    assert not set(first["profiles"]) & set(supplement["profiles"])
    assert supplement["case_count"] == 175
    assert descriptor["query_count"] == 151 and descriptor["case_count"] == 3950
    assert {entry["rule_id"] for entry in descriptor["profiles"]} == set(first["profiles"]) | set(supplement["profiles"])

    passed = 0
    rows = []
    for rule in defaults:
        query, _label = source(rule)
        cases = RUFF / "rules" / rule / "cases.toml"
        if rule in first["profiles"]:
            proof = first["profiles"][rule]
            assert query is not None and query.name == "query.ryoql" and cases.is_file()
            assert proof["intended_passes"] == proof["cases"]
            assert proof["raw_exact"] + proof["documented_differences"] + proof["metadata_asymmetries"] == proof["cases"]
            status = "PASS"
            reason = f"[sealed fixture proof](results/native143-split-2026-11-02/summary.json): {proof['cases']}/{proof['cases']} intended, {proof['raw_exact']} raw exact"
        elif rule in supplement["profiles"]:
            proof = supplement["profiles"][rule]
            directory = RESULTS / "native151-supplement-2026-11-02" / rule
            for filename, key in (("result.json", "result_sha256"), ("cases.jsonl", "cases_sha256"), ("stdout.bin", "stdout_sha256"), ("stderr.bin", "stderr_sha256")):
                assert sha(directory / filename) == proof[key]
            assert query is not None and query.name == "query.ryoql" and cases.is_file()
            assert sha(query) == proof["source_sha256"]["query"]
            assert sha(cases) == proof["source_sha256"]["cases"]
            profile = RUFF / "profiles" / rule / "benchmark.toml"
            assert sha(profile) == proof["source_sha256"]["profile"]
            modules = tomllib.loads(profile.read_text(encoding="utf-8"))["query-modules"]
            assert set(modules) == set(proof["module_sha256"])
            for logical, relative in modules.items():
                assert sha(RUFF / relative) == proof["module_sha256"][logical]
            assert proof["candidate_intended"] == proof["reference_intended"] == proof["raw_exact"] == proof["cases"]
            status = "PASS"
            reason = f"[sealed fixture proof](results/native151-supplement-2026-11-02/{rule}/result.json): {proof['cases']}/{proof['cases']} intended and raw exact"
        else:
            status = "FAIL"
            if query is None:
                reason = "No query source"
            elif not cases.is_file():
                reason = f"No fixtures for [query]({query.relative_to(RUFF).as_posix()})"
            else:
                reason = f"No current sealed-worker proof for [query]({query.relative_to(RUFF).as_posix()})"
        passed += status == "PASS"
        rows.append(f"| {rule} | {status} | {reason} |")
    assert len(rows) == 413 and passed == 151
    return "\n".join([
        "# Ruff 0.16.5 rule check", "",
        f"**{passed} PASS / {413-passed} FAIL / 413 TOTAL**", "",
        f"PASS means current fixture proof on sealed worker `{first['worker_sha256']}`. "
        "FAIL means that proof is missing; the reason is listed per rule. "
        "[Native143 receipts](results/native143-split-2026-11-02/summary.json) and "
        "[eight-rule supplement](results/native151-supplement-2026-11-02/summary.json) "
        "are separate runs, not a single 151-rule execution. "
        "The full 413-rule CI runner reports its own PASS/FAIL result and raw receipts.", "",
        "| Rule | Result | Reason / evidence |", "|---|---|---|", *rows, "",
    ])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="refuse a stale RULES.md")
    parser.add_argument("--full-inventory-result", type=Path, help="render all 413 PASS/FAIL rows from a completed or interrupted full-inventory summary")
    args = parser.parse_args()
    destination = RUFF / "RULES.md"
    full_result = args.full_inventory_result or RESULTS / "full-inventory-current/summary.json"
    rendered = render_full_inventory(full_result) if full_result.is_file() else render()
    if args.full_inventory_result is not None and not full_result.is_file():
        raise ValueError("requested full-inventory result does not exist")
    if args.check:
        if not destination.is_file() or destination.read_text(encoding="utf-8") != rendered:
            print("RULES.md is stale; regenerate it", file=sys.stderr)
            return 1
        print("RULES.md matches pinned source and proof receipts")
        return 0
    destination.write_text(rendered, encoding="utf-8")
    print(f"wrote {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
