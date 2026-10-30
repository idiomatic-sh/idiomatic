"""The `manifest` step done through cargo: Cargo's metadata and npm's build file."""

from __future__ import annotations

import json
import subprocess
import tomllib
from pathlib import Path

from ci.release.manifest.interface import Manifest
from ci.release.use_case import Product

# A product ships when its distribution directory holds Cargo's registry (the `distribution` spec): every product ships to crates.io.
SHIPPED = "distribution/registries/cargo/smoke_test.rs"
# The platforms of a product that ships to npm and PyPI are the ones npm's build fixes; a product that ships to Cargo alone has the
# release's, the one set every npm build fixes (the `distribution` spec), which its GitHub Release archives.
BUILD = "distribution/registries/npm/dist-workspace.toml"


def cli_package(metadata: dict) -> dict:
    """The command line package of a product's Cargo metadata: the one that declares the
    `distribution` test target."""
    found = [
        package
        for package in metadata["packages"]
        if any(target["name"] == "distribution" and "test" in target["kind"] for target in package["targets"])
    ]
    if len(found) != 1:
        raise ValueError(f"expected one package with a `distribution` test target, found {len(found)}")
    return found[0]


class CargoManifest(Manifest):
    """`manifest` through cargo: `cargo metadata` for the command line package, and the
    platforms of npm's build, the ones the distribution spec fixes for every registry that has
    platforms."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def shipped(self) -> tuple[str, ...]:
        return tuple(sorted(path.parents[3].name for path in self.root.glob(f"crates/*/{SHIPPED}")))

    def read(self, product: str) -> Product:
        directory = self.root / "crates" / product
        metadata = json.loads(
            subprocess.run(
                ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
                cwd=directory,
                check=True,
                capture_output=True,
                text=True, encoding="utf-8",
            ).stdout
        )
        package = cli_package(metadata)
        (command,) = [target["name"] for target in package["targets"] if "bin" in target["kind"]]
        build = directory / BUILD
        targets = tomllib.loads(build.read_text())["dist"]["targets"] if build.is_file() else self.release_targets()
        return Product(
            name=product,
            package=package["name"],
            version=package["version"],
            license=package["license"],
            repository=package["repository"],
            # Cargo's metadata says `publish: []` for `publish = false`.
            publishable=package["publish"] != [],
            command=command,
            targets=tuple(sorted(targets)),
        )

    def release_targets(self) -> list[str]:
        """The release's targets: the one set every product's npm build fixes."""
        fixed = {tuple(sorted(tomllib.loads(path.read_text())["dist"]["targets"])) for path in self.root.glob(f"crates/*/{BUILD}")}
        if len(fixed) > 1:
            raise ValueError(f"the npm builds disagree on the release's targets: {sorted(fixed)}")
        return list(next(iter(fixed), ()))

    def host(self) -> str:
        said = subprocess.run(["rustc", "-vV"], cwd=self.root, check=True, capture_output=True, text=True, encoding="utf-8").stdout
        (host,) = [line.removeprefix("host: ") for line in said.splitlines() if line.startswith("host: ")]
        return host
