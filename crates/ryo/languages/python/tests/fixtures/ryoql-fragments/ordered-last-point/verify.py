"""Check ordered self-matching against the former quadratic last-point expression."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

WORKER_RULE = "MAX001"
CASES = {
    "empty": {"a.py": []},
    "singleton": {"a.py": [7]},
    "gaps": {"a.py": [1, 3, 99]},
    "multiple-files": {"a.py": [2, 7], "b.py": [0, 100]},
    "duplicate-points": {"a.py": [5, 5, 7, 7], "b.py": [3, 3]},
}


def query_for(files):
    query = '''let root_nodes = select id, file, span from "semantic-syntax" where kind = "module";
let file_roots = select file from root_nodes;
'''
    inputs = []
    for filename, points in files.items():
        for point in points:
            index = len(inputs)
            query += f'let roots_{index} = select file from file_roots where matches(file, {json.dumps(filename.replace(".", "[.]") + "$")});\n'
            query += f'let row_{index} = extend(from=roots_{index}, values={{point: {point}}});\n'
            query += f'let point_{index} = select point, file from row_{index};\n'
            inputs.append(f"point_{index}")
    if inputs:
        previous = inputs[0]
        for index, source in enumerate(inputs[1:]):
            query += f'let rows_{index} = union(left={previous}, right={source});\n'
            previous = f"rows_{index}"
        query += f'let file_event_points = select point, file from {previous};\n'
    else:
        query += '''let zero = extend(from=file_roots, values={point: 0});
let file_event_points = select point, file from zero where point < 0;
'''
    result = query + '''let old_older = select l.point as point, l.file as file from file_event_points as l join file_event_points as r on l.file = r.file where l.point < r.point;
let expected = antijoin(left=file_event_points, right=old_older, left-on=["point","file"], right-on=["point","file"]);
let new_older = ordered-match(left=file_event_points, right=file_event_points, left-on=["file"], right-on=["file"], left-order="point", right-order="point", output={point:[left,point],file:[left,file]});
let actual = antijoin(left=file_event_points, right=new_older, left-on=["point","file"], right-on=["point","file"]);
let expected_counts = group-count(from=expected, keys=["point","file"], count="copies");
let actual_counts = group-count(from=actual, keys=["point","file"], count="copies");
let missing = antijoin(left=expected_counts, right=actual_counts, left-on=["point","file","copies"], right-on=["point","file","copies"]);
let extra = antijoin(left=actual_counts, right=expected_counts, left-on=["point","file","copies"], right-on=["point","file","copies"]);
let differences = union(left=missing, right=extra);
let different_files = select file from differences;
let actual_files = select file from actual;
let missing_files = antijoin(left=file_roots, right=actual_files, left-on=["file"], right-on=["file"]);
let actual_file_counts = group-count(from=actual, keys=["file"], count="copies");
let wrong_cardinality = select file from actual_file_counts where copies != 1;
let cardinality_failures = union(left=missing_files, right=wrong_cardinality);
let failure_files = union(left=different_files, right=cardinality_failures);
let bad_roots = select r.id as id, r.file as file, r.span as span from failure_files as l join root_nodes as r on l.file = r.file;
emit bad_roots subject file key(id);
'''
    if not inputs:
        result = result.replace('let missing_files = antijoin(left=file_roots, right=actual_files, left-on=["file"], right-on=["file"]);', 'let missing_files = select file from actual;')
    return result


def compiled_dedup(events):
    operations = {}
    for line in events.read_text().splitlines():
        event = json.loads(line)
        if not any(span.get("rule") == WORKER_RULE for span in event.get("spans", [])):
            continue
        span = event.get("span", {})
        if span.get("operation_details"):
            operations[span["step_index"]] = span["operation_details"]
    matches = []
    for step, operation in operations.items():
        if not operation.startswith("Some(OrderedMatch {"):
            continue
        left = re.search(r"left: Step\(StepId\((\d+)\)\)", operation)
        right = re.search(r"right: Step\(StepId\((\d+)\)\)", operation)
        assert left and right and left.group(1) == right.group(1), operation
        input_step = int(left.group(1))
        source = operations[input_step]
        assert source.startswith("Some(Project {") and "distinct: Dropped" in source, source
        matches.append(dict(step=step, input_step=input_step, input_operation=source, operation=operation))
    assert len(matches) == 1, matches
    return matches[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    records = []
    for name, files in CASES.items():
        directory = args.output / name
        corpus = directory / "corpus"
        rules = directory / "rules" / WORKER_RULE
        rules.mkdir(parents=True, exist_ok=True)
        corpus.mkdir(exist_ok=True)
        for filename in files:
            (corpus / filename).write_text("pass\n")
        query = query_for(files)
        (rules / "query.ryoql").write_text(query)
        request = dict(root=str(corpus), rules=str(rules.parent), query_modules={},
                       rule_applicability={WORKER_RULE: "always"}, parser_configuration="python-target=3.10",
                       files=sorted(files), cache=tempfile.mkdtemp(prefix="fresh-cache-", dir=directory))
        request_bytes = json.dumps(request, sort_keys=True).encode()
        request_path = directory / "request.json"
        request_path.write_bytes(request_bytes)
        events = directory / "events.jsonl"
        environment = dict(os.environ, RYO_DIAGNOSTIC_STEPS="1", RYO_DIAGNOSTIC_PREFIXES="1")
        completed = subprocess.run([str(args.worker), "diagnostic-ryo", "--request-file", str(request_path),
                                    "python-target=3.10", str(events)], text=True, capture_output=True,
                                   env=environment, timeout=30)
        (directory / "stdout").write_text(completed.stdout)
        (directory / "stderr").write_text(completed.stderr)
        assert completed.returncode == 0, (name, completed.stderr)
        report = json.loads(completed.stdout)
        assert not report["diagnostics"] and not report["skipped"] and report["coverage"]["unknown_or_unread"] == 0, (name, report)
        dedup = compiled_dedup(events) if name == "duplicate-points" else None
        records.append(dict(case=name, points=files, report=report, compiled_dedup=dedup,
                            query_sha256=hashlib.sha256(query.encode()).hexdigest(),
                            request_sha256=hashlib.sha256(request_bytes).hexdigest()))
    proof = dict(schema="ordered-last-point-equivalence/v1", cases=len(CASES), records=records,
                 worker_sha256=hashlib.sha256(args.worker.read_bytes()).hexdigest(),
                 comparison="Typed point/file rows and multiplicities, via symmetric antijoins of per-row counts",
                 order_argument="The final antijoin's left input is identical; only membership on its right changes. The antijoin retains left traversal order.")
    (args.output / "proof.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps({"cases": len(CASES), "all_equivalent": True}))


if __name__ == "__main__":
    main()
