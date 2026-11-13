"""The root `commitizen` tool checks the commits a pushed branch holds beyond origin/main, in a clone of it as CI checks it out,
and, run for a pull request on GitHub Actions, the pull request's title, which a squash merge makes the commit main keeps. The
history main already holds is not checked again."""

from __future__ import annotations

import json
import os
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
    """A clone of a branch pushed with `messages` over main, whose one commit is no conventional commit."""
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
    git(tmp_path, "clone", "-q", "--branch", "claude/plant", f"file://{origin}", str(clone))
    return clone


def exit_of(clone: Path, event: dict | None = None) -> int:
    environment = {key: value for key, value in os.environ.items() if key != "GITHUB_EVENT_PATH"}
    if event is not None:
        path = clone.parent / "event.json"
        path.write_text(json.dumps(event))
        environment["GITHUB_EVENT_PATH"] = str(path)
    return subprocess.run(run_tool(), cwd=clone, env=environment, capture_output=True, check=False).returncode


def test_a_pushed_branch_with_a_bad_message_fails(tmp_path) -> None:
    assert exit_of(pushed_branch(tmp_path, ["fix: good", "not a conventional message"])) != 0


def test_a_pushed_branch_of_conventional_commits_passes_though_main_holds_an_old_message(tmp_path) -> None:
    assert exit_of(pushed_branch(tmp_path, ["fix: good", "feat: also good"])) == 0


def test_a_branch_with_no_commit_ahead_of_main_has_none_to_check(tmp_path) -> None:
    assert exit_of(pushed_branch(tmp_path, [])) == 0


def test_a_pull_request_whose_title_is_no_conventional_commit_fails(tmp_path) -> None:
    event = {"pull_request": {"title": "not a conventional title"}}
    assert exit_of(pushed_branch(tmp_path, ["fix: good"]), event) != 0


def test_a_pull_request_with_a_conventional_title_passes(tmp_path) -> None:
    event = {"pull_request": {"title": "fix: a conventional title"}}
    assert exit_of(pushed_branch(tmp_path, ["fix: good"]), event) == 0


def test_an_event_that_is_no_pull_request_checks_no_title(tmp_path) -> None:
    assert exit_of(pushed_branch(tmp_path, ["fix: good"]), {"ref": "refs/heads/main"}) == 0
