"""What the git adapter of `diff` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.changes.diff.adapters.git.adapter import GitDiff
from ci.changes.diff.interface_test import conforms


def test_the_git_adapter_implements_the_port() -> None:
    """It is a `diff` adapter."""
    conforms(GitDiff)


def test_the_paths_are_the_merge_s_against_its_first_parent_a_rename_as_both_paths(monkeypatch) -> None:
    calls = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs["cwd"]))
        return subprocess.CompletedProcess(argv, 0, "crates/ryo/README.md\0README.md\0docs/a b.md\0")

    monkeypatch.setattr(subprocess, "run", run)
    assert GitDiff(Path("/repo")).paths() == ("crates/ryo/README.md", "README.md", "docs/a b.md")
    assert calls == [(["git", "diff", "--no-renames", "--name-only", "-z", "HEAD^1", "HEAD"], Path("/repo"))]


def test_a_merge_that_changes_nothing_has_no_path(monkeypatch) -> None:
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: subprocess.CompletedProcess(argv, 0, ""))
    assert GitDiff(Path("/repo")).paths() == ()
