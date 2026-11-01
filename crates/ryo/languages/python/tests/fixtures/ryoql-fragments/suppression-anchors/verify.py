"""Compare immutable suppression modules using an existing actual benchmark worker."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

RULE = "ANCHOR001"
BASELINE_SHA = "5ea3e0a1f045bf62b81e17c8c536a5ed47ba02859ff9544d769618c5fc762bde"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def operation_rows(events):
    operations = {}
    for event in events:
        if not any(span.get("rule") == RULE for span in event.get("spans", [])):
            continue
        for span in event.get("spans", []) + [event.get("span", {})]:
            if span.get("name") != "rule_step" or not span.get("operation_details"):
                continue
            step = span["step_index"]
            detail = span["operation_details"]
            assert operations.setdefault(step, detail) == detail, (step, detail)
    return operations


def step_input(operation, name):
    found = re.search(r"\b" + name + r": Step\(StepId\((\d+)\)\)", operation)
    assert found, (name, operation)
    return int(found.group(1))


def projection(operation):
    return [(side, int(column)) for side, column in re.findall(
        r"JoinColumn \{ side: (Left|Right), column: ColumnIndex\((\d+)\) \}", operation
    )]


def group_below(operations, step):
    while operations[step].startswith("Some(Project {"):
        step = step_input(operations[step], "from")
    assert operations[step].startswith("Some(GroupCount {"), operations[step]
    keys = re.search(r"keys: \[([^]]*)\]", operations[step]).group(1)
    assert len(re.findall(r"ColumnIndex", keys)) == 1, keys
    return step


def assert_observed_anchor_chains(events):
    """Assert actual executed compiled joins; do not infer absent unexecuted steps."""
    operations = operation_rows(events)
    expected = {
        "code": [("Right", 1), ("Right", 3), ("Right", 4)],
        "range": [("Left", 0), ("Left", 1), ("Left", 2), ("Left", 3),
                  ("Right", 5), ("Right", 6), ("Right", 3), ("Right", 4)],
    }
    result = {}
    for name, columns in expected.items():
        enriched = [step for step, operation in operations.items()
                    if operation.startswith("Some(Join {")
                    and "right: Relation(SourceLocation)" in operation
                    and projection(operation) == columns]
        assert len(enriched) == 1, (name, enriched)
        enrichment = enriched[0]
        token_join = step_input(operations[enrichment], "left")
        assert operations[token_join].startswith("Some(Join {"), operations[token_join]
        token_output = [("Right", index) for index in range(2 if name == "code" else 4)]
        assert projection(operations[token_join]) == token_output
        anchor_join = step_input(operations[token_join], "left")
        anchor = operations[anchor_join]
        assert anchor.startswith("Some(Join {") and projection(anchor) == [("Left", 0)], anchor
        candidate_files = step_input(anchor, "left")
        marker_files = step_input(anchor, "right")
        result[name] = dict(enrichment=enrichment, token_join=token_join, anchor_join=anchor_join,
                            candidate_files=candidate_files,
                            candidate_group=group_below(operations, candidate_files),
                            marker_group=group_below(operations, marker_files),
                            output_columns=columns)
    assert result["code"]["candidate_files"] == result["range"]["candidate_files"], result
    assert result["code"]["candidate_group"] == result["range"]["candidate_group"], result
    return result, operations


def assert_source_dependencies(baseline, candidate):
    """Pin every unchanged producer and inspect both complete source-DAG enrichment paths."""
    bindings = lambda text: dict(re.findall(r"^let ([a-z_]+) = (.*);$", text, re.M))
    before, after = bindings(baseline), bindings(candidate)
    changed = {"code_tokens", "code_token_ranges", "supp_source_tokens"}
    added = {"candidate_file_counts", "candidate_files", "file_suppression_file_counts",
             "file_suppression_files", "code_token_anchor_files", "anchored_code_tokens",
             "supp_range_comment_files", "supp_nested_head_files", "supp_marker_file_rows",
             "supp_marker_file_counts", "supp_marker_files", "supp_token_anchor_files",
             "supp_anchored_source_tokens"}
    assert set(after) - set(before) == added
    assert set(before) <= set(after)
    for name in set(before) - changed:
        assert before[name] == after[name], ("unchanged predicate/producer", name)
    assert after["code_tokens"] == before["code_tokens"].replace("select id from", "select id, file from", 1)
    assert after["code_token_ranges"] == before["code_token_ranges"].replace("from code_tokens as l", "from anchored_code_tokens as l", 1)
    assert after["supp_source_tokens"] == before["supp_source_tokens"].replace("from supp_all_source_tokens as l", "from supp_anchored_source_tokens as l", 1)
    for name, seed, tokens in [("anchored_code_tokens", "code_token_anchor_files", "code_tokens"),
                               ("supp_anchored_source_tokens", "supp_token_anchor_files", "supp_all_source_tokens")]:
        assert "from " + seed + " as l join " + tokens + " as r on l.file = r.file" in after[name]
    producers = sorted(name for name, expression in after.items() if 'join "source-location"' in expression)
    assert producers == sorted(name for name, expression in before.items() if 'join "source-location"' in expression)
    return {"unchanged_bindings": len(before) - len(changed), "changed_bindings": sorted(changed),
            "added_bindings": sorted(added), "all_source_location_producers": producers}


def run(worker, directory, request, diagnostics, allow_incomplete=False):
    request = dict(request, cache=tempfile.mkdtemp(prefix="fresh-cache-", dir=directory))
    request_path = directory / "request.json"
    request_bytes = json.dumps(request, sort_keys=True).encode()
    request_path.write_bytes(request_bytes)
    command = [str(worker), "diagnostic-ryo" if diagnostics else "run-ryo",
               "--request-file", str(request_path), "python-target=3.10"]
    environment = os.environ.copy()
    if diagnostics:
        command.append(str(directory / "events.jsonl"))
        environment.update(RYO_DIAGNOSTIC_STEPS="1", RYO_DIAGNOSTIC_JOINS="1", RYO_DIAGNOSTIC_PREFIXES="1")
    completed = subprocess.run(command, capture_output=True, text=True, env=environment, timeout=30)
    (directory / "stdout").write_text(completed.stdout)
    (directory / "stderr").write_text(completed.stderr)
    if completed.returncode:
        assert allow_incomplete and completed.returncode == 1 and not completed.stdout, (command, completed.returncode, completed.stderr)
        assert completed.stderr.startswith("incomplete candidate:"), completed.stderr
        report = {"protocol_error": {"returncode": completed.returncode, "stderr": completed.stderr}}
    else:
        report = json.loads(completed.stdout)
    events = ([json.loads(line) for line in (directory / "events.jsonl").read_text().splitlines()]
              if diagnostics else [])
    return report, events, digest(request_bytes)


def stage(root, label, module, query):
    profile = root / label
    (profile / "rules" / RULE).mkdir(parents=True, exist_ok=True)
    (profile / "fragments").mkdir(exist_ok=True)
    (profile / "rules" / RULE / "query.ryoql").write_text(query)
    (profile / "fragments" / "suppress.ryoql").write_bytes(module)
    return profile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", required=True, type=Path)
    parser.add_argument("--baseline-module", required=True, type=Path)
    parser.add_argument("--candidate-module", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    modules = {"baseline": args.baseline_module.read_bytes(), "candidate": args.candidate_module.read_bytes()}
    assert digest(modules["baseline"]) == BASELINE_SHA, "baseline bytes must remain frozen"
    source_guard = assert_source_dependencies(modules["baseline"].decode(), modules["candidate"].decode())
    query = (fixture / "caller.ryoql").read_text()
    profiles = {label: stage(args.output, label, module, query) for label, module in modules.items()}
    receipts = []
    plan = None
    negative_gates = {}
    for case in json.loads((fixture / "cases.json").read_text()):
        reports = {}
        for label, profile in profiles.items():
            directory = args.output / case["name"] / label
            corpus = directory / "corpus"
            corpus.mkdir(parents=True, exist_ok=True)
            for name, source in case["files"].items():
                path = corpus / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(source)
            request = dict(root=str(corpus), rules=str(profile / "rules"),
                           query_modules={"python/ruff/suppress": "fragments/suppress.ryoql"},
                           rule_applicability={RULE: "always"}, parser_configuration="python-target=3.10",
                           files=sorted(case["files"]), cache=str(directory / "fresh-cache"))
            diagnose = case.get("plan", False) or case["name"] in {"no-markers", "no-candidates"}
            report, events, request_sha = run(args.worker, directory, request, diagnose, case.get("partial", False))
            reports[label] = report
            receipts.append(dict(case=case["name"], variant=label, report=report, request_sha256=request_sha,
                                 source_sha256={name: digest(text.encode()) for name, text in case["files"].items()}))
            if label == "candidate" and case.get("plan"):
                plan, operations = assert_observed_anchor_chains(events)
                (directory / "operations.json").write_text(json.dumps(operations, indent=2) + "\n")
            if label == "candidate" and case["name"] in {"no-markers", "no-candidates"}:
                assert plan is not None, "positive compiled-chain fixture must run first"
                counts = {name: dict(events=0, builds=0, probes=0, hits=0) for name in plan}
                for event in events:
                    if event.get("target") != "ryo::diagnostic::stored_join":
                        continue
                    active = next((span.get("step_index") for span in reversed(event.get("spans", []))
                                   if span.get("name") == "rule_step"), None)
                    for name, gate in plan.items():
                        if active == gate["enrichment"]:
                            counts[name]["events"] += 1
                            for field in ["builds", "probes", "hits"]:
                                counts[name][field] += event["fields"][field]
                assert all(count["builds"] == count["probes"] == count["hits"] == 0 for count in counts.values()), counts
                negative_gates[case["name"]] = counts
        assert reports["baseline"] == reports["candidate"], (case["name"], reports)
        if case.get("partial"):
            refusal = reports["baseline"]["protocol_error"]["stderr"]
            assert int(re.search(r"unknown_count=(\d+)", refusal).group(1)) > 0, (case["name"], reports)
    # Probe each production anchor before enrichment. Duplicate findings/markers must not duplicate tokens.
    positive = json.loads((fixture / "cases.json").read_text())[0]
    for binding in ["anchored_code_tokens", "supp_anchored_source_tokens"]:
        probe = modules["candidate"].decode().replace("return violations;", '''let anchor_probe_rows = select l.id as id, l.file as file, r.span as span from ''' + binding + ''' as l join "source-location" as r on l.id = r.id;
let anchor_probe_counts = group-count(from=anchor_probe_rows, keys=["id", "file", "span"], count="copies");
let anchor_probe_bad = select id, file, span from anchor_probe_counts where copies != 1;
return anchor_probe_bad;''')
        directory = args.output / "token-multiplicity" / binding
        profile = stage(directory, "profile", probe.encode(), query)
        corpus = directory / "corpus"
        corpus.mkdir(exist_ok=True)
        (corpus / "case.py").write_text(positive["files"]["case.py"])
        request = dict(root=str(corpus), rules=str(profile / "rules"),
                       query_modules={"python/ruff/suppress": "fragments/suppress.ryoql"},
                       rule_applicability={RULE: "always"}, parser_configuration="python-target=3.10",
                       files=["case.py"], cache=str(directory / "fresh-cache"))
        report, _, request_sha = run(args.worker, directory, request, False)
        assert report["diagnostics"] == [] and report["coverage"]["unknown_or_unread"] == 0, (binding, report)
        receipts.append(dict(probe=binding, report=report, request_sha256=request_sha, probe_module_sha256=digest(probe.encode())))
    proof = dict(schema="suppression-anchor-fixture-proof/v1", worker=str(args.worker),
                 baseline_module_sha256=digest(modules["baseline"]), candidate_module_sha256=digest(modules["candidate"]),
                 caller_sha256=digest(query.encode()), cases_sha256=digest((fixture / "cases.json").read_bytes()),
                 fixture_cases=8, full_report_pairs=6, partial_refusal_pairs=2, multiplicity_probes=2,
                 observed_compiled_chains=plan, source_graph_guard=source_guard, negative_gate_counters=negative_gates, receipts=receipts,
                 worker_sha256=digest(args.worker.read_bytes()),
                 limitation="Step tracing asserts executed compiled chains. It cannot enumerate all unexecuted retained plan steps.")
    (args.output / "proof.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps({key: proof[key] for key in ["fixture_cases", "full_report_pairs", "partial_refusal_pairs", "multiplicity_probes", "candidate_module_sha256"]}))


if __name__ == "__main__":
    main()
