"""PreToolUse hook for Agent: refuses a subagent spawning further subagents."""

import json
import os
import sys

MESSAGE = ("Blocked: a builder never spawns subagents; do the work serially in your own worktree, "
           "or reply BLOCKED: <one question>")


def main():
    try:
        data = json.load(sys.stdin)
    except ValueError:
        return 0
    if not isinstance(data, dict) or data.get("tool_name") != "Agent":
        return 0
    path = data.get("transcript_path")
    if isinstance(path, str) and "subagents" in os.path.normpath(path).split(os.sep)[:-1]:
        print(MESSAGE, file=sys.stderr)
        return 2
    return 0


sys.exit(main())
