"""Measure one cold child, including startup, on a single available CPU (Linux)."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def file_sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def process(command, *, stdout, stderr, cwd=None, stdin=None, env=None, allowed=(0,)):
    with tempfile.NamedTemporaryFile() as usage:
        # GNU time starts with a small address space before it forks the worker.
        # Linux would otherwise include the Python coordinator's RSS at fork.
        core = str(min(os.sched_getaffinity(0)))
        wrapped = ["taskset", "-c", core, "/usr/bin/time", "-q", "-o", usage.name,
                   "-f", "%U %S %M", *command]
        start_load = os.getloadavg()[0]
        start = time.monotonic()
        child = subprocess.run(wrapped, cwd=cwd, stdin=stdin, stdout=stdout, stderr=stderr, env=env)
        end = time.monotonic()
        measurements = usage.read().split()
    if child.returncode not in allowed:
        stderr.flush()
        stderr.seek(max(0, stderr.tell() - 3000))
        raise RuntimeError(f"{command[0]} exited {child.returncode}: {stderr.read()!r}")
    user, system, peak_kib = map(float, measurements)
    return dict(cpu_s=user + system, user_s=user,
                system_s=system, wall_s=end - start, peak_mib=peak_kib / 1024,
                workers=1, load=[start_load, os.getloadavg()[0]])


def candidate(worker, packets, *, timed=True):
    project_output = packets.directory / "project-receipts"
    results = packets.directory / "receipts"
    with packets.project.open("rb") as source, project_output.open("wb") as out, (
        packets.directory / "reader.log"
    ).open("w+b") as log:
        command = [worker, packets.mixed, project_output, results]
        if timed:
            metrics = process(command, stdin=source, stdout=out, stderr=log)
        else:
            subprocess.run(command, stdin=source, stdout=out, stderr=log, check=True)
            metrics = None
    return metrics, results


def compare(expected, results):
    outcomes = {}
    with results.open() as stream:
        for (identity, reference), raw in zip(expected.items(), stream, strict=True):
            answer = json.loads(raw)
            if answer.pop("id") != identity:
                raise ValueError(f"receipt order or identity changed at {identity}")
            if reference["unknowns"]:
                status = "partial"
            elif "refused" in answer:
                status = "refused"
            elif answer == {"rows": reference["rows"], "sha256": reference["sha256"]}:
                status = "exact"
            else:
                status = "different"
            outcomes[identity] = (status, answer)
    return outcomes
