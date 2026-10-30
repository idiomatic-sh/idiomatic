"""The root `merge-conflict` tool finds a conflict marker in a file git does not track yet, as in one it does."""

from __future__ import annotations

import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MARKED = "left\n<<<<<<< ours\nmine\n=======\ntheirs\n>>>>>>> theirs\n"


def run_tool() -> list[str]:
    tools = tomllib.loads((ROOT / "asymptote.toml").read_text())["tools"]
    (argv,) = [tool["run"] for tool in tools if tool["id"] == "merge-conflict"]
    return argv


def repository(path: Path) -> None:
    for arguments in (["init", "-q"], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]):
        subprocess.run(["git", *arguments], cwd=path, check=True, capture_output=True)


def exit_of(path: Path) -> int:
    return subprocess.run(run_tool(), cwd=path, capture_output=True, check=False).returncode


def test_a_clean_tree_passes(tmp_path) -> None:
    repository(tmp_path)
    (tmp_path / "a.txt").write_text("fine\n")
    assert exit_of(tmp_path) == 0


def test_an_untracked_file_with_markers_fails(tmp_path) -> None:
    repository(tmp_path)
    (tmp_path / "new.txt").write_text(MARKED)
    assert exit_of(tmp_path) != 0


def test_the_same_file_staged_and_tracked_fails(tmp_path) -> None:
    repository(tmp_path)
    (tmp_path / "new.txt").write_text(MARKED)
    subprocess.run(["git", "add", "new.txt"], cwd=tmp_path, check=True)
    assert exit_of(tmp_path) != 0


def test_a_file_git_ignores_is_not_read(tmp_path) -> None:
    repository(tmp_path)
    (tmp_path / ".gitignore").write_text("build.log\n")
    (tmp_path / "build.log").write_text(MARKED)
    assert exit_of(tmp_path) == 0


def test_a_failing_git_is_a_failure_not_a_pass(tmp_path) -> None:
    repository(tmp_path)
    (tmp_path / ".git" / "HEAD").write_text("garbage\n")
    assert exit_of(tmp_path) != 0
