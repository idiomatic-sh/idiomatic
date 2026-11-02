"""The `release` step's port: what every adapter of the step does."""

from __future__ import annotations

import dataclasses
import typing


@dataclasses.dataclass(frozen=True)
class Opened:
    """A release pull request, opened or updated: its number, its address, its branch and the version it moves each crate to."""

    number: int
    url: str
    branch: str
    versions: tuple[str, ...]


class Release(typing.Protocol):
    """The `release` step, done one way per adapter: a product's release pull request."""

    def open(self, product: str) -> Opened | None:
        """Opens `product`'s release pull request from main's changes since its last release, or updates the one open; None
        when nothing is unreleased."""
        ...
