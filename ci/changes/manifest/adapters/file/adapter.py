"""The `manifest` step done through file."""

from __future__ import annotations

import os
import tomllib
from pathlib import Path

from ci.changes.manifest.interface import Manifest
from ci.tag.use_case import PRODUCTS


class FileManifest(Manifest):
    """`manifest` through file: the product's asymptote.toml as the checkout holds it, each tool input read from the product's
    directory up to its first capture or wildcard (`languages/{lang}` names `languages`)."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def named(self, product: str) -> frozenset[str]:
        directory = self.root / "crates" / product
        manifest = tomllib.loads((directory / "asymptote.toml").read_text(encoding="utf-8"))
        found = set()
        for tool in manifest.get("tools", []):
            for each in tool.get("inputs", ["."]):
                literal = each.split("{")[0].split("*")[0].split("?")[0] or "."
                parts = Path(os.path.relpath(os.path.normpath(directory / literal), self.root)).parts
                if len(parts) > 1 and parts[0] == "crates" and parts[1] in PRODUCTS and parts[1] != product:
                    found.add(parts[1])
        return frozenset(found)
