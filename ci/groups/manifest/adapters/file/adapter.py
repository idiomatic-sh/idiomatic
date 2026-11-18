"""The `manifest` step done through file."""

from __future__ import annotations

import tomllib
from pathlib import Path

from ci.groups.manifest.interface import Manifest


class FileManifest(Manifest):
    """`manifest` through file: the product's asymptote.toml as the checkout holds it."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def groups(self, product: str) -> frozenset[str] | None:
        manifest = self.root / "crates" / product / "asymptote.toml"
        if not manifest.is_file():
            return None
        return frozenset(tool["group"] for tool in tomllib.loads(manifest.read_text(encoding="utf-8")).get("tools", []))
