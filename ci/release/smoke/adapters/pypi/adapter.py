"""The `smoke` step done through PyPI's tools: the built wheel installed into a fresh virtual
environment and the command run both ways."""

from __future__ import annotations

import subprocess
import tomllib
from pathlib import Path

from ci.release.smoke.interface import Smoke, version_matches
from ci.release.use_case import Artifact, Product


class PypiSmoke(Smoke):
    """`smoke` through PyPI's tools: each built wheel installed without the index, and
    `<command> --version` and `python -m <module> --version` holding the crate's version."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def smoke(self, product: Product, artifacts: list[Artifact], into: Path) -> None:
        crate = self.root / "crates" / product.name
        pyproject = crate / "distribution" / "registries" / "pypi" / "pyproject.toml"
        module = tomllib.loads(pyproject.read_text())["tool"]["maturin"]["module-name"]
        for artifact in artifacts:
            environment = into / "venv"
            subprocess.run(["uv", "venv", "--quiet", str(environment)], check=True)
            python = environment / "bin" / "python"
            subprocess.run(["uv", "pip", "install", "--python", str(python), "--no-index", "--no-deps", str(artifact.path)], check=True)
            for argv in ([str(environment / "bin" / product.command)], [str(python), "-m", module]):
                said = subprocess.run([*argv, "--version"], check=True, capture_output=True, text=True, encoding="utf-8").stdout
                if not version_matches(product, said):
                    raise AssertionError(f"{' '.join(argv)} --version printed {said!r}")
