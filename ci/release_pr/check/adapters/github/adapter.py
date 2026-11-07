"""The `check` step done through GitHub, with `gh`."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.release_pr.check.interface import Check


class GithubCheck(Check):
    """`check` through GitHub Actions with `gh`: the check workflow dispatched on the branch with the product as its input. Its
    `gate` job's check run is the branch head commit's, the one branch protection requires of the pull request."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def start(self, product: str, branch: str) -> None:
        subprocess.run(["gh", "workflow", "run", "check.yml", "--ref", branch, "-f", f"product={product}"], cwd=self.root, check=True)
