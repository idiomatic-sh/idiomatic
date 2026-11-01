# /// script
# requires-python = ">=3.12"
# ///
"""The `perf` job's command: parses its command line, composes the job and runs it.

It runs the same on a laptop and in CI: `uv run ci/perf/command.py --help`.

    uv run ci/perf/command.py --against origin/main --bench commands --bench lint
    uv run ci/perf/command.py --bench distribution --bench lint --output target/benchmarks

`--against` checks the change's speed: the base checked out beside the checkout, every bench the change reaches run over both,
alternately, and each measure's head/base ratio judged (above 1.15 fails, above 1.05 warns). Without it, each bench runs once over
the checkout and `--output` keeps the documents it prints.
"""

from __future__ import annotations

import argparse
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

if not __package__:
    # Run as a script: the repository root is the import root, as it is for
    # `python -m ci.perf.command` and for pytest.
    sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from ci.perf import composition  # noqa: E402
from ci.perf.use_case import PRODUCTS, Refused, Request, UseCase  # noqa: E402


def parse(argv: list[str]) -> argparse.Namespace:
    """The command line's arguments."""
    parser = argparse.ArgumentParser(prog="ci/perf/command.py", description=__doc__.splitlines()[0])
    parser.add_argument("--bench", action="append", default=[], choices=sorted(PRODUCTS), help="a bench to run (repeat for more)")
    parser.add_argument("--against", help="the base to compare the checkout's speed against, such as origin/main")
    parser.add_argument("--output", type=Path, help="the directory that keeps each document a bench prints")
    return parser.parse_args(argv)


def run(argv: list[str], compose: Callable[[Path | None], UseCase]) -> int:
    """Runs the job `compose` makes as the command line asks: 0 when it passes, 1 when a measure is too slow,
    2 when the request or measured comparison is refused."""
    arguments = parse(argv)
    output = arguments.output.resolve() if arguments.output else None
    with tempfile.TemporaryDirectory(prefix="perf-") as scratch:
        try:
            return compose(output).run(Request(tuple(arguments.bench), arguments.against, Path(scratch)))
        except Refused as refused:
            print(f"perf: {refused}", file=sys.stderr)
            return 2
        except subprocess.CalledProcessError as failed:
            print(f"perf: {shlex.join(failed.cmd)} failed with exit {failed.returncode}", file=sys.stderr)
            for label, stream in (("stdout", failed.stdout), ("stderr", failed.stderr)):
                if stream:
                    text = stream.decode("utf-8", errors="backslashreplace") if isinstance(stream, bytes) else stream
                    print(f"--- child {label} ---\n{text}", file=sys.stderr)
            return failed.returncode if failed.returncode > 0 else 2


def main() -> int:
    """The entry: the job with its real adapters."""
    return run(sys.argv[1:], composition.build)


if __name__ == "__main__":
    sys.exit(main())
