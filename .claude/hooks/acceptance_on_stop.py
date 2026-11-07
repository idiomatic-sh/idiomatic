"""SubagentStop hook: a builder may stop only on a passing acceptance script (DONE) or one question (BLOCKED)."""

import json
import os
import re
import subprocess
import sys
import textwrap

LIMIT = 5
STATE = os.environ.get("CLAUDE_HOOKS_STATE", "/tmp/claude-hooks")
BD_HOME = "/workspaces/idiomatic"


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
