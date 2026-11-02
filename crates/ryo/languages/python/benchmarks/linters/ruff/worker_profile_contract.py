"""Validate a benchmark worker command against its profiling manifest.

External profilers must declare the worker protocol explicitly. Environment flags
cannot turn the JSONL diagnostic worker into an ordinary timing worker.
"""

from collections.abc import Mapping, Sequence


_TRACING = {
    "run-ryo": "disabled",
    "profile-ryo": "chrome",
    "diagnostic-ryo": "jsonl",
}


def validate_worker_profile(manifest: Mapping[str, object], argv: Sequence[str]) -> None:
    """Reject a worker protocol that differs from the declared profile mode."""
    if len(argv) < 2:
        raise ValueError("worker command is missing")
    actual = _TRACING.get(argv[1])
    declared = manifest.get("worker_tracing")
    if actual is None or declared not in _TRACING.values():
        raise ValueError("worker tracing mode is unknown")
    if actual != declared:
        raise ValueError(f"worker tracing is {actual}, expected {declared}")
    if manifest.get("measurement") is True and actual != "disabled":
        raise ValueError("a traced worker cannot provide a benchmark measurement")
