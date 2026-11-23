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
        command.add_argument("workers", type=Path, help="directory written by build.py")
        if verb == "measure":
            command.add_argument("--tm", required=True, type=Path, help="pinned pristine tm executable")
            command.add_argument("--workspace", choices=WORKSPACES, action="append")
            command.add_argument("--repeats", type=int, default=3)
            command.add_argument("--json", type=Path, help="save hashes, individual runs and load")
    args = parser.parse_args()
    workers = args.workers.resolve(strict=True)
    verify_build(workers, "reference")
    if args.verb != "verify-reference-cases":
        verify_build(workers, "candidate")
    if args.verb == "measure":
        run(ROOT, workers, args.tm.resolve(strict=True), args.workspace or WORKSPACES, args.repeats, args.json)
    else:
        raise SystemExit(int(not verify(workers, args.verb == "verify-reference-cases")))
