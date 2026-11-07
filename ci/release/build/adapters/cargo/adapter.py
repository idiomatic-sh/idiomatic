"""The `build` step done through Cargo: the crate is its own package."""

from __future__ import annotations

from pathlib import Path

from ci.release.build.interface import Build
from ci.release.use_case import SHARED, Artifact, Product


class CargoBuild(Build):
    """`build` through Cargo: crates.io takes the crate's source, the same for every target,
    so the package is the command line crate's directory; Cargo builds it at install."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def build(self, product: Product, scope: str, into: Path) -> list[Artifact]:
        if scope != SHARED:
            return []
        return [Artifact("cargo", self.root / "crates" / product.name / "cli", SHARED)]
