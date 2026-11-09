"""The root `commitizen` tool checks a branch's own commits since its merge-base with origin/main, in a full-history clone of the
pushed branch, as CI checks it out; main's history is not checked again."""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parents[3]


def run_tool() -> list[str]:
    tools = tomllib.loads((ROOT / "asymptote.toml").read_text())["tools"]
    (argv,) = [tool["run"] for tool in tools if tool["id"] == "commitizen"]
    return argv


def git(path: Path, *arguments: str) -> None:
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *arguments], cwd=path, check=True, capture_output=True)


def pushed_branch(tmp_path: Path, messages: list[str]) -> Path:
    """A whole-history clone of a branch pushed with `messages`, from an origin whose main holds one commit that is no
    conventional commit."""
    origin, work, clone = tmp_path / "origin", tmp_path / "work", tmp_path / "clone"
    git(tmp_path, "init", "-q", "--bare", "-b", "main", str(origin))
    git(tmp_path, "clone", "-q", str(origin), str(work))
    shutil.copy(ROOT / "cz.toml", work / "cz.toml")
    git(work, "add", "-A")
    git(work, "commit", "-q", "-m", "an old message main already holds")
    git(work, "push", "-q", "origin", "HEAD:main")
    git(work, "switch", "-q", "-c", "claude/plant")
    for message in messages:
        git(work, "commit", "-q", "--allow-empty", "-m", message)
    git(work, "push", "-q", "origin", "claude/plant")
    git(tmp_path, "clone", "-q", "--branch", "claude/plant", str(origin), str(clone))
    return clone


def exit_of(path: Path) -> int:
    return subprocess.run(run_tool(), cwd=path, capture_output=True, check=False).returncode


def test_a_pushed_branch_with_a_bad_message_fails_in_a_full_history_clone(tmp_path) -> None:
    assert exit_of(pushed_branch(tmp_path, ["fix: good", "not a conventional message"])) != 0


def test_a_pushed_branch_of_conventional_commits_passes_though_main_holds_an_old_message(tmp_path) -> None:
    assert exit_of(pushed_branch(tmp_path, ["fix: good", "feat: also good"])) == 0
