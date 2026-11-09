"""The root `commitizen` tool checks the commits since its pinned baseline, in a full-history clone of the pushed branch as CI checks
it out; the history before the baseline is not checked again. Each case stands its own scratch baseline in for the pinned one."""

from __future__ import annotations

import re
import shutil
import subprocess
from pathlib import Path

import tomllib

ROOT = Path(__file__).resolve().parents[3]


BASELINE = re.compile(r"base=([0-9a-f]{40});")


def run_tool(baseline: str) -> list[str]:
    tools = tomllib.loads((ROOT / "asymptote.toml").read_text())["tools"]
    (argv,) = [tool["run"] for tool in tools if tool["id"] == "commitizen"]
    pinned = BASELINE.search(argv[-1])
    assert pinned, "the commitizen tool names its baseline"
    return [*argv[:-1], argv[-1].replace(pinned[1], baseline)]


def git(path: Path, *arguments: str) -> None:
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", *arguments], cwd=path, check=True, capture_output=True)


def pushed_branch(tmp_path: Path, messages: list[str]) -> tuple[Path, str]:
    """A whole-history clone of a branch pushed with `messages`, and its baseline: main's one commit, which is no conventional
    commit."""
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
    baseline = subprocess.run(["git", "rev-parse", "origin/main"], cwd=clone, check=True, capture_output=True, text=True).stdout.strip()
    return clone, baseline


def exit_of(cloned: tuple[Path, str]) -> int:
    path, baseline = cloned
    return subprocess.run(run_tool(baseline), cwd=path, capture_output=True, check=False).returncode


def test_a_pushed_branch_with_a_bad_message_fails_in_a_full_history_clone(tmp_path) -> None:
    assert exit_of(pushed_branch(tmp_path, ["fix: good", "not a conventional message"])) != 0


def test_a_pushed_branch_of_conventional_commits_passes_though_the_history_before_the_baseline_holds_an_old_message(tmp_path) -> None:
    assert exit_of(pushed_branch(tmp_path, ["fix: good", "feat: also good"])) == 0
