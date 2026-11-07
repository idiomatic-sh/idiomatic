"""The `check` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Check(typing.Protocol):
    """The `check` step, done one way per adapter: the check workflow started for a product on a branch."""

    def start(self, product: str, branch: str) -> None:
        """Starts the check workflow on `branch`'s head commit for `product` alone."""
        ...
