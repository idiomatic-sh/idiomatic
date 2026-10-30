"""What the git adapter of `tree` promises beside the port's contract, proven
without running the tool it wraps."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.perf.tree.adapters.git.adapter import GitTree
from ci.perf.tree.interface_test import conforms


def test_the_git_adapter_implements_the_port() -> None:
    """It is a `tree` adapter."""
    conforms(GitTree)


def test_the_change_is_what_the_checkout_did_since_it_left_the_base(monkeypatch) -> None:
    calls = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs["cwd"]))
        return subprocess.CompletedProcess(argv, 0, "crates/ryo/cli/src/main.rs\nREADME.md\n\n")

    monkeypatch.setattr(subprocess, "run", run)
    assert GitTree(Path("/repo")).changed("origin/main") == ("crates/ryo/cli/src/main.rs", "README.md")
    assert calls == [(["git", "diff", "--name-only", "origin/main...HEAD"], Path("/repo"))], "three dots: since the merge base"


def test_the_base_is_a_detached_worktree_removed_by_its_path(monkeypatch) -> None:
    calls = []
    monkeypatch.setattr(subprocess, "run", lambda argv, **_: calls.append(argv))
    tree = GitTree(Path("/repo"))
    assert tree.checkout("origin/main", Path("/scratch/base")) == Path("/scratch/base")
    tree.remove(Path("/scratch/base"))
    assert calls == [
        ["git", "worktree", "add", "--detach", "/scratch/base", "origin/main"],
        ["git", "worktree", "remove", "--force", "/scratch/base"],
    ]
