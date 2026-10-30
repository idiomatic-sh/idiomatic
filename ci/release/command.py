# /// script
# requires-python = ">=3.12"
# ///
"""The `release` job's command: parses its command line, composes the job and runs it.

It runs the same on a laptop and in CI: `uv run ci/release/command.py --help`.

    uv run ci/release/command.py --dist dist --dry-run

The build job made, on each target's own machine, the wheel of every shipped product; `--dist` is the
directory that holds them, one directory per target. The job installs and runs what this machine can, hands
every package to its registry's dry-run publish (the wheels, npm's platform packages and launchers made from
the wheels' binaries, the crates) and writes each product's GitHub Release archives to
`dist/archives/<package>-v<version>`, the tag of that product's release. `--publish`
uploads, after the dry run passes for everything, to crates.io, then npm, then PyPI; only the maintainer runs it.
"""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

if not __package__:
    # Run as a script: the repository root is the import root, as it is for
    # `python -m ci.release.command` and for pytest.
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from ci.release import composition  # noqa: E402
from ci.release.use_case import Refused, Request, UseCase  # noqa: E402


def parse(argv: list[str]) -> argparse.Namespace:
    """The command line's arguments."""
    parser = argparse.ArgumentParser(prog="ci/release/command.py", description=__doc__.splitlines()[0])
    parser.add_argument("--dist", type=Path, required=True, help="the directory of the build job's artifacts: a directory per target")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--dry-run", action="store_true", help="hand every package to its registry's dry run; upload nothing")
    mode.add_argument("--publish", action="store_true", help="upload, after the dry run passes (maintainers only)")
    parser.add_argument("--product", action="append", default=[], help="a product to release (default: every shipped one)")
    return parser.parse_args(argv)


def run(argv: list[str], compose: Callable[[Path], UseCase]) -> int:
    """Runs the job `compose` makes over the artifacts the command line names: 0 when it passes,
    1 when a step fails, 2 when the request is refused."""
    arguments = parse(argv)
    with tempfile.TemporaryDirectory(prefix="release-") as scratch:
        try:
            return compose(arguments.dist.resolve()).run(Request(tuple(arguments.product), arguments.dry_run, Path(scratch)))
        except Refused as refused:
            print(f"release: {refused}", file=sys.stderr)
            return 2


def main() -> int:
    """The entry: the job with its real adapters."""
    return run(sys.argv[1:], composition.build)


if __name__ == "__main__":
    sys.exit(main())
