"""The `build` step done for the GitHub Release: a target's bare binary, archived."""

from __future__ import annotations

import gzip
import io
import tarfile
import zipfile
from pathlib import Path

from ci.release.build.interface import Build
from ci.release.use_case import SHARED, Artifact, Product, Refused

# The directory of `dist` the archives are written to: one directory per product, named for the tag of its GitHub
# Release, `<package>-v<version>`, which the workflow creates and uploads the directory to.
ARCHIVES = "archives"
EXECUTABLE = 0o755


def tag(product: Product) -> str:
    """The tag of the product's own GitHub Release, which cargo-binstall's `pkg-url` names."""
    return f"{product.package}-v{product.version}"


def archive(product: Product, target: str, binary: bytes, into: Path) -> Path:
    """The archive of `binary`, named `<package>-<target>`: a zip on Windows and a tar.gz elsewhere,
    holding the executable alone, the same bytes on every run."""
    windows = "-windows-" in target
    name = f"{product.command}.exe" if windows else product.command
    into.mkdir(parents=True, exist_ok=True)
    path = into / f"{product.package}-{target}{'.zip' if windows else '.tar.gz'}"
    if windows:
        member = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
        member.external_attr = EXECUTABLE << 16
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as opened:
            opened.writestr(member, binary)
        return path
    info = tarfile.TarInfo(name)
    info.size = len(binary)
    info.mode = EXECUTABLE
    with path.open("wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as zipped, tarfile.open(fileobj=zipped, mode="w") as opened:
        opened.addfile(info, io.BytesIO(binary))
    return path


class GithubBuild(Build):
    """`build` for the GitHub Release: for a target, the archive of its binary, taken from the wheel
    the build job made; nothing that every target shares."""

    def __init__(self, root: Path, dist: Path) -> None:
        self.root = root
        self.dist = dist

    def build(self, product: Product, scope: str, into: Path) -> list[Artifact]:
        if scope == SHARED:
            return []
        pattern = f"{product.package.replace('-', '_')}-*.whl"
        wheels = list((self.dist / scope).glob(pattern))
        if not wheels:
            raise Refused(f"no wheel {self.dist / scope / pattern}: run the build job for {scope} first")
        (wheel,) = wheels
        with zipfile.ZipFile(wheel) as opened:
            (member,) = [name for name in opened.namelist() if name.endswith((f".data/scripts/{product.command}", f".data/scripts/{product.command}.exe"))]
            binary = opened.read(member)
        return [Artifact("github", archive(product, scope, binary, self.dist / ARCHIVES / tag(product)), scope)]
