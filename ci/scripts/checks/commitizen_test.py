"""The root `commitizen` tool checks the message main keeps, read from the event GitHub hands the run: main takes squash merges alone,
so a pull request's run checks its title, which the squash merge makes main's commit, and not the branch's own commits; a push to
main checks the commits it added. A run with no event, or an event that lands nothing on main, has no message to check."""

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


def git(path: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *arguments], cwd=path, check=True, capture_output=True, text=True
    ).stdout.strip()


def pushed(tmp_path: Path, messages: list[str]) -> tuple[Path, str, str]:
    """A clone, as CI checks it out, of a repository whose main holds the tool's files under a message that is no conventional
    commit, and whose `claude/plant` branch adds `messages` over it; with main's head and the branch's head."""
    tmp_path.mkdir(parents=True, exist_ok=True)
    origin, work, clone = tmp_path / "origin", tmp_path / "work", tmp_path / "clone"
    git(tmp_path, "init", "-q", "--bare", "-b", "main", str(origin))
    git(tmp_path, "clone", "-q", str(origin), str(work))
    shutil.copy(ROOT / "cz.toml", work / "cz.toml")
    (work / "ci/scripts/checks").mkdir(parents=True)
    shutil.copy(ROOT / "ci/scripts/checks/commitizen.py", work / "ci/scripts/checks/commitizen.py")
    git(work, "add", "-A")
    git(work, "commit", "-q", "-m", "an old message main already holds")
    git(work, "push", "-q", "origin", "HEAD:main")
    before = git(work, "rev-parse", "HEAD")
    git(work, "switch", "-q", "-c", "claude/plant")
    for message in messages:
        git(work, "commit", "-q", "--allow-empty", "-m", message)
    git(work, "push", "-q", "origin", "claude/plant")
    git(tmp_path, "clone", "-q", "--branch", "claude/plant", f"file://{origin}", str(clone))
    return clone, before, git(work, "rev-parse", "HEAD")


def exit_of(clone: Path, event: dict | None) -> int:
    environment = {key: value for key, value in os.environ.items() if key != "GITHUB_EVENT_PATH"}
    if event is not None:
        path = clone.parent / "event.json"
        path.write_text(json.dumps(event))
        environment["GITHUB_EVENT_PATH"] = str(path)
    return subprocess.run(run_tool(), cwd=clone, env=environment, capture_output=True, check=False).returncode


def test_a_pull_request_whose_title_is_no_conventional_commit_fails(tmp_path) -> None:
    clone, _, _ = pushed(tmp_path, ["fix: good"])
    assert exit_of(clone, {"pull_request": {"title": "not a conventional title"}}) != 0


def test_a_pull_request_with_a_conventional_title_passes_though_its_branch_holds_a_bad_commit(tmp_path) -> None:
    clone, _, _ = pushed(tmp_path, ["fix: good", "not a conventional message"])
    assert exit_of(clone, {"pull_request": {"title": "fix: a conventional title"}}) == 0


def test_a_push_to_main_with_a_bad_commit_fails(tmp_path) -> None:
    clone, before, after = pushed(tmp_path, ["fix: good", "not a conventional message"])
    assert exit_of(clone, {"ref": "refs/heads/main", "before": before, "after": after}) != 0


def test_a_push_to_main_of_conventional_commits_passes_though_main_holds_an_old_message(tmp_path) -> None:
    clone, before, after = pushed(tmp_path, ["fix: good", "feat: also good"])
    assert exit_of(clone, {"ref": "refs/heads/main", "before": before, "after": after}) == 0


def test_a_push_that_creates_main_checks_its_head_commit(tmp_path) -> None:
    created = "0" * 40
    clone, _, after = pushed(tmp_path / "bad", ["not a conventional message"])
    assert exit_of(clone, {"ref": "refs/heads/main", "before": created, "after": after}) != 0
    clone, _, after = pushed(tmp_path / "good", ["fix: good"])
    assert exit_of(clone, {"ref": "refs/heads/main", "before": created, "after": after}) == 0


def test_a_run_with_no_event_or_a_push_to_another_branch_checks_nothing(tmp_path) -> None:
    clone, before, after = pushed(tmp_path, ["not a conventional message"])
    assert exit_of(clone, None) == 0
    assert exit_of(clone, {"ref": "refs/heads/claude/plant", "before": before, "after": after}) == 0
