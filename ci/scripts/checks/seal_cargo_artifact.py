#!/usr/bin/env python3
"""Seal one release executable identified by Cargo's compiler-artifact JSON."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import shutil


def select_artifact(lines: list[str], name: str, kind: str, source_suffix: str,
                    profile_test: bool) -> Path:
    """Cargo's target.test is eligibility; profile.test identifies this build mode."""
    matches = []
    for line in lines:
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        target = record.get("target", {})
        source = target.get("src_path", "")
        if (record.get("reason") == "compiler-artifact"
            and target.get("name") == name
            and kind in target.get("kind", [])
            and record.get("profile", {}).get("test") is profile_test
            and isinstance(source, str)
            and source.replace("\\", "/").endswith(source_suffix)
            and isinstance(record.get("executable"), str)):
            matches.append(Path(record["executable"]))
    if len(matches) != 1:
        raise ValueError(f"expected one {name} release executable, found {len(matches)}")
    return matches[0]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("jsonl", type=Path)
    parser.add_argument("name")
    parser.add_argument("kind")
    parser.add_argument("source_suffix")
    parser.add_argument("profile_test", choices=["true", "false"])
    parser.add_argument("sealed", type=Path)
    args = parser.parse_args()
    source = select_artifact(args.jsonl.read_text().splitlines(), args.name, args.kind,
                             args.source_suffix, args.profile_test == "true").resolve(strict=True)
    if args.sealed.exists() or args.sealed.with_name(args.sealed.name + "-seal.json").exists():
        raise ValueError(f"sealed output already exists: {args.sealed}")
    shutil.copyfile(source, args.sealed)
    args.sealed.chmod(0o755)
    digest = hashlib.sha256(args.sealed.read_bytes()).hexdigest()
    seal = {"source_artifact": str(source), "sha256": digest}
    args.sealed.with_name(args.sealed.name + "-seal.json").write_text(
        json.dumps(seal, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
