"""PreToolUse hook for Write, Edit, MultiEdit and NotebookEdit: review evidence is written only into a claude/checkpoint-N worktree.

Applies guard_shell.evidence_write_problem to tool_input.file_path. It fails closed on its own errors for
evidence-named paths and lets every other path through.
"""

import importlib.util
import json
import os
import re
import sys

NAMED = re.compile(r"(rust-review|contract-review|leak-review|tm-steward)[^/]*\.json$|evidence/branches/")


def main():
    try:
        data = json.load(sys.stdin)
    except ValueError:
        return 0
    given = (data.get("tool_input") or {}) if isinstance(data, dict) else {}
    path = given.get("file_path") or given.get("notebook_path")
    if not isinstance(path, str):
        return 0
    try:
        spec = importlib.util.spec_from_file_location("guard_shell", os.path.join(os.path.dirname(os.path.abspath(__file__)), "guard_shell.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        found = module.evidence_write_problem(path, data.get("cwd") or os.getcwd())
    except Exception as error:
        if NAMED.search(path):
            print("Blocked: guard failed closed: " + type(error).__name__ + ": " + str(error), file=sys.stderr)
            return 2
        return 0
    if found:
        print("Blocked: " + found, file=sys.stderr)
        return 2
    return 0


sys.exit(main())
