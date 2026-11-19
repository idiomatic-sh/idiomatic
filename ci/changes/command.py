# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""The `changes` job's command: parses its command line, composes the job and runs it.

It runs the same on a laptop and in CI: `uv run ci/changes/command.py --help`.

    uv run ci/changes/command.py --event=pull_request --product=
    uv run ci/changes/command.py --event=workflow_dispatch --product=tastemaker

It prints, one per line as `<name>=<JSON>`, what the run checks: `products`, the products check.yml's Linux job runs; `rows`,
the release's check rows, each a product and a row; `skips`, the jobs left with nothing to run, which their own conditions skip
and the gate lets skip. On GitHub Actions it writes the same lines as the step's outputs.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

if not __package__:
    # Run as a script: the repository root is the import root, as it is for
    # `python -m ci.changes.command` and for pytest.
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from ci.changes import composition  # noqa: E402
from ci.changes.use_case import Refused, Request, UseCase  # noqa: E402


def parse(argv: list[str]) -> Request:
    """The request the command line asks for: the event that started the run, and a hand run's product (empty otherwise)."""
    parser = argparse.ArgumentParser(prog="ci/changes/command.py", description=__doc__.splitlines()[0])
    parser.add_argument("--event", required=True, help="the event that started the run: pull_request, push or workflow_dispatch")
    parser.add_argument("--product", default="", help="a hand run's product, a directory under crates/")
    arguments = parser.parse_args(argv)
    return Request(arguments.event, arguments.product)


def run(argv: list[str], job: UseCase) -> int:
    """Runs `job` as the command line asks and writes what the run checks: 0 when it runs, 2 when the request is refused."""
    try:
        plan = job.run(parse(argv))
    except Refused as refused:
        print(f"changes: {refused}", file=sys.stderr)
        return 2
    lines = "".join(
        f"{name}={json.dumps(list(value), separators=(',', ':'))}\n"
        for name, value in (("products", plan.products), ("rows", plan.rows), ("skips", plan.skips))
    )
    print(lines, end="")
    # GitHub Actions reads a step's outputs from the file it names in GITHUB_OUTPUT.
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with Path(output).open("a", encoding="utf-8") as file:
            file.write(lines)
    return 0


def main() -> int:
    """The entry: the job with its real adapters."""
    return run(sys.argv[1:], composition.build())


if __name__ == "__main__":
    sys.exit(main())
