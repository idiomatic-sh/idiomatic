"""Measure one cold child, including startup, on a single available CPU (Linux)."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def retained_files(directory, exclude=None):
    """Logical file bytes and count in a newly created store or workspace."""
    sizes = [path.lstat().st_size for path in directory.rglob("*")
             if path != exclude and (path.is_symlink() or path.is_file())]
    return sum(sizes), len(sizes)


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


# Keys match the native worker's CHECK_SESSION record. All clocks use seconds.
SESSION_PARTS = {
    "prep": "Query preparation",
    "admission": "File admission",
    "read_parse": "Read, parse, tree projection and reader facts",
    "first": "First half",
    "second": "Second half",
    "project": "Project phase",
    "assembly": "Answer assembly and receipt output",
}


READER_PARTS = {
    "read_versions": "Read and identify file versions",
    "reader_build": "Parser and provider build",
    "reader_projection_facts": "Tree projection and reader facts",
}


def session_times(lines, cpu):
    for raw in lines:
        if raw.startswith(b"CHECK_SESSION "):
            fields = dict(field.split("=", 1) for field in raw.decode().split()[1:] if "=" in field)
            if not SESSION_PARTS.keys() <= fields.keys():
                return None
            parts = {key: float(fields[key]) for key in SESSION_PARTS}
            parts["other"] = cpu - sum(parts.values())
            if all(key in fields for key in READER_PARTS):
                del parts["read_parse"]
                parts.update((key, float(fields[key])) for key in READER_PARTS)
            return parts
    return None


def candidate(worker, packets, *, timed=True):
    results = packets.directory / "receipts"
    with tempfile.TemporaryDirectory(dir=packets.directory) as directory, (packets.directory / "reader.log").open("w+b") as log:
        fresh = Path(directory).resolve()
        output = fresh / "receipts"
        command = [Path(worker).resolve(), "--tm-batch", packets.manifest.resolve(), output]
        if timed:
            metrics = process(command, cwd=fresh, stdout=log, stderr=log)
            log.seek(0)
            metrics["parts_cpu_s"] = session_times(log, metrics["cpu_s"])
            metrics["output_bytes"] = output.stat().st_size
            metrics["store_bytes"], metrics["store_files"] = retained_files(fresh, exclude=output)
        else:
            subprocess.run(command, cwd=fresh, stdout=log, stderr=log, check=True)
            metrics = None
        output.replace(results)
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
