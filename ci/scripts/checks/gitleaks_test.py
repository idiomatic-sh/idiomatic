"""The root gitleaks tools: the allowlist lets only a recorded digest line in a record folder through; `gitleaks` scans the staged
change in the pre-commit hook's lint group, and `gitleaks-history` the branch's own commits, in a full-history clone of the pushed
branch as CI checks it out, in a group the hook does not run."""

from __future__ import annotations

import hashlib
import shutil
import subprocess
from pathlib import Path

import pytest
import tomllib

ROOT = Path(__file__).resolve().parents[3]
# A 64-hex value with a secret's entropy, as a SHA-256 has.
DIGEST = hashlib.sha256(b"planted").hexdigest()
RECORD = "crates/ryo/languages/python/benchmarks/linters/ruff/results/run/summary.json"
# The leak review's five planted keys: a secret's own name with a dot or a slash, each holding 64 hex digits.
PLANTED = ["jwt.secret", "api.token", "aws/secret_access_key", "stripe/api_key", "wallet.key"]


def tool(identity: str) -> dict:
    (found,) = [each for each in tomllib.loads((ROOT / "asymptote.toml").read_text())["tools"] if each["id"] == identity]
    return found


def git(path: Path, *arguments: str) -> None:
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *arguments], cwd=path, check=True, capture_output=True)


def pushed_branch(tmp_path: Path, files: dict[str, str]) -> Path:
    """A clone, with its whole history, of a branch pushed to an origin whose main holds the scanner's settings, the branch
    adding `files` in a commit of its own: the checkout CI makes of a dispatched ref."""
    tmp_path.mkdir(parents=True, exist_ok=True)
    origin, work, clone = tmp_path / "origin", tmp_path / "work", tmp_path / "clone"
    git(tmp_path, "init", "-q", "--bare", "-b", "main", str(origin))
    git(tmp_path, "clone", "-q", str(origin), str(work))
    for name in (".gitleaks.toml", ".gitleaksignore"):
        shutil.copy(ROOT / name, work / name)
    git(work, "add", "-A")
    git(work, "commit", "-q", "-m", "chore: settings")
    git(work, "push", "-q", "origin", "HEAD:main")
    git(work, "switch", "-q", "-c", "claude/plant")
    for name, text in files.items():
        (work / name).parent.mkdir(parents=True, exist_ok=True)
        (work / name).write_text(text)
    git(work, "add", "-A")
    git(work, "commit", "-q", "-m", "feat: plant")
    git(work, "push", "-q", "origin", "claude/plant")
    git(tmp_path, "clone", "-q", "--branch", "claude/plant", str(origin), str(clone))
    return clone


def exit_of(path: Path, identity: str = "gitleaks-history") -> int:
    return subprocess.run(tool(identity)["run"], cwd=path, capture_output=True, check=False).returncode


@pytest.mark.parametrize("key", PLANTED)
def test_a_planted_secret_named_with_a_dot_or_a_slash_is_found_even_in_a_record_folder(tmp_path, key) -> None:
    assert exit_of(pushed_branch(tmp_path, {RECORD: f'{{\n  "{key}": "{DIGEST}"\n}}\n'})) == 1


def test_a_recorded_digest_is_let_through_only_in_a_record_folder(tmp_path) -> None:
    line = f'{{\n  "crates/core/src/key.rs": "{DIGEST}",\n  "python/ruff/sequence-index-accesses": "{DIGEST}"\n}}\n'
    assert exit_of(pushed_branch(tmp_path / "record", {RECORD: line})) == 0
    assert exit_of(pushed_branch(tmp_path / "elsewhere", {"config/summary.json": line})) == 1


def test_a_secret_committed_on_the_branch_and_deleted_is_found_after_the_push(tmp_path) -> None:
    clone = pushed_branch(tmp_path, {"config/settings.json": f'{{\n  "jwt_secret": "{DIGEST}"\n}}\n'})
    (clone / "config/settings.json").unlink()
    git(clone, "commit", "-q", "-am", "fix: drop it")
    assert exit_of(clone) == 1


def test_the_pre_commit_hook_scans_the_staged_change_alone_and_the_history_scan_runs_outside_it() -> None:
    hooked = tomllib.loads((ROOT / "asymptote.toml").read_text())["hooks"]["pre-commit"]
    assert tool("gitleaks")["group"] in hooked and "--staged" in tool("gitleaks")["run"]
    assert tool("gitleaks-history")["group"] not in hooked


def test_a_staged_secret_fails_the_pre_commit_scan(tmp_path) -> None:
    clone = pushed_branch(tmp_path, {"notes.txt": "fine\n"})
    (clone / "config.json").write_text(f'{{\n  "jwt.secret": "{DIGEST}"\n}}\n')
    git(clone, "add", "config.json")
    assert exit_of(clone, "gitleaks") == 1
