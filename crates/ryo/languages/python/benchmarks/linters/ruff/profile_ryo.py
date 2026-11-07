"""Sample an explicit Ruff worker protocol with a worker/request-bound perf receipt.

Example: python3 profile_ryo.py --worker ./ruff-worker --request-file request.json
    --output-dir /tmp/ryo-cpu-profile --worker-tracing disabled
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time

from worker_profile_contract import validate_worker_profile


COMMAND = {"disabled": "run-ryo", "chrome": "profile-ryo", "jsonl": "diagnostic-ryo"}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def profile_manifest(
    worker: Path, request: Path, tracing: str, frequency: int,
    expected_report: Path | None, worker_command: list[str],
) -> dict[str, object]:
    """External perf sampling is intrusive even with ordinary worker tracing off."""
    return {
        "schema": "ryo-worker-cpu-profile/v1",
        "measurement": False,
        "timing_acceptance": False,
        "sampling_instrumentation": "external-perf-intrusive",
        "worker_tracing": tracing,
        "worker_sha256": sha256(worker),
        "request_sha256": sha256(request),
        "input_binding": "worker-and-request-bytes; request-referenced corpus is not hashed",
        "expected_report_sha256": sha256(expected_report) if expected_report else None,
        "sampling_hz": frequency,
        "worker_command": worker_command,
    }


def run_profile_command(command: list[str], env: dict[str, str], stdout, stderr,
                        timeout_seconds: float) -> tuple[int | None, str, str | None]:
    """Reap the whole perf/worker process group, including on timeout."""
    process = None
    try:
        process = subprocess.Popen(command, env=env, stdout=stdout, stderr=stderr,
                                   start_new_session=True)
        try:
            return process.wait(timeout=timeout_seconds), "completed", None
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
            return process.returncode, "timeout", f"profile exceeded {timeout_seconds} seconds"
    except Exception as error:
        if process is not None and process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
        return None, "launch-error", str(error)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", required=True, type=Path)
    parser.add_argument("--request-file", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--worker-tracing", required=True, choices=COMMAND)
    parser.add_argument("--parser-configuration", default="python-target=3.10")
    parser.add_argument("--expected-report", type=Path)
    parser.add_argument("--frequency", type=int, default=49)
    parser.add_argument("--lock-file", type=Path)
    parser.add_argument("--timeout-seconds", type=float, default=900)
    args = parser.parse_args()
    if args.frequency < 1 or args.frequency > 1000:
        parser.error("sampling frequency must be between 1 and 1000 Hz")
    if args.timeout_seconds <= 0:
        parser.error("timeout must be positive")
    worker = args.worker.resolve(strict=True)
    request = args.request_file.resolve(strict=True)
    out = args.output_dir.resolve()
    worker_command = [str(worker), COMMAND[args.worker_tracing], "--request-file", str(request), args.parser_configuration]
    if args.worker_tracing != "disabled":
        worker_command.append(str(out / "worker.trace"))
    manifest = profile_manifest(
        worker, request, args.worker_tracing, args.frequency,
        args.expected_report, worker_command,
    )
    validate_worker_profile(manifest, worker_command)
    out.mkdir(parents=True, exist_ok=False)
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    command = ["perf", "record", "-q", "-F", str(args.frequency), "-g", "--call-graph", "dwarf,8192",
               "-o", str(out / "profile.perf.data"), "--", *worker_command]
    env = {**os.environ, "RYO_DIAGNOSTIC_SUBPLANS": "0", "RYO_DIAGNOSTIC_STEPS": "0"}
    queue_started = time.monotonic()
    lock = None
    status = "lock-error"
    exit_code = None
    error = None
    queue_seconds = 0.0
    started = None
    try:
        if args.lock_file:
            import fcntl

            lock = args.lock_file.open("a+")
            fcntl.flock(lock, fcntl.LOCK_EX)
        queue_seconds = time.monotonic() - queue_started
        started = time.monotonic()
        with (out / "stdout.bin").open("wb") as stdout, (out / "stderr.bin").open("wb") as stderr:
            exit_code, status, error = run_profile_command(
                command, env, stdout, stderr, args.timeout_seconds,
            )
    except Exception as run_error:
        error = str(run_error)
    finally:
        if lock is not None:
            lock.close()
    (out / "stdout.bin").touch(exist_ok=True)
    (out / "stderr.bin").touch(exist_ok=True)
    receipt = {"command": command, "status": status, "exit_code": exit_code,
               "error": error, "lock_queue_seconds": queue_seconds,
               "intrusive_wall_seconds": time.monotonic() - started if started is not None else 0.0,
               "stdout_sha256": sha256(out / "stdout.bin"),
               "stderr_sha256": sha256(out / "stderr.bin")}
    if status == "completed" and exit_code == 0 and args.expected_report:
        try:
            observed = json.loads((out / "stdout.bin").read_text())
            expected = json.loads(args.expected_report.read_text())
            keys = ("diagnostics", "skipped", "coverage", "files")
            receipt["exact_expected_report"] = all(observed[key] == expected[key] for key in keys)
        except (OSError, ValueError, KeyError) as comparison_error:
            receipt["exact_expected_report"] = False
            receipt["comparison_error"] = str(comparison_error)
    (out / "result.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt), flush=True)
    if status != "completed" or exit_code != 0 or receipt.get("exact_expected_report") is False:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
