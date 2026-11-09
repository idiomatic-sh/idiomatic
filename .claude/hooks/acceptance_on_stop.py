"""SubagentStop hook: a builder may stop only on a passing acceptance script (DONE) or one question (BLOCKED); a review
agent may stop only once its verdict file is written into the INTEGRATION_TREE its prompt names (or on BLOCKED)."""

import json
import os
import re
import subprocess
import sys
import textwrap
from datetime import datetime

LIMIT = 5
STATE = os.environ.get("CLAUDE_HOOKS_STATE", "/tmp/claude-hooks")
BD_HOME = "/workspaces/idiomatic"
EVIDENCE = "crates/ryo/docs/contracts/evidence/"
# The verdict files each review agent writes (agent name -> evidence paths, a trailing / meaning any .json inside).
REVIEWERS = {"rust-review": ["checkpoint/rust-review.json"], "contract-review": ["checkpoint/contract-review.json"],
             "leak-review": ["checkpoint/leak-review.json", "branches/"], "tm-steward": ["checkpoint/tm-steward.json"]}


def text_of(message):
    content = message.get("content")
    if isinstance(content, str):
        return content
    parts = [b.get("text", "") for b in content or [] if isinstance(b, dict) and b.get("type") == "text"]
    return "\n".join(parts)


def read_transcript(path):
    first_user, last_assistant = "", ""
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            try:
                row = json.loads(line)
            except ValueError:
                continue
            message = row.get("message") if isinstance(row.get("message"), dict) else row
            role = message.get("role") or row.get("type")
            text = text_of(message).strip()
            if role == "user" and not first_user and text:
                first_user = text
            elif role == "assistant" and text:
                last_assistant = text
    return first_user, last_assistant


def started_at(path):
    """The time of the transcript's first timestamped row, as a POSIX time, or None."""
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            try:
                stamp = json.loads(line).get("timestamp")
                return datetime.fromisoformat(stamp.replace("Z", "+00:00")).timestamp()
            except (ValueError, AttributeError, TypeError):
                continue
    return None


def verdict_files(tree, agent):
    found = []
    for rel in REVIEWERS[agent]:
        path = os.path.join(tree, EVIDENCE, rel)
        if rel.endswith("/"):
            found += [os.path.join(path, n) for n in sorted(os.listdir(path)) if n.endswith(".json")] \
                if os.path.isdir(path) else []
        else:
            found.append(path)
    return found


def written_verdict(path, agent, since):
    """True when path holds this agent's verdict, written during this run."""
    try:
        if since is not None and os.path.getmtime(path) < since:
            return False
        with open(path, encoding="utf-8") as handle:
            review = json.load(handle)
    except (OSError, ValueError):
        return False
    return isinstance(review, dict) and review.get("reviewer") == agent and isinstance(review.get("verdict"), str) \
        and bool(review["verdict"])


def reviewer_stop(data, agent):
    """A review agent stops only after writing its verdict file into the tree its prompt names as INTEGRATION_TREE."""
    path = data.get("agent_transcript_path") or data.get("transcript_path")
    if not path or not os.path.exists(path):
        return 0
    first, last = read_transcript(path)
    last = data.get("last_assistant_message") or last
    tree = re.search(r"INTEGRATION_TREE\W{0,3}\s*(/[^\s`'\"]+)", first)
    if not tree or last.strip().startswith("BLOCKED:"):
        return 0
    tree = tree.group(1).rstrip(".,;")
    if any(written_verdict(f, agent, started_at(path)) for f in verdict_files(tree, agent)):
        return success("review-" + agent)
    wanted = " or ".join(os.path.join(tree, EVIDENCE, r) + ("<branch>.json" if r.endswith("/") else "")
                         for r in REVIEWERS[agent])
    return failure("review-" + agent, "Review not finished: write the verdict file " + wanted + " (reviewer " +
                   repr(agent) + ", a verdict) the moment the verdict is decided, then the write-up; or reply "
                   "BLOCKED: <one question>.")


def acceptance_script(bead):
    shown = subprocess.run(["bd", "show", bead, "--json"], cwd=BD_HOME if os.path.isdir(BD_HOME) else None,
                           capture_output=True, text=True, check=False)
    try:
        records = json.loads(shown.stdout)
    except ValueError:
        return None
    record = records[0] if isinstance(records, list) and records else records
    lines = str((record or {}).get("notes") or "").splitlines()
    start = next((i for i, l in enumerate(lines) if re.fullmatch(r"\s*ACCEPTANCE:\s*", l)), None)
    if start is None:
        return None
    body = []
    for line in lines[start + 1:]:
        if re.fullmatch(r"\s*END ACCEPTANCE\s*", line):
            return textwrap.dedent("\n".join(body))
        body.append(line)
    return None


def run(script, worktree):
    try:
        done = subprocess.run(["bash", "-e", "-c", script], cwd=worktree, capture_output=True, text=True,
                              timeout=600, check=False)
    except subprocess.TimeoutExpired:
        return False, "timed out after 600 s"
    return done.returncode == 0, done.stdout + done.stderr


def failure(bead, detail):
    os.makedirs(STATE, exist_ok=True)
    counter = os.path.join(STATE, bead + ".count")
    try:
        with open(counter, encoding="utf-8") as handle:
            count = int(handle.read().strip() or 0) + 1
    except (OSError, ValueError):
        count = 1
    if count >= LIMIT:
        os.remove(counter)
        print("Acceptance failed %d times; stop allowed; escalate" % LIMIT, file=sys.stderr)
        return 0
    with open(counter, "w", encoding="utf-8") as handle:
        handle.write(str(count))
    print(detail, file=sys.stderr)
    return 2


def success(bead):
    try:
        os.remove(os.path.join(STATE, bead + ".count"))
    except OSError:
        pass
    return 0


def main():
    data = json.load(sys.stdin)
    if data.get("agent_type") in REVIEWERS:
        return reviewer_stop(data, data["agent_type"])
    path = data.get("transcript_path")
    if not path or not os.path.exists(path):
        return 0
    first, last = read_transcript(path)
    bead = re.search(r"BEAD:\s*(ryo-[\w.]+)", first)
    tree = re.search(r"WORKTREE:\s*(/\S+)", first)
    if not bead or not tree:
        return 0
    bead, tree = bead.group(1), tree.group(1)
    if last.startswith("BLOCKED:"):
        return success(bead)
    if not last.startswith("DONE:"):
        return failure(bead, "Report must start with DONE: or BLOCKED:")
    script = acceptance_script(bead)
    if script is None:
        return failure(bead, "Acceptance failed for %s: no ACCEPTANCE: ... END ACCEPTANCE block in the bead notes. "
                       "Reply BLOCKED: <one question>." % bead)
    passed, output = run(script, tree)
    if passed:
        return success(bead)
    tail = "\n".join(output.splitlines()[-40:])
    return failure(bead, "Acceptance failed for %s: %s. Fix and run it again, or reply BLOCKED: <one question>."
                   % (bead, tail))


sys.exit(main())
