# /// script
# requires-python = ">=3.12"
# ///
"""The `release_pr` job's command: parses its command line, composes the job and runs it.

It runs the same on a laptop and in CI: `uv run ci/release_pr/command.py --help`.

    uv run ci/release_pr/command.py --product ryo

It opens the product's release pull request from main's changes since its last release, or updates the one open, with
`release-plz` (GH_TOKEN the token it opens it with): every crate of the product at one new version and the product's
`CHANGELOG.md`, titled `chore(<product>): release <version>`. It then commits the new version to the product's `tastemaker.toml`
on the pull request's branch and starts the check workflow there for the product, whose `gate` the pull request needs. It writes
the pull request's address, or nothing when the product has nothing unreleased. Merging the pull request releases the product.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

if not __package__:
    # Run as a script: the repository root is the import root, as it is for
    # `python -m ci.release_pr.command` and for pytest.
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from ci.release_pr import composition  # noqa: E402
from ci.release_pr.use_case import Refused, Request, UseCase  # noqa: E402
from ci.tag.use_case import PRODUCTS  # noqa: E402


def parse(argv: list[str]) -> argparse.Namespace:
    """The command line's arguments."""
    parser = argparse.ArgumentParser(prog="ci/release_pr/command.py", description=__doc__.splitlines()[0])
    parser.add_argument("--product", required=True, choices=PRODUCTS, help="the product whose release pull request to open")
    return parser.parse_args(argv)


def run(argv: list[str], compose: Callable[[], UseCase]) -> int:
    """Runs the job `compose` makes as the command line asks and writes the pull request's address: 0 when it runs, a product
    with nothing unreleased included, 2 when the release pull request is refused."""
    arguments = parse(argv)
    try:
        opened = compose().run(Request(arguments.product))
    except Refused as refused:
        print(f"release_pr: {refused}", file=sys.stderr)
        return 2
    if opened is None:
        print(f"release_pr: {arguments.product} has nothing unreleased", file=sys.stderr)
        return 0
    print(opened.url)
    return 0


def main() -> int:
    """The entry: the job with its real adapters."""
    return run(sys.argv[1:], composition.build)


if __name__ == "__main__":
    sys.exit(main())
