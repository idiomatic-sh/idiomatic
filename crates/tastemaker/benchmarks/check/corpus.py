"""Capture today's CI inputs and measure the release and candidate on one committed checkout."""
from collections import Counter
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import tempfile
import tomllib

from measure import candidate, compare, file_sha, process
from packets import Packets, capture_pairs, row_digest

WORKSPACES = ("ryo", "tastemaker", "asymptote", "root")


def shipping(cwd, binary, output, trace=None):
    store = cwd / ".tastemaker_cache"
    if store.exists():
        raise ValueError(f"expected a fresh store at {store}")
    tools = tomllib.loads((cwd / "asymptote.toml").read_text())["tools"]
    command = next(t["run"] for t in tools if t["id"] == "tastemaker")
    if command != ["tm", "check", "--format", "sarif"]:
        raise ValueError(f"unexpected CI workload {command!r}")
    environment = dict(os.environ)
    environment.pop("TM_BENCH_TRACE", None)
    if trace:
        environment["TM_BENCH_TRACE"] = str(trace)
    try:
        with output.open("wb") as out, tempfile.TemporaryFile() as errors:
            metrics = process([binary, *command[1:]], cwd=cwd, stdout=out, stderr=errors,
                              env=environment, allowed=(0, 1))
        document = json.loads(output.read_bytes())
        results = [r for run in document["runs"] for r in run["results"]]
        suppressed = sum(bool(r.get("suppressions")) for r in results)
        return dict(metrics, findings=len(results) - suppressed, suppressed=suppressed,
                    report_sha256=file_sha(output))
    finally:
        if store.is_dir():
            shutil.rmtree(store)


def prepare(packets, trace):
    expected = {}
    with packets.manifest.open("w") as stream:
        for call, report in capture_pairs(trace):
            packets.append(stream, call)
            unknown = {r[0] for r in report["unknowns"]}
            for i, rule in enumerate(call["rules"]):
                identity = f'{call["stamp"]}/{i}'
                if identity in expected:
                    raise ValueError(f"duplicate request {identity}")
                n, sha = report["answers"].get(rule["id"], (0, row_digest([])))
                expected[identity] = dict(rows=n, sha256=sha, unknowns=rule["id"] in unknown)
    if not expected:
        raise ValueError("empty SDK trace")
    return expected


def summary(runs):
    return {k: statistics.median(r[k] for r in runs) for k in ("cpu_s", "wall_s", "peak_mib")}


def run_workspace(name, checkout, reader, observer_binary, tm, repeats, storage):
    with tempfile.TemporaryDirectory(dir=storage) as tmp:
        directory = Path(tmp)
        cwd = checkout if name == "root" else checkout / "crates" / name
        raw = directory / "trace"
        report = directory / "report.sarif"
        print(f"Capturing {name} SDK inputs", file=sys.stderr, flush=True)
        observer = shipping(cwd, observer_binary, report, raw)
        packets = Packets(directory)
        expected = prepare(packets, raw)
        raw.unlink()
        attempts = []
        seen = {}
        for repeat in range(repeats):
            run = {}
            for engine in (("shipping", "candidate") if repeat % 2 == 0 else ("candidate", "shipping")):
                print(f"{name} {engine} {repeat + 1}/{repeats}", file=sys.stderr, flush=True)
                if engine == "shipping":
                    run[engine] = shipping(cwd, tm, report)
                    if run[engine]["report_sha256"] != observer["report_sha256"]:
                        raise ValueError(f"observer's complete SARIF differs from release on {name}")
                else:
                    run[engine], actual = candidate(reader, packets)
                    for key, (status, receipt) in compare(expected, actual).items():
                        # Different refusal messages can describe the same unavailable
                        # operation. Completed rows must remain byte-equal on every run.
                        value = (status, receipt)
                        previous = seen.setdefault(key, value)
                        if previous != value:
                            seen[key] = ("unstable", None)
            attempts.append(run)
        counts = Counter(value[0] for value in seen.values())
        if sum(counts.values()) != len(expected):
            raise ValueError("incomplete candidate receipt coverage")
        return dict(workspace=name, requests=len(expected), outcomes=counts, repeats=attempts,
                    shipping=summary([r["shipping"] for r in attempts]),
                    candidate=summary([r["candidate"] for r in attempts]), capture=observer)


def run(root, reader, observer, tm, names, repeats, output):
    if repeats < 1:
        raise ValueError("at least one cold process per engine is required")
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    pin = tomllib.loads((root / "tastemaker.toml").read_text())["tastemaker"]
    for binary in (tm, observer):
        version = subprocess.check_output([binary, "--version"], text=True).strip()
        if version.split()[:2] != ["tm", pin]:
            raise ValueError(f"{binary} reports {version!r}, workspace requires tm {pin}")
    result = dict(commit=revision, version=pin, ryo_whole_check_target_cpu_s=5, workers=1,
                  binaries={"release": file_sha(tm), "observer": file_sha(observer), "candidate": file_sha(reader)}, workspaces=[])
    with tempfile.TemporaryDirectory(prefix="tm-corpus-") as tmp:
        storage = Path(tmp)
        checkout = storage / "checkout"
        subprocess.run(["git", "clone", "--quiet", "--local", "--shared", "--no-checkout", root, checkout], check=True)
        subprocess.run(["git", "-C", checkout, "checkout", "--quiet", "--detach", revision], check=True)
        print(f"Tastemaker {revision}; one worker; whole Ryo check target <= 5 CPU-s", flush=True)
        print("workspace | exact/total | different | refused | partial | unstable | candidate CPU/wall/peak MiB | release CI CPU/wall/peak MiB", flush=True)
        for name in names:
            record = run_workspace(name, checkout, reader, observer, tm, repeats, storage)
            result["workspaces"].append(record)
            counts = record["outcomes"]
            metrics = ["/".join(f'{record[e][k]:.3f}' for k in ("cpu_s", "wall_s", "peak_mib"))
                       for e in ("candidate", "shipping")]
            print(" | ".join([name, f'{counts["exact"]}/{record["requests"]}',
                  *[str(counts[k]) for k in ("different", "refused", "partial", "unstable")], *metrics]), flush=True)
            if output:
                output.write_text(json.dumps(result, indent=2) + "\n")
    return result
