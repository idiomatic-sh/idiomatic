"""Measure one cold child, including startup, on a single available CPU (Linux)."""
import hashlib
import json
import os
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
    results = packets.directory / "receipts"
    command = [worker, "--tm-batch", packets.manifest, results]
    with (packets.directory / "reader.log").open("w+b") as log:
        if timed:
            metrics = process(command, stdout=log, stderr=log)
        else:
            subprocess.run(command, stdout=log, stderr=log, check=True)
            metrics = None
    return metrics, results


def compare(expected, results):
    outcomes = {}
    with results.open() as stream:
        for (identity, reference), raw in zip(expected.items(), stream, strict=True):
            fields = raw.rstrip("\n").split("\t")
            if len(fields) not in (3, 4):
                raise ValueError("invalid native receipt")
            key, native, answer = fields[:3]
            if key != identity:
                raise ValueError(f"receipt order or identity changed at {identity}")
            # The original batch protocol can only return rows or a refusal.
            # A worker with unknowns can append the typed path/message pairs.
            typed = json.loads(fields[3]) if len(fields) == 4 else []
            if not isinstance(typed, list) or any(
                not isinstance(pair, list) or len(pair) != 2 or
                not all(isinstance(value, str) for value in pair) for pair in typed
            ):
                raise ValueError("invalid native unknowns")
            if native == "refused":
                if answer or typed:
                    raise ValueError("refusal unexpectedly included an answer")
            elif native != "different" or len(answer) != 64 or any(c not in "0123456789abcdef" for c in answer):
                raise ValueError("expected a canonical row digest or explicit refusal")
            unknowns = tuple(sorted(tuple(pair) for pair in typed))
            wanted = tuple(sorted(tuple(pair) for pair in reference["unknowns"]))
            if native != "refused" and answer == reference["sha256"] and unknowns == wanted:
                status = "exact"
            elif wanted or unknowns:
                status = "partial"
            elif native == "refused":
                status = "refused"
            else:
                status = "different"
            outcomes[identity] = (status, (native, answer, unknowns))
    return outcomes
