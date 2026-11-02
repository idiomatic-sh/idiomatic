#!/usr/bin/env python3
"""Check a certificate-bearing authoring stage before deriving profile hashes."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile_root", type=Path)
    parser.add_argument(
        "--active-modules", action="store_true",
        help="also require every imported module to match the current canonical source",
    )
    args = parser.parse_args()
    try:
        assert_staged_profile_fixed_point(args.profile_root)
        if args.active_modules:
            assert_active_module_closure(args.profile_root)
    except (OSError, ValueError, TypeError) as error:
        print(f"profile normalization preflight refused: {error}", file=sys.stderr)
        return 1
    print(f"fixed-point profile sha256={sha256(args.profile_root / 'benchmark.toml')}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
