"""The `pending` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Pending(typing.Protocol):
    """The `pending` step, done one way per adapter: the products whose version is not released yet."""

    def unreleased(self) -> frozenset[str]:
        """The products whose workspace version at the checkout's commit has no release tag yet."""
        ...
