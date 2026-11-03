"""The `tags` step done through GitHub, with `gh`."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

from ci.tag.tags.interface import Tags


class GithubTags(Tags):
    """`tags` through GitHub's API with `gh`, for the repository the checkout's remote names: a tag read from its refs, and created
    as a ref at the commit `git rev-parse` resolves."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def exists(self, tag: str) -> bool:
        listed = subprocess.run(
            ["gh", "api", f"repos/{{owner}}/{{repo}}/git/matching-refs/tags/{tag}"],
            cwd=self.root,
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
        ).stdout
        return any(ref["ref"] == f"refs/tags/{tag}" for ref in json.loads(listed))

    def create(self, tag: str, revision: str) -> None:
        commit = subprocess.run(
            ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"], cwd=self.root, check=True, capture_output=True, text=True, encoding="utf-8"
        ).stdout.strip()
        subprocess.run(
            ["gh", "api", "--method", "POST", "repos/{owner}/{repo}/git/refs", "-f", f"ref=refs/tags/{tag}", "-f", f"sha={commit}"],
            cwd=self.root,
            check=True,
            capture_output=True,
        )
