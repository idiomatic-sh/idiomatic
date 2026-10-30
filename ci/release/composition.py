"""The `release` job's composition: the one place its adapters are chosen."""

from __future__ import annotations

import dataclasses
import typing
from pathlib import Path

from ci.release.build.adapters.cargo.adapter import CargoBuild
from ci.release.build.adapters.github.adapter import GithubBuild
from ci.release.build.adapters.npm.adapter import NpmBuild
from ci.release.build.adapters.pypi.adapter import PypiBuild
from ci.release.manifest.adapters.cargo.adapter import CargoManifest
from ci.release.publish.adapters.cargo.adapter import CargoPublish
from ci.release.publish.adapters.npm.adapter import NpmPublish
from ci.release.publish.adapters.pypi.adapter import PypiPublish
from ci.release.smoke.adapters.npm.adapter import NpmSmoke
from ci.release.smoke.adapters.pypi.adapter import PypiSmoke
from ci.release.use_case import Registry, UseCase

if typing.TYPE_CHECKING:
    from collections.abc import Mapping

    from ci.release.manifest.interface import Manifest

# The repository this job's file sits in.
ROOT = Path(__file__).resolve().parents[2]


@dataclasses.dataclass(frozen=True)
class Parts:
    """The ports the job is composed from: the manifests, and per registry, in the order
    they run (crates.io first), its build, its smoke when it has one, and its publish when it has one."""

    manifest: Manifest
    registries: Mapping[str, Registry]


def build(dist: Path) -> UseCase:
    """The job with its real adapters, over the artifacts the build job left in `dist`: a directory per target."""
    return compose(
        Parts(
            manifest=CargoManifest(ROOT),
            registries={
                "cargo": Registry(CargoBuild(ROOT), None, CargoPublish()),
                "npm": Registry(NpmBuild(ROOT, dist), NpmSmoke(ROOT), NpmPublish()),
                "pypi": Registry(PypiBuild(ROOT, dist), PypiSmoke(ROOT), PypiPublish()),
                # The release's archives upload through the workflow, with `gh`.
                "github": Registry(GithubBuild(ROOT, dist), None, None),
            },
        )
    )


def compose(parts: Parts) -> UseCase:
    """The job wired from supplied parts: the seam its intent test fakes every port through."""
    return UseCase(parts.manifest, parts.registries)
