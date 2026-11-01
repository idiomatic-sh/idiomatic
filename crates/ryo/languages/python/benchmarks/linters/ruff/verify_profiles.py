"""Materialize and verify one-rule full-report Ruff acceptance profiles."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path


RUFF_DIR = Path(__file__).resolve().parent
REPOSITORY = RUFF_DIR.parents[6]
PROFILE_DIR = RUFF_DIR / "profiles"
MANIFEST = PROFILE_DIR / "prepared.json"
RULES = ("F401", "F811", "F821", "F822", "F823", "F841")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ruff", type=Path, help="pinned Ruff executable")
    parser.add_argument("rules", nargs="*", help="optional rule IDs; default: every prepared profile")
    arguments = parser.parse_args()

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    profiles = manifest["profiles"]
    if set(profiles) != set(RULES):
        parser.error("prepared profile set does not match this acceptance cohort")
    selected = arguments.rules or sorted(profiles)
    if len(set(selected)) != len(selected) or set(selected) - profiles.keys():
        parser.error("unknown or duplicate profile rule")

    files = manifest["files"]
    helper_path = Path(__file__).relative_to(REPOSITORY).as_posix()
    expected_files = {helper_path}
    for rule in RULES:
        expected_files.update(
            {
                (RUFF_DIR / "profiles" / rule / "benchmark.toml").relative_to(REPOSITORY).as_posix(),
                (RUFF_DIR / "rules" / rule / "candidate.ryoql").relative_to(REPOSITORY).as_posix(),
                (RUFF_DIR / "rules" / rule / "cases.toml").relative_to(REPOSITORY).as_posix(),
            }
        )
    if set(files) != expected_files:
        parser.error("prepared input paths do not match the candidate acceptance cohort")
    if files.get(helper_path) != sha256(Path(__file__)):
        parser.error("profile helper hash does not match prepared.json")
    for relative, expected in files.items():
        path = REPOSITORY / relative
        if not path.is_file() or sha256(path) != expected:
            parser.error(f"prepared input hash mismatch: {relative}")

    ruff = arguments.ruff.resolve()
    for rule in selected:
        profile = PROFILE_DIR / rule / "benchmark.toml"
        profile_data = tomllib.loads(profile.read_text(encoding="utf-8"))
        if profile_data.get("defaults") != [rule]:
            parser.error(f"profile must select only {rule}")
        if (
            profile_data.get("tool") != "ruff"
            or profile_data.get("version") != manifest["reference_version"]
            or profile_data.get("sha256") != manifest["reference_sha256"]
        ):
            parser.error(f"tool identity mismatch for {rule}")
        candidate_path = RUFF_DIR / "rules" / rule / "candidate.ryoql"
        cases_path = RUFF_DIR / "rules" / rule / "cases.toml"
        cases = tomllib.loads(cases_path.read_text(encoding="utf-8"))["cases"]
        recorded = profiles[rule]
        names = [case["name"] for case in cases]
        if (
            recorded.get("selected_rules") != [rule]
            or names != recorded["case_names"]
            or len(cases) != recorded["case_count"]
        ):
            parser.error(f"case inventory mismatch for {rule}")

        with tempfile.TemporaryDirectory(prefix=f"ryo-ruff-{rule}-") as temporary:
            destination = Path(temporary)
            (destination / "rules" / rule).mkdir(parents=True)
            shutil.copyfile(profile, destination / "benchmark.toml")
            shutil.copyfile(
                candidate_path,
                destination / "rules" / rule / "query.ryoql",
            )
            shutil.copyfile(cases_path, destination / "rules" / rule / "cases.toml")
            subprocess.run(
                [
                    "cargo",
                    "bench",
                    "--locked",
                    "-p",
                    "idiomatic-sh-ryo-python",
                    "--bench",
                    "ruff",
                    "--",
                    "verify-cases",
                    str(ruff),
                    str(destination),
                ],
                cwd=REPOSITORY / "crates" / "ryo",
                check=True,
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
