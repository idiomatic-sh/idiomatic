"""The `build` step done through npm: the binary of a target's wheel repackaged as npm's platform
package, and the launcher package (the `distribution` spec)."""

from __future__ import annotations

import json
import shutil
import subprocess
import zipfile
from pathlib import Path

from ci.release.build.interface import Build
from ci.release.use_case import SHARED, Artifact, Product, Refused


def binary(wheel: Path, command: str) -> bytes:
    """The executable named `command` that the wheel installs: the binary the build job made, which the
    platform package repackages."""
    with zipfile.ZipFile(wheel) as opened:
        (member,) = [name for name in opened.namelist() if name.endswith((f".data/scripts/{command}", f".data/scripts/{command}.exe"))]
        return opened.read(member)


def stage(source: Path, into: Path, product: Product, license_text: Path) -> None:
    """Copies the package whose `package.json` is in `source` into `into`, stamped with the
    crate's version, license and repository (Cargo is the one version authority), beside the
    repository's LICENSE text, which npm packs whatever the package's `files` say."""
    into.mkdir(parents=True)
    shutil.copy(source / "package.json", into / "package.json")
    shutil.copy(license_text, into / "LICENSE")
    subprocess.run(
        ["npm", "pkg", "set", f"version={product.version}", f"license={product.license}", f"repository={product.repository}"],
        cwd=into,
        check=True,
    )


def pack(staged: Path, into: Path) -> Path:
    """Packs the staged package into `into` and returns the tarball."""
    into.mkdir(parents=True, exist_ok=True)
    said = subprocess.run(
        ["npm", "pack", "--json", "--pack-destination", str(into)], cwd=staged, check=True, capture_output=True, text=True, encoding="utf-8"
    ).stdout
    (packed,) = json.loads(said)
    return into / packed["filename"]


class NpmBuild(Build):
    """`build` through npm: for a target, the platform package holding the binary of its wheel; for
    what every target shares, the launcher, depending on every planned platform's package at the
    crate's version, published after them. Nothing for a product that has no npm registry."""

    def __init__(self, root: Path, dist: Path) -> None:
        self.root = root
        self.dist = dist

    def build(self, product: Product, scope: str, into: Path) -> list[Artifact]:
        npm = self.root / "crates" / product.name / "distribution" / "registries" / "npm"
        if not npm.is_dir():
            return []
        if scope == SHARED:
            return [Artifact("npm", self.launcher(product, npm, into), SHARED)]
        return [Artifact("npm", self.platform(product, npm, scope, into), scope)]

    def platform(self, product: Product, npm: Path, target: str, into: Path) -> Path:
        pattern = f"{product.package.replace('-', '_')}-*.whl"
        wheels = list((self.dist / target).glob(pattern))
        if not wheels:
            raise Refused(f"no wheel {self.dist / target / pattern}: run the build job for {target} first")
        (wheel,) = wheels
        staged = into / "platform"
        stage(npm / "platforms" / target, staged, product, self.root / "LICENSE")
        (staged / "bin").mkdir()
        name = f"{product.command}.exe" if "-windows-" in target else product.command
        executable = staged / "bin" / name
        executable.write_bytes(binary(wheel, product.command))
        executable.chmod(0o755)
        return pack(staged, into / "packed")

    def launcher(self, product: Product, npm: Path, into: Path) -> Path:
        staged = into / "launcher"
        stage(npm, staged, product, self.root / "LICENSE")
        shutil.copy(npm / "launcher.js", staged / "launcher.js")
        dependencies = [
            f"optionalDependencies.{json.loads((npm / 'platforms' / target / 'package.json').read_text())['name']}={product.version}"
            for target in product.targets
        ]
        subprocess.run(["npm", "pkg", "set", *dependencies], cwd=staged, check=True)
        return pack(staged, into / "packed")
