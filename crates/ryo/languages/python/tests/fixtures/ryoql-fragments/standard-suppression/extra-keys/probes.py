"""Compare full-row suppression with the hidden-ID adapter, retaining duplicate diagnostics."""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--worker", type=Path, required=True)
parser.add_argument("--candidate-root", type=Path, required=True)
parser.add_argument("--baseline-root", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
root = args.output.resolve()
root.mkdir(exist_ok=False)
relative = Path("crates/ryo/languages/python/benchmarks/linters/ruff")
candidate = args.candidate_root.resolve() / relative
baseline = args.baseline_root.resolve() / relative
spec = importlib.util.spec_from_file_location(
    "report_guard", candidate / "verify_profiles.py"
)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
source = "target = left + right\n"
cases = [
    ("duplicate-id-extra-keys", {"case.py": source}, 2),
    ("duplicate-id-noqa", {"case.py": source.rstrip() + "  # noqa: SIM220\n"}, 0),
    ("duplicate-id-wrong-noqa", {"case.py": source.rstrip() + "  # noqa: SIM221\n"}, 2),
    ("two-files-same-shape", {"case.py": source, "other.py": source}, 4),
    (
        "two-files-one-suppressed",
        {"case.py": source, "other.py": source.rstrip() + "  # noqa: SIM220\n"},
        2,
    ),
    ("duplicate-id-block", {"case.py": "# ruff: disable[SIM220]\n" + source}, 0),
]
prefix = """let seeds = select id, file, span from "identifier" where name = "target";
let tags = select id, file from "identifier" where name = "left" or name = "right";
let reportable = select l.id as id, l.file as file, l.span as span, r.id as unary, r.id as negated_name, r.id as positive_name from seeds as l join tags as r on l.file = r.file;
let candidate_locations = select l.id as id, l.file as file, l.span as span, l.unary as unary, l.negated_name as negated_name, l.positive_name as positive_name, r."start-line" as line from reportable as l join "source-location" as r on l.id = r.id;
"""
old = (baseline / "rules/SIM220/candidate.ryoql").read_text()
new = (candidate / "rules/SIM220/candidate.ryoql").read_text()
queries = {
    "baseline": prefix + old[old.index("let quoted =") :],
    "candidate": 'import suppress from "python/ruff/suppress";\n'
    + prefix
    + new[new.index("let suppression_candidates =") :],
}
profiles = {}
for side, query in queries.items():
    stage = root / side
    (stage / "rules/SIM220").mkdir(parents=True)
    (stage / "rules/SIM220/query.ryoql").write_text(query)
    (stage / "fragments").mkdir()
    shutil.copyfile(
        candidate / "fragments/suppress.ryoql", stage / "fragments/suppress.ryoql"
    )
    profiles[side] = stage
manifest = {
    "worker_sha256": digest(args.worker),
    "validator_sha256": digest(candidate / "verify_profiles.py"),
    "helper_sha256": digest(candidate / "fragments/suppress.ryoql"),
    "query_sha256": {
        side: digest(stage / "rules/SIM220/query.ryoql")
        for side, stage in profiles.items()
    },
    "module_registry": {"python/ruff/suppress": "fragments/suppress.ryoql"},
    "cases": cases,
    "benchmark_measurement": False,
    "no_ruff_invocations": True,
}
(root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
records = []
for name, files, expected_count in cases:
    corpus = root / "corpus"
    shutil.rmtree(corpus, ignore_errors=True)
    corpus.mkdir()
    for file, text in files.items():
        (corpus / file).write_text(text)
    reports = {}
    for side, stage in profiles.items():
        request = {
            "root": str(corpus),
            "rules": str(stage / "rules"),
            "files": list(files),
            "cache": None,
            "query_modules": manifest["module_registry"],
            "rule_applicability": {"SIM220": "accepted-syntax"},
            "parser_configuration": "python-target=3.10",
        }
        result = subprocess.run(
            [str(args.worker.resolve()), "run-ryo", json.dumps(request)],
            text=True,
            capture_output=True,
            timeout=20,
            env=dict(os.environ, RAYON_NUM_THREADS="1"),
        )
        report = guard.complete_run_ryo_report(
            result.returncode, result.stdout, result.stderr, list(files), ["SIM220"]
        )
        reports[side] = report
        records.append(
            {
                "case": name,
                "side": side,
                "request": request,
                "status": result.returncode,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "report": report,
            }
        )
        (root / "records.jsonl").write_text(
            "".join(json.dumps(row) + "\n" for row in records)
        )
    assert reports["baseline"] == reports["candidate"], name
    assert len(reports["candidate"]["diagnostics"]) == expected_count, (name, reports)
    for file in files:
        rows = [
            row for row in reports["candidate"]["diagnostics"] if row["file"] == file
        ]
        assert not rows or len(rows) == 2 and rows[0] == rows[1], (name, file, rows)
proof = {
    "cases": len(cases),
    "side_records": len(records),
    "all_process_statuses_zero": True,
    "full_report_differences": 0,
    "unknown_or_unread": 0,
    "extra_key_duplicate_diagnostics_retained": True,
    "records_sha256": digest(root / "records.jsonl"),
    "manifest_sha256": digest(root / "manifest.json"),
}
(root / "proof.json").write_text(json.dumps(proof, indent=2) + "\n")
print(json.dumps(proof))
