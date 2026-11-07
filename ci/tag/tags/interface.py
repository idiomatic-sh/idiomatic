"""The `tags` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Tags(typing.Protocol):
    """The `tags` step, done one way per adapter: the repository's tags, read and created."""

    def exists(self, tag: str) -> bool:
        """Whether the repository has the tag `tag`."""
        ...

    def create(self, tag: str, revision: str) -> None:
        """Creates the tag `tag` at the commit `revision` names."""
        ...
