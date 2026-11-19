"""The `diff` step done through git."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.changes.diff.interface import Diff


class GitDiff(Diff):
    """`diff` through git: a pull request's checkout is the merge GitHub makes of it into its branch, so the paths it changes are
    the ones that merge changes against its first parent, the branch's newest commit. Renames count twice, as the path removed
    and the path added, so a file moved out of a product checks that product too."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def paths(self) -> tuple[str, ...]:
        listed = subprocess.run(
            ["git", "diff", "--no-renames", "--name-only", "-z", "HEAD^1", "HEAD"],
            cwd=self.root,
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        ).stdout
        return tuple(path for path in listed.split("\0") if path)
