"""The `tag` job's use case: the order of its steps over their ports, and nothing else.

A product is released by the first green commit whose workspace version has no tag yet, `idiomatic-sh-<product>-v<version>`: the commit
that changes it, or, when that commit's check was red, the next green one (a rerun finds the tag made). A product whose manifest declares no workspace version at the commit is refused before any tag is created. Asked to, the job creates each such
product's tag at the commit; the release then builds and publishes those products alone.
"""

from __future__ import annotations

import dataclasses
import typing

if typing.TYPE_CHECKING:
    from ci.tag.manifest.interface import Manifest
    from ci.tag.tags.interface import Tags

# The products, each a workspace under `crates/<product>` released on its own.
PRODUCTS = ("ryo", "tastemaker", "asymptote")


def tag(product: str, version: str) -> str:
    """A product's release tag, the one its GitHub Release and cargo-binstall name."""
    return f"idiomatic-sh-{product}-v{version}"


class Refused(Exception):
    """A request the job cannot run, said before any tag is created."""


@dataclasses.dataclass(frozen=True)
class Request:
    """What one run of the job is asked to do: the commit to judge, and whether to create the tags."""

    revision: str
    push: bool


class UseCase:
    """The job, over the ports its composition supplies."""

    def __init__(self, manifest: Manifest, tags: Tags) -> None:
        self.manifest = manifest
        self.tags = tags

    def run(self, request: Request) -> tuple[str, ...]:
        """The products `request.revision` releases, in `PRODUCTS`' order, each one's tag created when `request.push` asks."""
        released = []
        for product in PRODUCTS:
            version = self.manifest.version(product, request.revision)
            if version is None:
                raise Refused(f"crates/{product}/Cargo.toml declares no workspace version at {request.revision}")
            if not self.tags.exists(tag(product, version)):
                released.append((product, version))
        if request.push:
            for product, version in released:
                self.tags.create(tag(product, version), request.revision)
        return tuple(product for product, _ in released)
