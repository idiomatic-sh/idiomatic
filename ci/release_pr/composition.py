"""The `release_pr` job's composition: the one place its adapters are chosen."""

from __future__ import annotations

import dataclasses
import typing
from pathlib import Path

from ci.release_pr.check.adapters.github.adapter import GithubCheck
from ci.release_pr.placement.adapters.github.adapter import GithubPlacement
from ci.release_pr.release.adapters.release_plz.adapter import ReleasePlzRelease
from ci.release_pr.use_case import UseCase

if typing.TYPE_CHECKING:
    from ci.release_pr.check.interface import Check
    from ci.release_pr.placement.interface import Placement
    from ci.release_pr.release.interface import Release

# The repository this job's file sits in.
ROOT = Path(__file__).resolve().parents[2]


@dataclasses.dataclass(frozen=True)
class Parts:
    """The ports the job is composed from: the release pull request, the product's placement on its branch, and its check."""

    release: Release
    placement: Placement
    check: Check


def build() -> UseCase:
    """The job with its real adapters."""
    return compose(Parts(release=ReleasePlzRelease(ROOT), placement=GithubPlacement(ROOT), check=GithubCheck(ROOT)))


def compose(parts: Parts) -> UseCase:
    """The job wired from supplied parts: the seam its intent test fakes every port through."""
    return UseCase(parts.release, parts.placement, parts.check)
