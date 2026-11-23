"""Capture today's CI inputs and measure the release and candidate on one committed checkout."""
from collections import Counter, defaultdict
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import tempfile
import tomllib

from measure import READER_PARTS, SESSION_PARTS, candidate, compare, file_sha, process, retained_files
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
        output_bytes = output.stat().st_size
        store_bytes, store_files = retained_files(store)
        return dict(metrics, output_bytes=output_bytes, store_bytes=store_bytes, store_files=store_files,
                    findings=len(results) - suppressed, suppressed=suppressed, report_sha256=file_sha(output))
    finally:
        if store.is_dir():
            shutil.rmtree(store)


def prepare(packets, trace):
    expected = {}
    with packets.manifest.open("w") as stream:
        for call, report in capture_pairs(trace):
            packets.append(stream, call)
            unknown = defaultdict(list)
            for rule_id, path, message in report["unknowns"]:
                unknown[rule_id].append((path, message))
            for i, rule in enumerate(call["rules"]):
                identity = f'{call["stamp"]}/{i}'
                if identity in expected:
                    raise ValueError(f"duplicate request {identity}")
                n, sha = report["answers"].get(rule["id"], (0, row_digest([])))
                expected[identity] = dict(rows=n, sha256=sha, unknowns=unknown.get(rule["id"], ()))
    if not expected:
        raise ValueError("empty SDK trace")
    return expected


def summary(runs):
    result = {k: statistics.median(r[k] for r in runs) for k in ("cpu_s", "wall_s", "peak_mib")}
    middle = sorted(runs, key=lambda r: r["cpu_s"])[(len(runs) - 1) // 2]
    return dict(result, **{k: middle[k] for k in ("output_bytes", "store_bytes", "store_files")})


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
    result = dict(commit=revision, version=pin, ryo_query_session_target_cpu_s=5, workers=1,
                  binaries={"release": file_sha(tm), "observer": file_sha(observer), "candidate": file_sha(reader)}, workspaces=[])
    with tempfile.TemporaryDirectory(prefix="tm-corpus-") as tmp:
        storage = Path(tmp)
        checkout = storage / "checkout"
        subprocess.run(["git", "clone", "--quiet", "--local", "--shared", "--no-checkout", root, checkout], check=True)
        subprocess.run(["git", "-C", checkout, "checkout", "--quiet", "--detach", revision], check=True)
        print(f"Tastemaker {revision}; one worker; complete, equal Ryo query session target <= 5 CPU-s", flush=True)
        print("workspace | exact/total | different | refused | partial | unstable | candidate CPU/wall/peak MiB | release CI CPU/wall/peak MiB | candidate receipt bytes | candidate store bytes/files | release SARIF bytes | release store bytes/files", flush=True)
        for name in names:
            record = run_workspace(name, checkout, reader, observer, tm, repeats, storage)
            result["workspaces"].append(record)
            counts = record["outcomes"]
            metrics = ["/".join(f'{record[e][k]:.3f}' for k in ("cpu_s", "wall_s", "peak_mib"))
                       for e in ("candidate", "shipping")]
            disk = []
            for engine in ("candidate", "shipping"):
                measured = record[engine]
                disk.extend((str(measured["output_bytes"]), f'{measured["store_bytes"]}/{measured["store_files"]}'))
            print(" | ".join([name, f'{counts["exact"]}/{record["requests"]}',
                  *[str(counts[k]) for k in ("different", "refused", "partial", "unstable")],
                  *metrics, *disk]), flush=True)
            ordered = sorted((r["candidate"] for r in record["repeats"]), key=lambda run: run["cpu_s"])
            measured = ordered[(len(ordered) - 1) // 2]
            parts = measured["parts_cpu_s"]
            if parts is None:
                print("Session component CPU unavailable from this worker", flush=True)
            else:
                print("Session part | CPU-s (one measured process)", flush=True)
                for key, label in {**SESSION_PARTS, "other": "Startup, scheduling and teardown"}.items():
                    if key == "read_parse" and all(k in parts for k in READER_PARTS):
                        for subkey, detail in READER_PARTS.items():
                            print(f'{detail} | {parts[subkey]:.3f}', flush=True)
                    else:
                        print(f'{label} | {parts[key]:.3f}', flush=True)
                print(f'Session total | {measured["cpu_s"]:.3f}', flush=True)
            if output:
                output.write_text(json.dumps(result, indent=2) + "\n")
    return result
