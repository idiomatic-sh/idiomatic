"""The `tree` step done through git."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.perf.tree.interface import Tree


class GitTree(Tree):
    """`tree` through git: the paths `git diff` names since the merge base, and the base as a detached worktree."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def changed(self, base: str) -> tuple[str, ...]:
        listed = subprocess.run(
            ["git", "diff", "--name-only", f"{base}...HEAD"], cwd=self.root, check=True, capture_output=True, text=True
        ).stdout
        return tuple(line for line in listed.splitlines() if line)

    def checkout(self, base: str, into: Path) -> Path:
        subprocess.run(["git", "worktree", "add", "--detach", str(into), base], cwd=self.root, check=True)
        return into

    def remove(self, tree: Path) -> None:
        subprocess.run(["git", "worktree", "remove", "--force", str(tree)], cwd=self.root, check=True)
