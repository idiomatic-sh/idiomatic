"""The `release` job's use case: the order of its steps over their ports, and nothing else.

Per registry, per product: build → smoke (a registry whose package is the source itself, Cargo, builds nothing to install and has no
smoke; a registry whose artifacts are not packages, the GitHub Release's archives, has no publish: the workflow uploads them). Every
artifact is handed to its registry's dry-run publish first; only a publish run then uploads, registry after registry in the order
they are composed, so nothing uploads unless everything would.
"""

from __future__ import annotations

import dataclasses
import typing
from pathlib import Path

if typing.TYPE_CHECKING:
    from collections.abc import Mapping

    from ci.release.build.interface import Build
    from ci.release.manifest.interface import Manifest
    from ci.release.publish.interface import Publish
    from ci.release.smoke.interface import Smoke

# The scope of what every target shares: npm's launcher package and the crate.
SHARED = "shared"


class Refused(Exception):
    """A request the job cannot run, said before any step runs."""


@dataclasses.dataclass(frozen=True)
class Product:
    """A shipped product, as its manifests describe it."""

    name: str
    package: str
    version: str
    license: str
    repository: str
    publishable: bool
    command: str
    targets: tuple[str, ...]


@dataclasses.dataclass(frozen=True)
class Artifact:
    """One package a registry's build made: for one target, or `SHARED` by every target."""

    registry: str
    path: Path
    scope: str


@dataclasses.dataclass(frozen=True)
class Registry:
    """A registry's steps: its build, its smoke when the build made something to install, and its publish when something else uploads."""

    build: Build
    smoke: Smoke | None
    publish: Publish | None


@dataclasses.dataclass(frozen=True)
class Request:
    """What one run of the job is asked to do: the products (every shipped one when none
    is named), whether it only dry-runs the publish, and a scratch directory it may fill."""

    products: tuple[str, ...]
    dry_run: bool
    scratch: Path


class UseCase:
    """The job, over the ports its composition supplies."""

    def __init__(self, manifest: Manifest, registries: Mapping[str, Registry]) -> None:
        self.manifest = manifest
        self.registries = registries

    def run(self, request: Request) -> int:
        """Builds, smokes and publishes every product's packages, for every target it plans:
        0 when every step passes; a failing step raises."""
        products = [self.manifest.read(name) for name in request.products or self.manifest.shipped()]
        if not products:
            raise Refused("no product ships")
        host = self.manifest.host()
        built: list[tuple[Registry, Product, Artifact]] = []
        for name, registry in self.registries.items():
            for product in products:
                directory = request.scratch / name / product.name
                artifacts = [
                    artifact
                    for scope in (*product.targets, SHARED)
                    for artifact in registry.build.build(product, scope, directory / scope)
                ]
                runnable = [each for each in artifacts if each.scope in (SHARED, host)]
                if registry.smoke is not None and runnable:
                    registry.smoke.smoke(product, runnable, directory)
                built += [(registry, product, artifact) for artifact in artifacts]
        for registry, product, artifact in built:
            if registry.publish is not None:
                registry.publish.publish(product, artifact, True)
        if not request.dry_run:
            for registry, product, artifact in built:
                if registry.publish is not None:
                    registry.publish.publish(product, artifact, False)
        return 0
