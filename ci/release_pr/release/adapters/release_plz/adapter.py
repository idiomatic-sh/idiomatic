"""The `release` step done through release-plz."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

from ci.release_pr.release.interface import Opened, Release


class ReleasePlzRelease(Release):
    """`release` through `release-plz release-pr`, over the product's workspace and its `release-plz.toml`, which turns off every
    publish, tag and GitHub Release: it opens or updates the pull request and nothing else. The token `gh` reads, GH_TOKEN, is
    handed to it as the git token it opens the pull request with."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def open(self, product: str) -> Opened | None:
        workspace = f"crates/{product}"
        printed = subprocess.run(
            ["release-plz", "release-pr", "--manifest-path", f"{workspace}/Cargo.toml", "--config", f"{workspace}/release-plz.toml", "--output", "json"],
            cwd=self.root,
            check=True,
            stdout=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            env={**os.environ, "GIT_TOKEN": os.environ.get("GH_TOKEN", "")},
        ).stdout
        prs = json.loads(printed)["prs"]
        if not prs:
            return None
        (pr,) = prs
        return Opened(
            number=pr["number"],
            url=pr["html_url"],
            branch=pr["head_branch"],
            versions=tuple(release["version"] for release in pr["releases"]),
        )
