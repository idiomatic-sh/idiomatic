"""The `manifest` step done through git."""

from __future__ import annotations

import subprocess
import tomllib
from pathlib import Path

from ci.tag.manifest.interface import Manifest


class GitManifest(Manifest):
    """`manifest` through git: the product's Cargo.toml as `git show` reads it at the commit."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def version(self, product: str, revision: str) -> str | None:
        shown = subprocess.run(
            ["git", "show", f"{revision}:crates/{product}/Cargo.toml"], cwd=self.root, check=False, capture_output=True, text=True, encoding="utf-8"
        )
        if shown.returncode != 0:
            return None
        return tomllib.loads(shown.stdout).get("workspace", {}).get("package", {}).get("version")
