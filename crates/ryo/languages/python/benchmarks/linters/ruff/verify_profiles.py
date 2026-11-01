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


def self_test() -> int:
    """Exercise fail-closed profile-selection and hash-mismatch guards without invoking Cargo."""
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="ryo-ruff-profile-guards-") as temporary:
        root = Path(temporary)
        for relative in manifest["files"]:
            source = REPOSITORY / relative
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)

        helper_relative = Path(__file__).relative_to(REPOSITORY).as_posix()
        manifest["files"][helper_relative] = sha256(Path(__file__))
        temporary_manifest = root / MANIFEST.relative_to(REPOSITORY)
        temporary_manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        helper = root / helper_relative
        ruff = str(Path(sys.executable).resolve())
        invalid_selection = subprocess.run(
            [sys.executable, str(helper), ruff, "INVALID-RULE"],
            capture_output=True,
            text=True,
            check=False,
        )
        if invalid_selection.returncode != 2 or "unknown or duplicate profile rule" not in invalid_selection.stderr:
            print("invalid profile selection did not fail closed", file=sys.stderr)
            return 1

        candidate = root / next(
            path
            for path in manifest["files"]
            if path.endswith("/rules/F401/candidate.ryoql")
        )
        candidate.write_bytes(candidate.read_bytes() + b"\n")
        changed_input = subprocess.run(
            [sys.executable, str(helper), ruff, "F401"],
            capture_output=True,
            text=True,
            check=False,
        )
        if changed_input.returncode != 2 or "prepared input hash mismatch" not in changed_input.stderr:
            print("prepared input hash mismatch did not fail closed", file=sys.stderr)
            return 1
    print("profile selection and hash-mismatch guards passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ruff", type=Path, nargs="?", help="pinned Ruff executable")
    parser.add_argument("rules", nargs="*", help="optional rule IDs; default: every prepared profile")
    parser.add_argument("--self-test", action="store_true", help="exercise fail-closed profile guards without running Cargo")
    arguments = parser.parse_args()
    if arguments.self_test:
        return self_test()
    if arguments.ruff is None:
        parser.error("the pinned Ruff executable is required unless --self-test is selected")

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
    statuses = []
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
            try:
                result = subprocess.run(
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
                    check=False,
                )
                status = result.returncode
            except OSError as error:
                print(f"{rule}: could not start verifier: {error}", file=sys.stderr)
                status = 1
            print(f"{rule}: verifier exit status {status}", file=sys.stderr)
            statuses.append((rule, status))
    failed = [rule for rule, status in statuses if status != 0]
    if failed:
        print(f"candidate profiles failed: {', '.join(failed)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
