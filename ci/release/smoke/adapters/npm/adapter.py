"""The `smoke` step done through npm: the built packages installed offline into a scratch
prefix and the command run through the launcher."""

from __future__ import annotations

import subprocess
from pathlib import Path

from ci.release.smoke.interface import Smoke, version_matches
from ci.release.use_case import Artifact, Product


class NpmSmoke(Smoke):
    """`smoke` through npm: when a platform package was built, it and the launcher installed
    offline, and `<command> --version` holding the crate's version."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def smoke(self, product: Product, artifacts: list[Artifact], into: Path) -> None:
        if len(artifacts) < 2:
            return
        prefix = into / "prefix"
        tarballs = [str(artifact.path) for artifact in artifacts]
        subprocess.run(
            ["npm", "install", "--global", "--offline", "--no-audit", "--no-fund", "--prefix", str(prefix), *tarballs],
            cwd=into,
            check=True,
        )
        said = subprocess.run(
            [str(prefix / "bin" / product.command), "--version"], check=True, capture_output=True, text=True, encoding="utf-8"
        ).stdout
        if not version_matches(product, said):
            raise AssertionError(f"npm's {product.command} --version printed {said!r}")
