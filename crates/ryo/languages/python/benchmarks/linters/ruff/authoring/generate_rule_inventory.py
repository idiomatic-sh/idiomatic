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


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source(rule: str) -> tuple[Path | None, str]:
    directory = RUFF / "rules" / rule
    for name, label in (("query.ryoql", "native query"), ("candidate.ryoql", "candidate")):
        path = directory / name
        if path.is_file():
            return path, label
    return None, "absent"


def render() -> str:
    base = tomllib.loads((RUFF / "benchmark.toml").read_text(encoding="utf-8"))
    rules = base["defaults"]
    assert len(rules) == 413 and len(rules) == len(set(rules))
    descriptor = json.loads((RUFF / "profiles/native-standalone.json").read_text(encoding="utf-8"))
    current = json.loads((RESULTS / "native138-current-2026-11-02/summary.json").read_text(encoding="utf-8"))
    historical = json.loads((RESULTS / "correctness-324-per-rule-2026-11-01.json").read_text(encoding="utf-8"))
    historical_final = json.loads((RESULTS / "correctness-324-final-2026-11-01.json").read_text(encoding="utf-8"))
    historical_manifest = json.loads((RESULTS / "correctness-324-manifest-2026-11-01.json").read_text(encoding="utf-8"))
    archive = RESULTS / "native138-current-2026-11-02/raw.tar.gz"
    assert current["raw_archive_sha256"] == sha(archive)
    assert historical["raw_records_sha256"] == historical_final["raw_candidate_records_sha256"]
    assert historical["worker_sha256"] == historical_final["worker_sha256"]
    assert historical["selection_sha256"] == historical_manifest["inventory_sha256"]
    assert descriptor["query_count"] == len(current["profiles"]) == 138
    assert descriptor["case_count"] == sum(row["cases"] for row in current["profiles"].values()) == 3504
    assert set(current["profiles"]) == {row["rule_id"] for row in descriptor["profiles"]}
    assert set(historical["profiles"]) == set(historical_manifest["selection"])
    assert len(historical["profiles"]) == 324
    assert sum(row["cases"] == row["raw_exact"] for row in historical["profiles"].values()) == 302

    authored = 0
    rows = []
    for rule in rules:
        path, label = source(rule)
        case_path = RUFF / "rules" / rule / "cases.toml"
        if path is not None:
            authored += 1
            relative = path.relative_to(RUFF).as_posix()
            source_cell = f"[{label}]({relative}) `{sha(path)[:12]}`"
        else:
            source_cell = "absent"
        current_row = current["profiles"].get(rule)
        if current_row is not None:
            assert path is not None and label == "native query" and case_path.is_file()
            declared = next(item for item in descriptor["profiles"] if item["rule_id"] == rule)
            assert declared["query_sha256"] == sha(path)
            assert declared["cases_sha256"] == sha(case_path)
            assert current_row["intended_passes"] == current_row["cases"]
            assert (current_row["raw_exact"] + current_row["documented_differences"]
                    + current_row["metadata_asymmetries"] == current_row["cases"])
            current_cell = (
                f"verified {current_row['cases']}/{current_row['cases']}; "
                f"raw {current_row['raw_exact']}; documented {current_row['documented_differences']}; "
                f"metadata {current_row['metadata_asymmetries']}"
            )
            action = "Current fixture proof held"
        elif path is None:
            current_cell = "no current proof"
            action = "Author query and fixtures"
        elif not case_path.is_file():
            current_cell = "no current proof"
            action = "Add independent fixtures, then verify"
        else:
            case_count = len(tomllib.loads(case_path.read_text(encoding="utf-8"))["cases"])
            current_cell = f"source only ({case_count} fixtures)"
            action = "Verify against current engine and helper closure"

        old = historical["profiles"].get(rule)
        if old is None:
            historical_cell = "outside frozen 324"
        else:
            historical_cell = f"{old['raw_exact']}/{old['cases']} raw exact"
            if path is None or not case_path.is_file() or (
                sha(path) != old["query_sha256"] or sha(case_path) != old["cases_sha256"]
            ):
                historical_cell += "; query or cases changed"
            else:
                historical_cell += "; query and cases unchanged"
        rows.append(f"| {rule} | {source_cell} | {current_cell} | {historical_cell} | {action} |")

    assert authored == 370
    assert len(rows) == 413
    assert authored - len(current["profiles"]) == 232
    raw = sum(row["raw_exact"] for row in current["profiles"].values())
    documented = sum(row["documented_differences"] for row in current["profiles"].values())
    metadata = sum(row["metadata_asymmetries"] for row in current["profiles"].values())
    assert (raw, documented, metadata) == (3423, 77, 4)
    return "\n".join([
        "# Ruff 0.16.5 pinned rule inventory",
        "",
        "Generated by `python3 authoring/generate_rule_inventory.py`; check with `--check`. "
        "This is a source and proof view, not a second task ledger.",
        "",
        "The pinned default list has **413 rules**. Current source contains **370** rule queries or candidates; "
        "**138** are admitted and fixture-verified on sealed worker "
        f"`{current['worker_sha256']}`; **232** have source but await current proof; **43** have no query source. "
        "The current one-shot native run covers **3,504/3,504 intended cases**, "
        "with **3,423 raw report matches**, **77 documented diagnostic differences**, and "
        "**four UP025 reference metadata asymmetries**. Ruff execution telemetry is unavailable "
        "for all 3,504 comparisons. [Summary](results/native138-current-2026-11-02/summary.json) "
        "and [raw receipts](results/native138-current-2026-11-02/raw.tar.gz) bind the exact current source, "
        f"manifest `{current['manifest_sha256']}`, and public-base commit `{current['source_head']}`.",
        "",
        "The separate frozen **324-rule / 8,086-case** combined replay on worker "
        f"`{historical['worker_sha256']}` had **302 rules entirely raw exact**, "
        "7,862 exact cases and 224 mismatches. Its [historical per-rule summary]"
        "(results/correctness-324-per-rule-2026-11-01.json) and "
        "[source manifest](results/correctness-324-manifest-2026-11-01.json) are older evidence. "
        "Even an unchanged query and fixture needs current engine and shared-helper revalidation. "
        "The frozen 310-rule Django workload is a performance subset, not the native admission count.",
        "",
        "| Rule | Current source | Current 7a fixture result | Frozen 279a combined result | Next step |",
        "|---|---|---|---|---|",
        *rows,
        "",
    ])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="refuse a stale RULES.md")
    args = parser.parse_args()
    destination = RUFF / "RULES.md"
    rendered = render()
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
