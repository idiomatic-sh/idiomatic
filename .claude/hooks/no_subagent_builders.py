"""PreToolUse hook for Agent: refuses a subagent spawning further subagents, and a builder launched without a scope
report the scope review passed (and a contract-migration launch, which edits code, the same way). scope_review_problem is the one validator of scope-review-<report stem>.json; the
SubagentStop hook loads it by path."""

import hashlib
import json
import os
import re
import subprocess
import sys

MESSAGE = ("Blocked: a builder never spawns subagents; do the work serially in your own worktree, "
           "or reply BLOCKED: <one question>")
STATE = os.environ.get("CLAUDE_HOOKS_STATE", "/tmp/claude-hooks")  # every hook's state root; the stop hook parses it
SCOPED = os.path.join(STATE, "scoped-branches")
RUBRIC = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".agents", "agents", "scope-review.md")
ROW = re.compile(r"^\|\s*(S\w+)\s", re.MULTILINE)
DEFERRAL = re.compile(r"For a DEFER or DROP report,[^.]*")
FOLLOWUP = re.compile(r"(?<![A-Za-z0-9_])FOLLOWUP=(\S+)")
GATED = {"builder", "contract-migration"}  # the agents that edit code launch only on a passed scope
UNGATED = "integrator"  # carries BEAD:/WORKTREE: briefs to land work, not to author it
TRAILING = ".,;:)]}>'\"`"
NEEDS = ("a builder needs SCOPE=<path to scope-<stem>.md>, a report whose first line starts PROCEED or RESHAPE and "
         "whose second line is exactly `BEADS: <id> ...` listing every bead of the brief's BEAD: lines, with "
         "scope-review-<stem>.json beside it holding the report's scope_sha256, verdict PASS and every rubric row "
         "PASS; a review-fix round carries FOLLOWUP=<branch> instead, from a WORKTREE on that branch, for beads an "
         "earlier builder launched on that branch with a passed scope")


def brief_beads(prompt):
    """Every bead a brief names: each token of each `BEAD:` line, in order, once. The stop hook reads it here too."""
    beads = []
    for line in re.findall(r"^BEAD:[ \t]*(.*)$", prompt, re.MULTILINE):
        beads += [word for word in line.split() if word not in beads]
    return beads


def brief_worktree(prompt):
    """The path of the brief's `WORKTREE:` line, or None."""
    found = re.search(r"^WORKTREE:[ \t]*(\S+)", prompt, re.MULTILINE)
    return found.group(1) if found else None


def brief_scope(prompt):
    """The value of the brief's `SCOPE=<path>` token, trailing punctuation stripped; None when it is absent or given
    with two different values."""
    found = {v.rstrip(TRAILING) for v in re.findall(r"(?<![A-Za-z0-9_])SCOPE=(\S+)", prompt)}
    return found.pop() if len(found) == 1 else None


def review_path(scope):
    """scope-review-<stem>.json beside scope-<stem>.md, or None when the scope report is not named that way."""
    found = re.fullmatch(r"scope-(.+)\.md", os.path.basename(scope))
    return os.path.join(os.path.dirname(scope), "scope-review-" + found.group(1) + ".json") if found else None


def rubric_rows(deferral=False):
    """The row ids scope-review.md grades, in order: every `| S<id> ...` row of its rubric table, or for a DEFER or
    DROP report the rows its "For a DEFER or DROP report, ..." sentence names. The rubric is the one definition."""
    try:
        with open(RUBRIC, encoding="utf-8", errors="replace") as handle:
            text = handle.read()
    except OSError:
        return []
    rows = ROW.findall(text)
    if not deferral:
        return rows
    sentence = DEFERRAL.search(text)
    named = set(re.findall(r"\bS\w+", sentence.group(0))) if sentence else set()
    return [row for row in rows if row in named]


def scope_lines(scope):
    """The scope report's lines, or None when it cannot be read."""
    try:
        with open(scope, encoding="utf-8", errors="replace") as handle:
            return handle.read().splitlines()
    except OSError:
        return None


def scope_verdict(scope):
    """The first word of the scope report's first line (PROCEED, RESHAPE, DEFER, DROP), or None."""
    lines = scope_lines(scope)
    first = lines[0] if lines else ""
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
    if digest is None:
        return "cannot be judged: the scope report " + scope + " cannot be read"
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


def scope_problem(scope, beads):
    if scope_verdict(scope) not in ("PROCEED", "RESHAPE"):
        return "the scope report " + scope + " does not start with PROCEED or RESHAPE"
    path = review_path(scope)
    if path is None:
        return "the scope report " + scope + " is not named scope-<stem>.md"
    if not beads:
        return "the brief names no BEAD: <id>"
    lines = scope_lines(scope) or []
    second = lines[1] if len(lines) > 1 else ""
    listed = second[len("BEADS: "):].split(" ") if re.fullmatch(r"BEADS:( \S+)+", second) else []
    missing = [bead for bead in beads if bead not in listed]
    if missing:
        return ("line 2 of the scope report " + scope + " is not exactly `BEADS: <id> ...` listing the brief's beads "
                "(missing " + ", ".join(missing) + ")")
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
    worktree = brief_worktree(prompt)
    if not worktree:
        return None
    result = subprocess.run(["git", "-C", worktree, "branch", "--show-current"], capture_output=True,
                            text=True, check=False)
    return result.stdout.strip() or None


def builder_problem(prompt):
    beads = brief_beads(prompt)
    scope = brief_scope(prompt)
    if scope:
        found = scope_problem(scope, beads)
        if found:
            return found + "; " + NEEDS
        branch = worktree_branch(prompt)
        if branch:
            os.makedirs(STATE, exist_ok=True)
            with open(SCOPED, "a", encoding="utf-8") as handle:
                handle.write("".join(branch + " " + bead + "\n" for bead in beads))
        return None
    followup = FOLLOWUP.search(prompt)
    if not followup:
        return NEEDS
    branch = worktree_branch(prompt)
    if branch != followup.group(1):
        return ("FOLLOWUP=" + followup.group(1) + " but the WORKTREE is on " + repr(branch) + "; " + NEEDS)
    scoped = scoped_branches()
    missing = [bead for bead in beads if (branch, bead) not in scoped]
    if not beads or missing:
        return ("no builder was launched on " + branch + " for " + (", ".join(missing) or "a bead") +
                " with a passed scope; " + NEEDS)
    return None


def main():
    try:
        data = json.load(sys.stdin)
    except ValueError:
        return 0
    if not isinstance(data, dict) or data.get("tool_name") != "Agent":
        return 0
    if data.get("agent_id"):  # set only when a subagent makes the call; transcript_path is the session's either way
        print(MESSAGE, file=sys.stderr)
        return 2
    tool_input = data.get("tool_input") or {}
    kind, prompt = tool_input.get("subagent_type"), str(tool_input.get("prompt") or "")
    try:
        brief = kind != UNGATED and bool(brief_beads(prompt)) and brief_worktree(prompt) is not None
    except Exception:  # a parser that breaks gates the launch
        brief = True
    if kind in GATED or brief:  # a builder brief needs a passed scope whatever agent type carries it
        try:
            found = builder_problem(prompt)
        except Exception as error:  # a gate that breaks refuses, never lets the launch through
            found = "the scope gate failed (" + type(error).__name__ + ": " + str(error) + ")"
        if found:
            print("Blocked: " + found, file=sys.stderr)
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
