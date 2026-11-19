"""The `manifest` step done through cargo: Cargo's metadata and npm's platform packages."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

from ci.release.manifest.interface import Manifest
from ci.release.use_case import Product

# A product ships when its command line crate holds the crates.io smoke test (the `cli` spec): every product ships to crates.io.
SHIPPED = "cli/tests/smoke.rs"
# The platforms of a product that ships to npm are the directory names of its npm platform packages; a product that ships to
# crates.io alone has the release's, the one set every product's npm platforms name (the `distribution` spec), which its GitHub
# Release archives.
PLATFORMS = "distribution/npm/platforms"


def cli_package(metadata: dict) -> dict:
    """The command line package of a product's Cargo metadata: the one that declares the
    `smoke` test target."""
    found = [
        package
        for package in metadata["packages"]
        if any(target["name"] == "smoke" and "test" in target["kind"] for target in package["targets"])
    ]
    if len(found) != 1:
        raise ValueError(f"expected one package with a `smoke` test target, found {len(found)}")
    return found[0]


def platform_names(platforms: Path) -> list[str]:
    """The targets npm ships to: the directory names under a product's npm platforms, sorted."""
    return sorted(path.name for path in platforms.iterdir() if path.is_dir())


class CargoManifest(Manifest):
    """`manifest` through cargo: `cargo metadata` for the command line package, and the
    platforms of npm's platform packages, the ones the distribution spec fixes for every registry
    that has platforms."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def shipped(self) -> tuple[str, ...]:
        return tuple(sorted(path.parents[2].name for path in self.root.glob(f"crates/*/{SHIPPED}")))

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
        platforms = directory / PLATFORMS
        targets = platform_names(platforms) if platforms.is_dir() else self.release_targets()
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
        """The release's targets: the one set every product's npm platform packages name."""
        fixed = {tuple(platform_names(path)) for path in self.root.glob(f"crates/*/{PLATFORMS}")}
        if len(fixed) > 1:
            raise ValueError(f"the npm platforms disagree on the release's targets: {sorted(fixed)}")
        return list(next(iter(fixed), ()))

    def host(self) -> str:
        said = subprocess.run(["rustc", "-vV"], cwd=self.root, check=True, capture_output=True, text=True, encoding="utf-8").stdout
        (host,) = [line.removeprefix("host: ") for line in said.splitlines() if line.startswith("host: ")]
        return host
