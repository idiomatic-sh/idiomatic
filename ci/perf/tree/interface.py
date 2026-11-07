"""The `tree` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing

if typing.TYPE_CHECKING:
    from pathlib import Path


class Tree(typing.Protocol):
    """The `tree` step, done one way per adapter: what the checkout changed against a base, and the base's own tree beside it."""

    def changed(self, base: str) -> tuple[str, ...]:
        """The paths, relative to the repository root, the checkout changed since it left `base`."""
        ...

    def checkout(self, base: str, into: Path) -> Path:
        """`base`'s tree checked out at `into`, which is returned."""
        ...

    def remove(self, tree: Path) -> None:
        """Removes a tree `checkout` made."""
        ...
