"""The `publish` step's port: what every adapter of the step does."""

from __future__ import annotations

import typing

if typing.TYPE_CHECKING:
    from ci.release.use_case import Artifact, Product


class Publish(typing.Protocol):
    """The `publish` step, done one way per registry. A dry run is a mode, not a step: the
    registry reads the package as it would upload it and uploads nothing."""

    def publish(self, product: Product, artifact: Artifact, dry_run: bool) -> None:
        """Hands `artifact` to the registry's publish: its dry run when `dry_run`, an upload
        through trusted publishing otherwise."""
        ...
