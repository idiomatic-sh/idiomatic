"""Tastemaker's verify-cases and measure verbs."""
import argparse
from pathlib import Path
import sys

# The benchmark itself can run from this checkout without adding files to it.
sys.dont_write_bytecode = True

from cases import verify
from build import verify_build
from corpus import WORKSPACES, run

ROOT = Path(__file__).resolve().parents[4]

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="verb", required=True)
    for verb in ("verify-cases", "verify-reference-cases", "measure"):
        command = commands.add_parser(verb)
        command.add_argument("workers", nargs="?" if verb == "measure" else None, type=Path, help="directory written by build.py")
        if verb == "measure":
            command.add_argument("--reader", type=Path, help="use this already built native vec-readers worker")
            command.add_argument("--observer", type=Path, help="use this already built release SDK observer")
            command.add_argument("--tm", required=True, type=Path, help="pinned pristine tm executable")
            command.add_argument("--workspace", choices=WORKSPACES, action="append")
            command.add_argument("--repeats", type=int, default=3)
            command.add_argument("--json", type=Path, help="save hashes, individual runs and load")
    args = parser.parse_args()
    if args.verb == "measure":
        if args.workers is not None and args.reader is None and args.observer is None:
            workers = args.workers.resolve(strict=True)
            verify_build(workers, "reference")
            verify_build(workers, "candidate")
            reader, observer = workers / "tm-check-reader", workers / "tm-observer"
        elif args.workers is None and args.reader and args.observer:
            reader, observer = args.reader.resolve(strict=True), args.observer.resolve(strict=True)
        else:
            parser.error("measure requires a built workers directory or both --reader and --observer")
        run(ROOT, reader, observer, args.tm.resolve(strict=True), args.workspace or WORKSPACES, args.repeats, args.json)
    else:
        workers = args.workers.resolve(strict=True)
        verify_build(workers, "reference")
        if args.verb == "verify-cases":
            verify_build(workers, "candidate")
        raise SystemExit(int(not verify(workers, args.verb == "verify-reference-cases")))
