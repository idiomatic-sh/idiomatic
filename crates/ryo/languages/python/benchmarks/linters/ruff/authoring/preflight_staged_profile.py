#!/usr/bin/env python3
"""Check profile identity; optionally run an existing worker over all staged cases."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tomllib

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verify_profiles import (  # noqa: E402
    RUFF_DIR,
    assert_staged_profile_fixed_point,
    profile_modules,
    sha256,
)


def assert_active_module_closure(profile_root: Path) -> None:
    """Require an admission stage to contain the complete active module bytes."""
    profile = tomllib.loads((profile_root / "benchmark.toml").read_text(encoding="utf-8"))
    registry = profile_modules(profile, profile_root)
    for logical, relative in registry.items():
        staged = profile_root / relative
        active = RUFF_DIR / relative
        if not staged.is_file() or not active.is_file() or sha256(staged) != sha256(active):
            raise ValueError(f"active module identity mismatch: {logical} at {relative}")
    sources = [profile_root / "rules" / rule / "query.ryoql" for rule in profile["defaults"]]
    sources.extend(profile_root / relative for relative in registry.values())
    for source in sources:
        if not source.is_file():
            raise ValueError(f"missing staged query or module: {source}")
        imports = set(re.findall(
            r'^\s*import\s+\w+\s+from\s+"([^"]+)"\s*;',
            source.read_text(encoding="utf-8"), re.MULTILINE,
        ))
        for logical in sorted(imports - registry.keys()):
            raise ValueError(f"undeclared active module {logical} imported by {source}")


def verify_with_worker(profile_root: Path, worker: Path, ruff: Path) -> None:
    """Use the supported fixture CLI, which actually parses and typechecks queries.

    The existing worker has no parse-only command. This explicitly requested
    validation therefore executes the complete staged fixtures on both tools.
    """
    command = [str(worker.resolve()), "verify-cases", str(ruff.resolve()), str(profile_root.resolve())]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode:
        detail = result.stderr.strip() or result.stdout.strip()
        raise ValueError(f"sealed worker verify-cases exited {result.returncode}: {detail}")
    if result.stdout:
        print(result.stdout.rstrip())
    print(f"worker sha256={sha256(worker)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile_root", type=Path)
    parser.add_argument(
        "--active-modules", action="store_true",
        help="also require every imported module to match the current canonical source",
    )
    parser.add_argument(
        "--worker", type=Path,
        help="existing sealed worker; runs its supported verify-cases command (all fixtures, not parse-only)",
    )
    parser.add_argument("--ruff", type=Path, help="pinned Ruff executable required with --worker")
    args = parser.parse_args()
    if (args.worker is None) != (args.ruff is None):
        parser.error("--worker and --ruff must be supplied together")
    try:
        assert_staged_profile_fixed_point(args.profile_root)
        if args.active_modules:
            assert_active_module_closure(args.profile_root)
        print(f"checked: profile serialization fixed point; sha256={sha256(args.profile_root / 'benchmark.toml')}")
        if args.active_modules:
            print("checked: canonical module identities and declared import closure")
        else:
            print("not checked: canonical module identities and declared import closure")
        if args.worker is not None:
            verify_with_worker(args.profile_root, args.worker, args.ruff)
            print("checked: query parsing/typechecking and fixture verification via sealed worker verify-cases")
        else:
            print("not checked: query parsing/typechecking, candidate execution, or fixture parity (no --worker)")
        print("not checked: admission, full-rule parity, or performance")
    except (OSError, ValueError, TypeError) as error:
        print(f"authoring preflight refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
