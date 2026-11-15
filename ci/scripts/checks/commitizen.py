# /// script
# requires-python = ">=3.12"
# ///
"""The root `commitizen` tool: the message main keeps is a conventional commit (cz.toml), read from the event GitHub hands the run.

    uv run ci/scripts/checks/commitizen.py "$GITHUB_EVENT_PATH"

Main only takes squash merges, so a pull request lands as one commit whose message is the pull request's title: a pull request's
event checks its title, and a branch's own commits, which main never keeps, are not checked. A push to main checks the commits the
push added, the event's `before..after`; the push that creates main checks its head commit. Any other event, and a run with no
event (the git hook, a local check), has no message main keeps to check, and passes.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

MAIN = "refs/heads/main"
# The `before` of a push that created its branch.
CREATED = "0" * 40


def arguments(event: dict) -> list[str] | None:
    """The `cz check` arguments for the messages `event` lands on main, or None when it lands none."""
    if "pull_request" in event:
        return ["--message", event["pull_request"]["title"]]
    if event.get("ref") == MAIN and "after" in event:
        after = event["after"]
        before = event.get("before", CREATED)
        if before == CREATED:
            message = subprocess.run(["git", "log", "-1", "--format=%B", after], capture_output=True, text=True, check=True).stdout
            return ["--message", message.strip()]
        return ["--rev-range", f"{before}..{after}"]
    return None


def main(argv: list[str]) -> int:
    path = argv[1] if len(argv) > 1 else ""
    if not path:
        print("commitizen: no event, so no message main keeps to check", file=sys.stderr)
        return 0
    found = arguments(json.loads(Path(path).read_text()))
    if found is None:
        print("commitizen: the event lands no message on main", file=sys.stderr)
        return 0
    return subprocess.run(["cz", "check", *found], check=False).returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv))
