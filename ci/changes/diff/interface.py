"""The `diff` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Diff(typing.Protocol):
    """The `diff` step, done one way per adapter: the paths a pull request changes."""

    def paths(self) -> tuple[str, ...]:
        """Every path the pull request adds, changes or removes against the branch it merges into, from the repository's top."""
        ...
