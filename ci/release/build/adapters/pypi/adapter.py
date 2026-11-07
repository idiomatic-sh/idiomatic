"""The `build` step done through PyPI's build: the maturin binary wheel of a target, which the
workflow's build job made on that target's own machine and uploaded."""

from __future__ import annotations

from pathlib import Path

from ci.release.build.interface import Build
from ci.release.use_case import SHARED, Artifact, Product, Refused


class PypiBuild(Build):
    """`build` through PyPI's build: for a target, its wheel in `dist/<target>`; nothing that
    every target shares, and nothing for a product that has no PyPI registry."""

    def __init__(self, root: Path, dist: Path) -> None:
        self.root = root
        self.dist = dist

    def build(self, product: Product, scope: str, into: Path) -> list[Artifact]:
        pypi = self.root / "crates" / product.name / "distribution" / "registries" / "pypi"
        if scope == SHARED or not pypi.is_dir():
            return []
        pattern = f"{product.package.replace('-', '_')}-*.whl"
        wheels = list((self.dist / scope).glob(pattern))
        if not wheels:
            raise Refused(f"no wheel {self.dist / scope / pattern}: run the build job for {scope} first")
        (wheel,) = wheels
        return [Artifact("pypi", wheel, scope)]
