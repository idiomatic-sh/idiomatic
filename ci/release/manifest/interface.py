"""The `manifest` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing

if typing.TYPE_CHECKING:
    from ci.release.use_case import Product


class Manifest(typing.Protocol):
    """The `manifest` step, done one way per adapter: the shipped products, as their own
    manifests describe them, and the machine the job runs on."""

    def shipped(self) -> tuple[str, ...]:
        """The products that ship, by directory name."""
        ...

    def read(self, product: str) -> Product:
        """A shipped product: its names, version, license, repository, whether it may be
        published, its command and its targets."""
        ...

    def host(self) -> str:
        """The target triple of the machine the job runs on."""
        ...
