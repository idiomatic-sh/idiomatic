"""Replay all SDK rule invocations and compare their typed answers in both orders."""
import argparse
import csv
from contextlib import nullcontext
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

EMPTY = hashlib.sha256(b"\n").hexdigest()


def invocations(trace):
    pending = {}
    with gzip.open(trace, "rt") as capture:
        for line in capture:
            record = json.loads(line)
            stamp = record["stamp"]
            if "rules" in record:
                pending[stamp] = record
            else:
                yield pending.pop(stamp), record
    if pending:
        raise ValueError(f"{len(pending)} unfinished SDK calls")


def compare(trace, binary, output, forward_input=None):
    expected = {}
    with tempfile.TemporaryFile(mode="w+") as receipts:
        with subprocess.Popen([binary, "--project"], stdin=subprocess.PIPE,
                              stdout=receipts, text=True) as process:
            for call, report in invocations(trace):
                corpus = call["corpus"]
                listed = corpus.get("listed", corpus.get("given"))
                inventory = sorted(set(listed).union(call["beside"])) if listed is not None else None
                partial = {unknown[0] for unknown in report["unknowns"]}
                for index, rule in enumerate(call["rules"]):
                    text = rule["query"]
                    tables = [t for t in call["supplied"] if json.dumps(t["name"]) in text]
                    ident = f"{call['stamp']}/{index}"
                    rows, sha = report["answers"].get(rule["id"], (0, EMPTY))
                    files = sorted(set(rule["admitted"]))
                    expected[ident] = (rule["id"], len(files), len(tables), rows, sha, rule["id"] in partial,
                                       report["unread"], report["skips"])
                    for order in ("forward", "reverse"):
                        request = dict(id=f"{ident}/{order}", query=text, parameters=rule["parameters"],
                                       files=files, inventory=inventory, hosted=tables)
                        if order == "reverse":
                            request["files"] = files[::-1]
                            request["hosted"] = [dict(t, rows=t["rows"][::-1]) for t in reversed(tables)]
                        packet = json.dumps(request, separators=(",", ":")) + "\n"
                        process.stdin.write(packet)
                        if order == "forward" and forward_input is not None:
                            forward_input.write(packet)
            process.stdin.close()
            if process.wait():
                raise RuntimeError("project executor failed")
        receipts.seek(0)
        answers = {}
        for line in receipts:
            record = json.loads(line)
            key = record.pop("id")
            if key in answers:
                raise ValueError(f"duplicate receipt {key}")
            answers[key] = record
    with output.open("w") as dest:
        w = csv.writer(dest, delimiter="\t", lineterminator="\n")
        w.writerow(("call/rule", "rule", "files", "tables", "shipping_rows", "sdk_unread", "sdk_skips", "status", "reason"))
        for ident, (rule, files, tables, rows, sha, partial, unread, skips) in expected.items():
            reason = ""
            forward, reverse = (answers.pop(f"{ident}/{order}") for order in ("forward", "reverse"))
            if forward != reverse:
                status, reason = "order dependent", json.dumps([forward, reverse])
            elif partial:
                status = "shipping partial" if "refused" in forward else "candidate answered partial reference"
                reason = forward.get("refused", "")
            elif "refused" in forward:
                status, reason = "refused", forward["refused"]
            elif forward == {"rows": rows, "sha256": sha}:
                status = "exact"
            else:
                status, reason = "different", json.dumps(forward)
            w.writerow((ident, rule, files, tables, rows, unread, skips, status, reason))
    if answers:
        raise ValueError("unrequested receipts")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trace", type=Path)
    parser.add_argument("executor", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("forward_input", type=Path, nargs="?", help="write the forward input stream for cold timing")
    args = parser.parse_args()
    with (args.forward_input.open("x") if args.forward_input else nullcontext()) as stream:
        compare(args.trace, args.executor, args.output, stream)
