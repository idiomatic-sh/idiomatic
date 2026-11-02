"""Compare wrapper boundaries and designated partial refusals with their old source forms."""

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--worker", type=Path, required=True)
parser.add_argument("--candidate-root", type=Path, required=True)
parser.add_argument(
    "--old-rule-root",
    type=Path,
    required=True,
    help="Frozen old F401 and PIE807 rule directory",
)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
fixture = Path(__file__).resolve().parent
bench = (
    args.candidate_root.resolve()
    / "crates/ryo/languages/python/benchmarks/linters/ruff"
)
root = args.output.resolve()
root.mkdir(exist_ok=False)
spec = importlib.util.spec_from_file_location("guard", bench / "verify_profiles.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
header = """input candidates: relation<id: node, file: file, span: span, line: number>;
param inline-pattern: string;
param range-pattern: string;
output relation<id: node, file: file, span: span>;
let candidate_locations = select id, file, span, line from candidates;
"""


def parameterized(body):
    body = re.sub(
        r'(let matching_codes = .*?matches\(codes, )".*"(\);)',
        r"\1$inline-pattern\2",
        body,
    )
    body = re.sub(
        r'(let supp_range_selected = .*?matches\(normalized, )".*"(\);)',
        r"\1$range-pattern\2",
        body,
    )
    body = re.sub(
        r'(let supp_nested_ignore_selected = .*?matches\(compact, )".*"(\);)',
        r"\1$range-pattern\2",
        body,
    )
    return body[: body.rindex("emit ")] + "return violations;\n"


old_unmapped = (args.old_rule_root / "PIE807.ryoql").read_text()
old_additional = (args.old_rule_root / "F401.ryoql").read_text()
modules = {
    "standard": ("suppress", (fixture / "baseline-72ea.ryoql").read_text()),
    "unmapped": (
        "suppress-unmapped",
        header
        + "let located = select id, file, span, line from candidate_locations;\n"
        + parameterized(old_unmapped[old_unmapped.index("let comments =") :]),
    ),
    "additional": (
        "suppress-with-additional-lines",
        header.replace(
            "param inline-pattern:",
            "input additional_locations: relation<id: node, file: file, line: number>;\nparam inline-pattern:",
        )
        + "let export_parent_lines = select id, file, line from additional_locations;\n"
        + parameterized(old_additional[old_additional.index("let quoted =") :]),
    ),
}
common_caller = """let native = select id, file, span from "semantic-syntax" where kind = "pass";
let located = select l.id as id, l.file as file, l.span as span, r."start-line" as line from native as l join "source-location" as r on l.id = r.id and l.file = r.file;
let candidates = union(left=located, right=located);
"""
additional_caller = """let parent_comments = select id, file from "comment";
let parent_positions = select l.file as file, r."start-line" as line from parent_comments as l join "source-location" as r on l.id = r.id;
let extra_locations = select l.id as id, l.file as file, r.line as line from candidates as l join parent_positions as r on l.file = r.file;
"""
profiles = {}
for mode, (logical, baseline) in modules.items():
    for side in ("baseline", "candidate"):
        stage = root / mode / side
        (stage / "rules/ANCHOR001").mkdir(parents=True)
        (stage / "fragments").mkdir()
        if side == "baseline":
            (stage / "fragments" / (logical + ".ryoql")).write_text(baseline)
            registry = {"python/ruff/" + logical: "fragments/" + logical + ".ryoql"}
        else:
            registry = {}
            for source in sorted((bench / "fragments").glob("suppress*.ryoql")):
                shutil.copyfile(source, stage / "fragments" / source.name)
                registry["python/ruff/" + source.stem] = "fragments/" + source.name
        caller = f'import suppress from "python/ruff/{logical}";\n' + common_caller
        if mode == "additional":
            caller += additional_caller
        caller += (
            "let violations = suppress(candidates=candidates, "
            + ("additional_locations=extra_locations, " if mode == "additional" else "")
            + 'inline-pattern="(?:^|[^A-Z0-9])ANCHOR001(?:$|[^A-Z0-9])", range-pattern="(?:^|,)(?:ANCHOR001)(?:,|$)");\nemit violations subject file key(id);\n'
        )
        (stage / "rules/ANCHOR001/query.ryoql").write_text(caller)
        profiles[mode, side] = (stage, registry)
cases = []
for original in json.loads(
    (fixture.parents[1] / "suppression-anchors/cases.json").read_text()
):
    for mode in ("standard", "unmapped"):
        cases.append({**original, "mode": mode})
cases += [
    {
        "name": "unmapped-does-not-follow-continuation",
        "mode": "unmapped",
        "files": {"case.py": "pass; x = \\\n 1  # noqa: ANCHOR001\n"},
        "expected_count": 1,
    },
    {
        "name": "standard-follows-continuation",
        "mode": "standard",
        "files": {"case.py": "pass; x = \\\n 1  # noqa: ANCHOR001\n"},
        "expected_count": 0,
    },
    {
        "name": "empty-additional-lines",
        "mode": "additional",
        "files": {"case.py": "pass\n"},
        "expected_count": 1,
    },
    {
        "name": "multiple-additional-lines-any-hidden",
        "mode": "additional",
        "files": {"case.py": "pass\n# noqa: ANCHOR001\n# parent without noqa\n"},
        "expected_count": 0,
    },
    {
        "name": "duplicate-hidden-id-lines",
        "mode": "additional",
        "files": {"case.py": "pass\n# noqa: ANCHOR001\n# noqa: ANCHOR001\n"},
        "expected_count": 0,
    },
    {
        "name": "additional-lines-wrong-code",
        "mode": "additional",
        "files": {"case.py": "pass\n# noqa: OTHER001\n# parent without noqa\n"},
        "expected_count": 1,
    },
    {
        "name": "additional-lines-two-files",
        "mode": "additional",
        "files": {
            "a.py": "pass\n# noqa: ANCHOR001\n",
            "b.py": "pass\n# parent without noqa\n",
        },
        "expected_count": 1,
    },
]
manifest = {
    "worker_sha256": sha(args.worker),
    "validator_sha256": sha(bench / "verify_profiles.py"),
    "cases": cases,
    "profiles": {
        mode + "/" + side: {
            "registry": registry,
            "query_sha256": sha(stage / "rules/ANCHOR001/query.ryoql"),
            "module_sha256": {
                logical: sha(stage / path) for logical, path in registry.items()
            },
        }
        for (mode, side), (stage, registry) in profiles.items()
    },
    "benchmark_measurement": False,
    "no_ruff_invocations": True,
}
(root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
records = []
for case in cases:
    corpus = root / "corpus"
    shutil.rmtree(corpus, ignore_errors=True)
    corpus.mkdir()
    for file, source in case["files"].items():
        (corpus / file).write_text(source)
    observations = {}
    for side in ("baseline", "candidate"):
        stage, registry = profiles[case["mode"], side]
        request = {
            "root": str(corpus),
            "rules": str(stage / "rules"),
            "query_modules": registry,
            "rule_applicability": {"ANCHOR001": "always"},
            "parser_configuration": "python-target=3.10",
            "files": list(case["files"]),
            "cache": None,
        }
        process = subprocess.run(
            guard.run_ryo_command(args.worker.resolve(), request),
            text=True,
            capture_output=True,
            timeout=20,
            env=dict(os.environ, RAYON_NUM_THREADS="1"),
        )
        observation = {
            "status": process.returncode,
            "stdout": process.stdout,
            "stderr": process.stderr,
        }
        if case.get("partial"):
            assert (
                process.returncode == 1
                and not process.stdout
                and process.stderr.startswith("incomplete candidate:")
            ), (case["name"], side, observation)
        else:
            observation["report"] = guard.complete_run_ryo_report(
                process.returncode,
                process.stdout,
                process.stderr,
                list(case["files"]),
                ["ANCHOR001"],
            )
            if "expected_count" in case:
                assert (
                    len(observation["report"]["diagnostics"]) == case["expected_count"]
                ), (case["name"], side, observation)
        observations[side] = observation
        records.append(
            {
                "name": case["name"],
                "mode": case["mode"],
                "side": side,
                "designated_partial": case.get("partial", False),
                "request": request,
                **observation,
            }
        )
        (root / "records.jsonl").write_text(
            "".join(json.dumps(row) + "\n" for row in records)
        )
    assert observations["baseline"] == observations["candidate"], (
        case["name"],
        case["mode"],
        observations,
    )
proof = {
    "cases": len(cases),
    "side_records": len(records),
    "full_report_differences": 0,
    "designated_partial_refusal_pairs": sum(
        bool(case.get("partial")) for case in cases
    ),
    "all_positive_reports_validated": True,
    "records_sha256": sha(root / "records.jsonl"),
    "manifest_sha256": sha(root / "manifest.json"),
}
(root / "proof.json").write_text(json.dumps(proof, indent=2) + "\n")
print(json.dumps(proof))
