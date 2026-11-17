# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""The `groups` job's command: parses its command line, composes the job and runs it.

It runs the same on a laptop and in CI: `uv run ci/groups/command.py --help`.

    uv run ci/groups/command.py --product asymptote --groups lint,check,memory

It prints, comma separated, the groups of `--groups` that the product's tools declare, and on GitHub Actions writes them as the
step's `groups` output, which the job's one `asymptote check` call names.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

if not __package__:
    # Run as a script: the repository root is the import root, as it is for
    # `python -m ci.groups.command` and for pytest.
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from ci.groups import composition  # noqa: E402
from ci.groups.use_case import Refused, Request, UseCase  # noqa: E402


def parse(argv: list[str]) -> Request:
    """The request the command line asks for."""
    parser = argparse.ArgumentParser(prog="ci/groups/command.py", description=__doc__.splitlines()[0])
    parser.add_argument("--product", required=True, help="the product, a directory under crates/")
    parser.add_argument("--groups", required=True, help="the groups the job asks of every product, comma separated")
    arguments = parser.parse_args(argv)
    return Request(arguments.product, tuple(arguments.groups.split(",")))


def run(argv: list[str], job: UseCase) -> int:
    """Runs `job` as the command line asks and writes the groups it keeps: 0 when it runs, 2 when the request is refused."""
    try:
        kept = ",".join(job.run(parse(argv)))
    except Refused as refused:
        print(f"groups: {refused}", file=sys.stderr)
        return 2
    print(kept)
    # GitHub Actions reads a step's outputs from the file it names in GITHUB_OUTPUT.
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with Path(output).open("a", encoding="utf-8") as file:
            file.write(f"groups={kept}\n")
    return 0


def main() -> int:
    """The entry: the job with its real adapters."""
    return run(sys.argv[1:], composition.build())


if __name__ == "__main__":
    sys.exit(main())
