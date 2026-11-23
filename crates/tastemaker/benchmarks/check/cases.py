"""Verify every registered query against its frozen typed rows and unknowns."""
from collections import Counter, defaultdict
import json
from pathlib import Path
import subprocess
import tempfile

from measure import candidate, compare
from packets import Packets, digest, line, row_digest, safe_path

FIXTURES = Path(__file__).resolve().parent / "tests/fixtures"


def cases():
    cases = []
    registered = json.loads((FIXTURES / "registry.json").read_bytes())
    discovered = [str(p.relative_to(FIXTURES)) for p in (FIXTURES / "rules").glob("*/cases/*.json")]
    if sorted(registered) != sorted(discovered):
        raise ValueError("case files differ from the committed inventory")
    rule_dirs = sorted((FIXTURES / "rules").iterdir())
    for directory in rule_dirs:
        query = (directory / "query.ryoql").read_bytes()
        if digest(query)[:12] != directory.name.rsplit("-", 1)[1]:
            raise ValueError(f"changed query bytes: {directory}")
        registrations = sorted((directory / "cases").glob("*.json"))
        if not registrations:
            raise ValueError(f"missing registered cases: {directory}")
        for path in registrations:
            case = json.loads(path.read_bytes())
            selected = [p for p in case["cohort"] if p["id"] == case["rule"]]
            if len(selected) != 1 or selected[0]["query"] != directory.name:
                raise ValueError(f"case does not select its registered rule: {path}")
            cases.append(case)
    cases.sort(key=lambda c: c["stamp"])
    if [c["stamp"] for c in cases] != list(range(len(cases))):
        raise ValueError("missing or duplicate case ordinal")
    return cases


def sources(entries):
    result = {}
    for name, key in entries.items():
        if len(key) != 64 or any(c not in "0123456789abcdef" for c in key):
            raise ValueError("invalid source content digest")
        value = (FIXTURES / "resources" / key[:2] / (key + ".txt")).read_bytes()
        if digest(value) != key:
            raise ValueError(f"source digest mismatch: {name}")
        result[name] = value.decode()
    return result


def inputs(c, root):
    rules = []
    for peer in c["cohort"]:
        query = safe_path(FIXTURES / "rules", peer["query"]) / "query.ryoql"
        rules.append(dict(peer, query=query.read_bytes().decode(), text=str(query), admitted=c["admitted"]))
    return dict(stamp=c["stamp"], root=str(root), corpus={"given": sources(c["given"])},
                beside=sources(c["beside"]), rules=rules, supplied=c["hosted"])


def verify(workers, reference_only=False):
    registry = cases()
    release = {}
    candidate_runs = []
    with tempfile.TemporaryDirectory() as tmp:
        directory = Path(tmp)
        calls = [inputs(c, directory) for c in registry]
        manifest = directory / "shipping.jsonl"
        with manifest.open("w") as stream:
            for call in calls:
                stream.write(line(dict(call, given=call["corpus"]["given"], hosted=call["supplied"])))
        typed = directory / "typed.jsonl"
        subprocess.run([workers / "tm-case-reference", manifest, directory / "hashes", typed], check=True)
        for raw in typed.read_text().splitlines():
            receipt = json.loads(raw)
            stamp = receipt["stamp"]
            c = registry[stamp]
            if c["rule"] != receipt["rule"]:
                continue
            if stamp in release:
                raise ValueError(f"duplicate released receipt {stamp}")
            reference = c["expected"]
            release[stamp] = receipt["rows"] == reference["rows"] and sorted(receipt["unknowns"]) == reference["unknowns"]
        if len(release) != len(registry):
            raise ValueError("missing released case receipt")
        if not reference_only:
            for reverse in (False, True):
                stage = directory / ("reverse" if reverse else "forward")
                stage.mkdir()
                packets = Packets(stage)
                expected = {}
                with packets.project.open("w") as project, packets.mixed.open("w") as mixed:
                    for c, call in zip(registry, calls, strict=True):
                        packets.append(project, mixed, call, c["rule"], reverse)
                        index = next(i for i, r in enumerate(c["cohort"]) if r["id"] == c["rule"])
                        ref = c["expected"]
                        expected[f'{c["stamp"]}/{index}'] = dict(rows=len(ref["rows"]), sha256=row_digest(ref["rows"]), unknowns=ref["unknowns"])
                for _ in range(3):
                    _, receipts = candidate(workers / "tm-check-reader", packets, timed=False)
                    candidate_runs.append(list(compare(expected, receipts).values()))
    classes = defaultdict(Counter)
    for i, c in enumerate(registry):
        counts = classes[c["dependency"]]
        counts["total"] += 1
        counts["released"] += release[i]
        if candidate_runs:
            first = candidate_runs[0][i]
            status = "unstable" if any(run[i] != first for run in candidate_runs[1:]) else first[0]
            counts[status] += 1
            if status != "exact":
                print(f'case {i:03d} {c["rule"]} {status}')
    print("Class | released | candidate | total")
    for name, counts in sorted(classes.items()):
        print(f'{name} | {counts["released"]} | {counts["exact"] if candidate_runs else "—"} | {counts["total"]}')
    total = len(registry)
    passed = sum(v["exact"] for v in classes.values())
    print(f"Tastemaker released rule cases: {sum(release.values())} of {total} pass")
    if candidate_runs:
        print(f"Tastemaker rule cases: {passed} of {total} pass")
    return all(release.values()) and (reference_only or passed == total)
