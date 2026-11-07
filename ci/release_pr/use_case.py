"""The `release_pr` job's use case: the order of its steps over their ports, and nothing else.

A maintainer asks for one product's release pull request. The `release` step opens it, or updates the one open, from main's
changes since the product's last release: every crate of the product at one new version, bumped from the conventional commits,
and the product's changelog. A product with nothing unreleased opens none, and the job stops there. Otherwise the product's
`tastemaker.toml` places its `Cargo.toml` with the version as a parameter, which the release raises too: the `placement` step
writes the new version there on the pull request's branch. Last, the `check` step starts the check workflow on that branch for
the product, since a pull request a workflow opens starts no workflow itself: its `gate` is the check the branch's head commit
needs to merge. Merging the pull request is the release: the release workflow tags and publishes each version a push to main
changes.
"""

from __future__ import annotations

import dataclasses
import re
import typing

if typing.TYPE_CHECKING:
    from ci.release_pr.check.interface import Check
    from ci.release_pr.placement.interface import Placement
    from ci.release_pr.release.interface import Opened, Release

# The line of a product's `tastemaker.toml` that places its Cargo workspace, the product's version its parameter.
WORKSPACE = re.compile(r'(\{ template = "idiomatic\.arch/cargo-workspace", file = "Cargo\.toml", params = \{ version = ")([^"]*)(")')


class Refused(Exception):
    """A release pull request the job cannot finish, said before the check starts."""


@dataclasses.dataclass(frozen=True)
class Request:
    """What one run of the job is asked to do: the product whose release pull request it opens."""

    product: str


def title(product: str, version: str) -> str:
    """The release pull request's title, the conventional commit its squash merge is."""
    return f"chore({product}): release {version}"


def placed(text: str, version: str) -> str:
    """`text`, a product's `tastemaker.toml`, with its Cargo workspace placed at `version`."""
    found = WORKSPACE.findall(text)
    if len(found) != 1:
        raise Refused(f"tastemaker.toml places the Cargo workspace {len(found)} times, not once")
    return WORKSPACE.sub(lambda match: f"{match[1]}{version}{match[3]}", text)


class UseCase:
    """The job, over the ports its composition supplies."""

    def __init__(self, release: Release, placement: Placement, check: Check) -> None:
        self.release = release
        self.placement = placement
        self.check = check

    def run(self, request: Request) -> Opened | None:
        """The release pull request `request.product`'s unreleased changes make, opened or updated, its placement at the new
        version and its check started; None when the product has nothing unreleased."""
        opened = self.release.open(request.product)
        if opened is None:
            return None
        if len(set(opened.versions)) != 1:
            raise Refused(f"the release moves {request.product}'s crates to {sorted(set(opened.versions))}, not one version")
        (version,) = set(opened.versions)
        text = self.placement.read(request.product, opened.branch)
        written = placed(text, version)
        if written != text:
            self.placement.write(request.product, opened.branch, written, title(request.product, version))
        self.check.start(request.product, opened.branch)
        return opened
