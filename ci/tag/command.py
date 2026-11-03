# /// script
# requires-python = ">=3.12"
# ///
"""The `tag` job's command: parses its command line, composes the job and runs it.

It runs the same on a laptop and in CI: `uv run ci/tag/command.py --help`.

    uv run ci/tag/command.py --dry-run

It names, as a JSON list, the products the checkout's commit releases: each whose workspace version in `crates/<product>/Cargo.toml`
differs from the parent commit's and whose tag, `idiomatic-sh-<product>-v<version>`, does not exist yet. `--push` then creates
each of those tags at the commit. On GitHub Actions the list is also the job's `products` output, which the release's build and
publish run over.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

if not __package__:
    # Run as a script: the repository root is the import root, as it is for
    # `python -m ci.tag.command` and for pytest.
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from ci.tag import composition  # noqa: E402
from ci.tag.use_case import Refused, Request, UseCase  # noqa: E402


def parse(argv: list[str]) -> argparse.Namespace:
    """The command line's arguments."""
    parser = argparse.ArgumentParser(prog="ci/tag/command.py", description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--dry-run", action="store_true", help="name the products the commit releases; create no tag")
    mode.add_argument("--push", action="store_true", help="name them and create each one's tag at the commit")
    return parser.parse_args(argv)


def run(argv: list[str], compose: Callable[[], UseCase]) -> int:
    """Runs the job `compose` makes as the command line asks and writes the products it releases: 0 when it runs, a commit that
    releases nothing included, 2 when the request is refused."""
    arguments = parse(argv)
    try:
        products = json.dumps(list(compose().run(Request("HEAD", arguments.push))))
    except Refused as refused:
        print(f"tag: {refused}", file=sys.stderr)
        return 2
    print(products)
    # GitHub Actions reads a step's outputs from the file it names in GITHUB_OUTPUT.
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with Path(output).open("a", encoding="utf-8") as file:
            file.write(f"products={products}\n")
    return 0


def main() -> int:
    """The entry: the job with its real adapters."""
    return run(sys.argv[1:], composition.build)


if __name__ == "__main__":
    sys.exit(main())
