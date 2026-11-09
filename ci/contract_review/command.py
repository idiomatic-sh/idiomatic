"""The same contract-review gate for CI and local integration checkpoints."""
from pathlib import Path
import argparse
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from ci.contract_review.service import git, protected, validate


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--head", default="HEAD")
    args = parser.parse_args()
    try:
        if args.head == "HEAD":
            changed = git(args.root, "diff", "--name-only", "-z", "HEAD") + git(args.root, "ls-files", "--others", "--exclude-standard", "-z")
            if any(protected(path.decode()) for path in changed.split(b"\0") if path):
                raise ValueError("protected working tree changes must be committed before review")
        print(validate(args.root, args.head))
    except (ValueError, subprocess.CalledProcessError) as error:
        print(f"Contract review gate failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
