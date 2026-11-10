"""PreToolUse hook for Agent: refuses a subagent spawning further subagents, and a builder launched without a scope
report the scope review passed. scope_review_problem is the one validator of scope-review-<bead>.json; the
SubagentStop hook loads it by path."""

import hashlib
import json
import os
import re
import subprocess
import sys

MESSAGE = ("Blocked: a builder never spawns subagents; do the work serially in your own worktree, "
           "or reply BLOCKED: <one question>")
STATE = os.environ.get("CLAUDE_HOOKS_STATE", "/tmp/claude-hooks")  # every hook's state root; the stop hook reads it here
SCOPED = os.path.join(STATE, "scoped-branches")
RUBRIC = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".agents", "agents", "scope-review.md")
ROW = re.compile(r"^\|\s*(S\w+)\s", re.MULTILINE)
DEFERRAL = re.compile(r"For a DEFER or DROP report,[^.]*")
SCOPE = re.compile(r"(?<![A-Za-z0-9_])SCOPE=(\S+)")
FOLLOWUP = re.compile(r"(?<![A-Za-z0-9_])FOLLOWUP=(\S+)")
WORKTREE = re.compile(r"^WORKTREE:\s*(\S+)", re.MULTILINE)
BEAD = re.compile(r"^BEAD:\s*(\S+)", re.MULTILINE)
NEEDS = ("a builder needs SCOPE=<path to scope-<bead>.md>, a report naming the brief's BEAD whose first line starts "
         "PROCEED or RESHAPE, with scope-review-<bead>.json beside it holding the report's scope_sha256, verdict PASS "
         "and every rubric row PASS; a review-fix round carries FOLLOWUP=<branch> instead, from a WORKTREE on that "
         "branch, for the bead and branch an earlier builder launched with a passed scope")


def review_path(scope):
    """scope-review-<bead>.json beside scope-<bead>.md, or None when the scope report is not named that way."""
    found = re.fullmatch(r"scope-(.+)\.md", os.path.basename(scope))
    return os.path.join(os.path.dirname(scope), "scope-review-" + found.group(1) + ".json") if found else None


def rubric_rows(deferral=False):
    """The row ids scope-review.md grades, in order: every `| S<id> ...` row of its rubric table, or for a DEFER or
    DROP report the rows its "For a DEFER or DROP report, ..." sentence names. The rubric is the one definition."""
    try:
        with open(RUBRIC, encoding="utf-8") as handle:
            text = handle.read()
    except OSError:
        return []
    rows = ROW.findall(text)
    if not deferral:
        return rows
    sentence = DEFERRAL.search(text)
    named = set(re.findall(r"\bS\w+", sentence.group(0))) if sentence else set()
    return [row for row in rows if row in named]


def scope_verdict(scope):
    """The first word of the scope report's first non-blank line (PROCEED, RESHAPE, DEFER, DROP), or None."""
    try:
        with open(scope, encoding="utf-8") as handle:
            first = next((line for line in handle if line.strip()), "")
    except OSError:
        return None
    found = re.match(r"[A-Z]+", first)
    return found.group(0) if found else None


def scope_review_problem(review, scope):
    """Why review is not a valid scope review of the report at scope, or None: the rows the rubric grades for that
    report's verdict, in order, each PASS or FAIL with a reason (a row carried forward from an earlier review counts
    as written), and a verdict that is PASS exactly when every row passes."""
    rubric = rubric_rows(deferral=scope_verdict(scope) in ("DEFER", "DROP"))
    if not rubric:
        return "cannot be judged: no rubric rows in " + os.path.normpath(RUBRIC)
    if not isinstance(review, dict) or not isinstance(review.get("rows"), list):
        return "holds no rows list"
    digest = scope_sha256(scope)
    if review.get("scope_sha256") != digest:
        return ("has scope_sha256 " + repr(review.get("scope_sha256")) + ", not the sha256 of " + scope + " as it is now ("
                + str(digest) + "): the report changed after its review, or the review graded another report")
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


def scope_sha256(scope):
    """The sha256 of the scope report's bytes, or None when it cannot be read."""
    try:
        with open(scope, "rb") as handle:
            return hashlib.sha256(handle.read()).hexdigest()
    except OSError:
        return None


def load(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return json.load(handle), None
    except OSError as error:
        return None, path + " cannot be read (" + str(error.strerror) + ")"
    except ValueError:
        return None, path + " is not JSON"


def names(text, bead):
    return re.search(r"(?<![\w.-])" + re.escape(bead) + r"(?![\w-]|\.\w)", text) is not None


def scope_problem(scope, bead):
    if scope_verdict(scope) not in ("PROCEED", "RESHAPE"):
        return "the scope report " + scope + " does not start with PROCEED or RESHAPE"
    path = review_path(scope)
    if path is None:
        return "the scope report " + scope + " is not named scope-<bead>.md"
    if not bead:
        return "the brief names no BEAD: <id>"
    with open(scope, encoding="utf-8", errors="replace") as handle:
        if not names(handle.read(), bead):
            return "the scope report " + scope + " does not name the brief's bead " + bead
    if not re.search(r"(?:^|-)" + re.escape(bead) + r"(?:-|\.json$)", os.path.basename(path)):
        return "the review " + path + " is not named for the brief's bead " + bead
    review, problem = load(path)
    if problem:
        return problem
    problem = scope_review_problem(review, scope)
    if problem:
        return path + " " + problem
    if review["verdict"] != "PASS":
        return path + " has verdict FAIL"
    return None


def scoped_branches():
    """The (branch, bead) pairs builders were launched on with a passed scope."""
    try:
        with open(SCOPED, encoding="utf-8") as handle:
            return {tuple(line.split()) for line in handle if len(line.split()) == 2}
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
    bead = BEAD.search(prompt)
    bead = bead.group(1) if bead else None
    scope = SCOPE.search(prompt)
    if scope:
        found = scope_problem(scope.group(1), bead)
        if found:
            return found + "; " + NEEDS
        branch = worktree_branch(prompt)
        if branch:
            os.makedirs(STATE, exist_ok=True)
            with open(SCOPED, "a", encoding="utf-8") as handle:
                handle.write(branch + " " + bead + "\n")
        return None
    followup = FOLLOWUP.search(prompt)
    if not followup:
        return NEEDS
    branch = worktree_branch(prompt)
    if branch != followup.group(1):
        return ("FOLLOWUP=" + followup.group(1) + " but the WORKTREE is on " + repr(branch) + "; " + NEEDS)
    if (branch, bead) not in scoped_branches():
        return "no builder was launched on " + branch + " for " + str(bead) + " with a passed scope; " + NEEDS
    return None


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
