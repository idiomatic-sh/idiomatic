"""The `tag` job's composition: the one place its adapters are chosen."""

from __future__ import annotations

import dataclasses
import typing
from pathlib import Path

from ci.tag.manifest.adapters.git.adapter import GitManifest
from ci.tag.tags.adapters.github.adapter import GithubTags
from ci.tag.use_case import UseCase

if typing.TYPE_CHECKING:
    from ci.tag.manifest.interface import Manifest
    from ci.tag.tags.interface import Tags

# The repository this job's file sits in.
ROOT = Path(__file__).resolve().parents[2]


@dataclasses.dataclass(frozen=True)
class Parts:
    """The ports the job is composed from: the products' manifests at a commit, and the repository's tags."""

    manifest: Manifest
    tags: Tags


def build() -> UseCase:
    """The job with its real adapters."""
    return compose(Parts(manifest=GitManifest(ROOT), tags=GithubTags(ROOT)))


def compose(parts: Parts) -> UseCase:
    """The job wired from supplied parts: the seam its intent test fakes every port through."""
    return UseCase(parts.manifest, parts.tags)
