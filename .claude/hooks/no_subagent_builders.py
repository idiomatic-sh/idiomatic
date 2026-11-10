"""PreToolUse hook for Agent: refuses a subagent spawning further subagents, and a builder launched without a scope
report the scope review passed. scope_review_problem is the one validator of scope-review-<bead>.json; the
SubagentStop hook loads it by path."""

import json
import os
import re
import subprocess
import sys

MESSAGE = ("Blocked: a builder never spawns subagents; do the work serially in your own worktree, "
           "or reply BLOCKED: <one question>")
STATE = os.environ.get("CLAUDE_HOOKS_STATE", "/tmp/claude-hooks")
SCOPED = os.path.join(STATE, "scoped-branches")
RUBRIC = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".agents", "agents", "scope-review.md")
ROW = re.compile(r"^\|\s*(S\w+)\s", re.MULTILINE)
SCOPE = re.compile(r"(?<![A-Za-z0-9_])SCOPE=(\S+)")
FOLLOWUP = re.compile(r"(?<![A-Za-z0-9_])FOLLOWUP=(\S+)")
WORKTREE = re.compile(r"^WORKTREE:\s*(\S+)", re.MULTILINE)
NEEDS = ("a builder needs SCOPE=<path to scope-<bead>.md> whose first line starts PROCEED or RESHAPE, with "
         "scope-review-<bead>.json beside it holding verdict PASS and every rubric row PASS; a review-fix round on a "
         "branch launched with a passed scope carries FOLLOWUP=<branch> instead")


def review_path(scope):
    """scope-review-<bead>.json beside scope-<bead>.md, or None when the scope report is not named that way."""
    found = re.fullmatch(r"scope-(.+)\.md", os.path.basename(scope))
    return os.path.join(os.path.dirname(scope), "scope-review-" + found.group(1) + ".json") if found else None


def rubric_rows():
    """The row ids of the rubric table in scope-review.md (its `| S<id> ...` rows), in order: the one definition."""
    try:
        with open(RUBRIC, encoding="utf-8") as handle:
            return ROW.findall(handle.read())
    except OSError:
        return []


def scope_review_problem(review):
    """Why review is not a valid scope review (the rubric rows in order, each PASS or FAIL with a reason, and a verdict
    that is PASS exactly when every row passes), or None."""
    rubric = rubric_rows()
    if not rubric:
        return "cannot be judged: no rubric rows in " + os.path.normpath(RUBRIC)
    if not isinstance(review, dict) or not isinstance(review.get("rows"), list):
        return "holds no rows list"
    rows = review["rows"]
    if [r.get("id") if isinstance(r, dict) else None for r in rows] != rubric:
        return "must hold the rows " + ", ".join(rubric) + " of " + os.path.normpath(RUBRIC) + " in order, each once"
    for row in rows:
        if row.get("result") not in ("PASS", "FAIL"):
            return "row " + row["id"] + " has result " + repr(row.get("result")) + ", not PASS or FAIL"
        if not isinstance(row.get("reason"), str) or not row["reason"].strip():
            return "row " + row["id"] + " gives no reason"
    expected = "PASS" if all(r["result"] == "PASS" for r in rows) else "FAIL"
    if review.get("verdict") != expected:
        return "has verdict " + repr(review.get("verdict")) + " where its rows give " + expected
    return None


def load(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle), None
    except OSError as error:
        return None, path + " cannot be read (" + str(error.strerror) + ")"
    except ValueError:
        return None, path + " is not JSON"


def scope_problem(scope):
    try:
        with open(scope, encoding="utf-8") as handle:
            first = next((line for line in handle if line.strip()), "")
    except OSError as error:
        return "the scope report " + scope + " cannot be read (" + str(error.strerror) + ")"
    if not first.startswith(("PROCEED", "RESHAPE")):
        return "the scope report " + scope + " does not start with PROCEED or RESHAPE"
    path = review_path(scope)
    if path is None:
        return "the scope report " + scope + " is not named scope-<bead>.md"
    review, problem = load(path)
    if problem:
        return problem
    problem = scope_review_problem(review)
    if problem:
        return path + " " + problem
    if review["verdict"] != "PASS":
        return path + " has verdict FAIL"
    return None


def scoped_branches():
    try:
        with open(SCOPED, encoding="utf-8") as handle:
            return set(handle.read().split())
    except OSError:
        return set()


def worktree_branch(prompt):
    found = WORKTREE.search(prompt)
    if not found:
        return None
    result = subprocess.run(["git", "-C", found.group(1), "branch", "--show-current"], capture_output=True,
                            text=True, check=False)
    return result.stdout.strip() or None


def builder_problem(prompt):
    scope = SCOPE.search(prompt)
    if scope:
        found = scope_problem(scope.group(1))
        if found:
            return found + "; " + NEEDS
        branch = worktree_branch(prompt)
        if branch:
            os.makedirs(STATE, exist_ok=True)
            with open(SCOPED, "a", encoding="utf-8") as handle:
                handle.write(branch + "\n")
        return None
    followup = FOLLOWUP.search(prompt)
    if followup and followup.group(1) in scoped_branches():
        return None
    if followup:
        return "no builder was launched on " + followup.group(1) + " with a passed scope; " + NEEDS
    return NEEDS


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
    tool_input = data.get("tool_input") or {}
    if tool_input.get("subagent_type") == "builder":
        found = builder_problem(str(tool_input.get("prompt") or ""))
        if found:
            print("Blocked: " + found, file=sys.stderr)
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
