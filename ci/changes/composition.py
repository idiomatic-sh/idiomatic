"""The `changes` job's composition: the one place its adapters are chosen."""

from __future__ import annotations

import dataclasses
import typing
from pathlib import Path

from ci.changes.builds.adapters.file.adapter import FileBuilds
from ci.changes.diff.adapters.git.adapter import GitDiff
from ci.changes.pending.adapters.tag.adapter import TagPending
from ci.changes.use_case import UseCase
from ci.tag import composition as tag_composition

if typing.TYPE_CHECKING:
    from ci.changes.builds.interface import Builds
    from ci.changes.diff.interface import Diff
    from ci.changes.pending.interface import Pending

# The repository this job's file sits in.
ROOT = Path(__file__).resolve().parents[2]


@dataclasses.dataclass(frozen=True)
class Parts:
    """The ports the job is composed from: the paths a pull request changes, the products a product builds against, and the
    products not released yet."""

    diff: Diff
    builds: Builds
    pending: Pending


def build() -> UseCase:
    """The job with its real adapters."""
    return compose(Parts(diff=GitDiff(ROOT), builds=FileBuilds(ROOT), pending=TagPending(tag_composition.build())))


def compose(parts: Parts) -> UseCase:
    """The job wired from supplied parts: the seam its intent test fakes every port through."""
    return UseCase(parts.diff, parts.builds, parts.pending)
