"""The `placement` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing


class Placement(typing.Protocol):
    """The `placement` step, done one way per adapter: a product's `tastemaker.toml` on a branch, read and committed."""

    def read(self, product: str, branch: str) -> str:
        """`crates/<product>/tastemaker.toml` as `branch` holds it."""
        ...

    def write(self, product: str, branch: str, text: str, message: str) -> None:
        """Commits `text` as `crates/<product>/tastemaker.toml` on `branch`, with the commit message `message`."""
        ...
