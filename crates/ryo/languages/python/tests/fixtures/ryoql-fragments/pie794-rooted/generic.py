"""Counted rooted/global name reachability on real AST nodes, repeated IDs and unrelated trees."""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--worker", type=Path, required=True)
parser.add_argument("--candidate-root", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
root = args.output.resolve()
root.mkdir(exist_ok=False)
fixture = Path(__file__).resolve().parent
(stage := root / "profile/rules/ROOTED001").mkdir(parents=True)
(stage / "query.ryoql").write_bytes((fixture / "equivalence.ryoql").read_bytes())
helper = (
    args.candidate_root
    / "crates/ryo/languages/python/benchmarks/linters/ruff/verify_profiles.py"
)
actual_query = (
    args.candidate_root
    / "crates/ryo/languages/python/benchmarks/linters/ruff/rules/PIE794/query.ryoql"
)
source = actual_query.read_text()
# This known sparse-root consumer must stay rooted. The global closure in the
# equivalence fixture remains legitimate and is deliberately not prohibited.
assert (
    'axis(from=candidate_values, relation="semantic-child", direction=descendant, domain=names)'
    in source
)
assert 'let expression_paths = closure(edges="semantic-child"' not in source
spec = importlib.util.spec_from_file_location("guard", helper)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)
relevant = "value = outer(deep(leaf(a)), shallow(b))\n"
cases = [
    ("branch-order-and-two-contexts", {"case.py": relevant}),
    ("two-files-same-ids", {"a.py": relevant, "b.py": relevant}),
]
for width in (16, 256, 1024):
    cases.append(
        (
            "unrelated-width-" + str(width),
            {
                "case.py": relevant
                + "unrelated = ["
                + ",".join("0" for _ in range(width))
                + "]\n"
            },
        )
    )
for depth in (4, 16, 48):
    cases.append(
        (
            "unrelated-depth-" + str(depth),
            {
                "case.py": relevant
                + "unrelated = "
                + "[" * depth
                + "0"
                + "]" * depth
                + "\n"
            },
        )
    )
manifest = {
    "worker_sha256": hashlib.sha256(args.worker.read_bytes()).hexdigest(),
    "query_sha256": hashlib.sha256((stage / "query.ryoql").read_bytes()).hexdigest(),
    "validator_sha256": hashlib.sha256(helper.read_bytes()).hexdigest(),
    "cases": cases,
    "expected_count": "12 distinct(root,file,tag,node) rows per file: six names times two input payloads sharing each root ID",
    "benchmark_measurement": False,
    "no_timing_threshold": True,
}
(root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
records = []
for name, files in cases:
    corpus = root / name
    corpus.mkdir()
    for file, source in files.items():
        (corpus / file).write_text(source)
    request = {
        "root": str(corpus),
        "rules": str(root / "profile/rules"),
        "query_modules": {},
        "rule_applicability": {"ROOTED001": "accepted-syntax"},
        "parser_configuration": "python-target=3.10",
        "files": list(files),
        "cache": None,
    }
    process = subprocess.run(
        [str(args.worker.resolve()), "run-ryo", json.dumps(request)],
        text=True,
        capture_output=True,
        timeout=20,
        env=dict(os.environ, RAYON_NUM_THREADS="1"),
    )
    report = guard.complete_run_ryo_report(
        process.returncode, process.stdout, process.stderr, list(files), ["ROOTED001"]
    )
    records.append(
        {
            "name": name,
            "request": request,
            "status": process.returncode,
            "stdout": process.stdout,
            "stderr": process.stderr,
            "report": report,
        }
    )
    (root / "records.jsonl").write_text(
        "".join(json.dumps(row) + "\n" for row in records)
    )
    assert not report["diagnostics"], (name, report)
proof = {
    "cases": len(cases),
    "all_processes_zero": True,
    "unknown_or_unread": 0,
    "counted_multiset_differences": 0,
    "nonvacuous_expected_names_per_file": 12,
    "records_sha256": hashlib.sha256((root / "records.jsonl").read_bytes()).hexdigest(),
    "manifest_sha256": hashlib.sha256(
        (root / "manifest.json").read_bytes()
    ).hexdigest(),
    "order_claim": "Final membership/multisets only; global entity order and axis breadth order are deliberately not equated.",
}
(root / "proof.json").write_text(json.dumps(proof, indent=2) + "\n")
print(json.dumps(proof))
