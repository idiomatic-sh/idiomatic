"""The `builds` step done through file."""

from __future__ import annotations

import os
import tomllib
from pathlib import Path

from ci.changes.builds.interface import Builds
from ci.tag.use_case import PRODUCTS


class FileBuilds(Builds):
    """`builds` through file: the `[patch.crates-io]` table of the product's Cargo.toml as the checkout holds it, each patch's
    `path` read from the product's directory."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def built_on(self, product: str) -> frozenset[str]:
        directory = self.root / "crates" / product
        manifest = tomllib.loads((directory / "Cargo.toml").read_text(encoding="utf-8"))
        found = set()
        for patch in manifest.get("patch", {}).get("crates-io", {}).values():
            if "path" not in patch:
                continue
            parts = Path(os.path.relpath(os.path.normpath(directory / patch["path"]), self.root)).parts
            if len(parts) > 1 and parts[0] == "crates" and parts[1] in PRODUCTS and parts[1] != product:
                found.add(parts[1])
        return frozenset(found)
