"""The `pending` step done through the `tag` job."""

from __future__ import annotations

import typing

from ci.changes.pending.interface import Pending
from ci.tag.use_case import Request

if typing.TYPE_CHECKING:
    from ci.tag.use_case import UseCase


class TagPending(Pending):
    """`pending` through the `tag` job, asked without pushing: the products its release would tag at the checkout's commit, each
    whose workspace version has no tag yet. The one answer to what is released, the release's own."""

    def __init__(self, job: UseCase) -> None:
        self.job = job

    def unreleased(self) -> frozenset[str]:
        return frozenset(self.job.run(Request("HEAD", push=False)))
