"""Where the Ruff rule inventory reads its sources and when a passing row is still current."""

from __future__ import annotations

import hashlib
from pathlib import Path
import tomllib

RUFF = Path(__file__).resolve().parents[1]
RESULTS = RUFF / "results"
REPOSITORY = RUFF.parents[6]
LATEST_FULL_RESULT = RESULTS / "integrated-3ff-full413-2026-11-04/summary.json"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source(rule: str) -> tuple[Path | None, str]:
    path = RUFF / "rules" / rule / "query.ryoql"
    return (path, "query") if path.is_file() else (None, "missing")


def current_source_matches(instance: dict[str, object]) -> bool:
    """A passing row remains current only while its registered settings and helpers match."""
    relative = instance.get("settings_source")
    if not isinstance(relative, str):
        return False
    profile = REPOSITORY / relative
    if not profile.is_file() or instance.get("settings_source_sha256") != sha(profile):
        return False
    settings = tomllib.loads(profile.read_text(encoding="utf-8"))
    base = tomllib.loads((RUFF / "benchmark.toml").read_text(encoding="utf-8"))
    registry = {**base.get("query-modules", {}), **settings.get("query-modules", {})}
    modules = instance.get("module_sha256")
    if not isinstance(modules, dict):
        return False
    return all(logical in registry and (RUFF / registry[logical]).is_file()
               and sha(RUFF / registry[logical]) == digest for logical, digest in modules.items())


def failure_category(reason: object) -> str:
    text = str(reason or "not run").lower()
    categories = [
        ("Stale source proof", ("changed or missing current source",)),
        ("Missing implementation", ("missing implementation", "no query source")),
        ("Registration failed", ("registration",)),
        ("Timed out", ("timeout", "timed out")),
        ("Query rejected", ("query source", "query parse", "typecheck")),
        ("Fixture mismatch", ("expected-vs-", "cases checked; failures", "mismatch")),
        ("Not run", ("not run", "not_run", "interrupted")),
    ]
    return next((category for category, patterns in categories if any(pattern in text for pattern in patterns)), "Verifier failed")
