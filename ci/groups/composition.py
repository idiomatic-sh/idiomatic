"""The `groups` job's composition: the one place its adapters are chosen."""

from __future__ import annotations

import dataclasses
import typing
from pathlib import Path

from ci.groups.manifest.adapters.file.adapter import FileManifest
from ci.groups.use_case import UseCase

if typing.TYPE_CHECKING:
    from ci.groups.manifest.interface import Manifest

# The repository this job's file sits in.
ROOT = Path(__file__).resolve().parents[2]


@dataclasses.dataclass(frozen=True)
class Parts:
    """The ports the job is composed from: the products' asymptote manifests."""

    manifest: Manifest


def build() -> UseCase:
    """The job with its real adapters."""
    return compose(Parts(manifest=FileManifest(ROOT)))


def compose(parts: Parts) -> UseCase:
    """The job wired from supplied parts: the seam its intent test fakes every port through."""
    return UseCase(parts.manifest)
