"""SubagentStop hook: a builder may stop only on a passing acceptance script (DONE) or one question (BLOCKED); a review
agent may stop only once the verdict file its prompt names (INTEGRATION_TREE=, SOURCE=, and BRANCH= for a branch
review) is written during its run and valid by the one review evidence validator, or on BLOCKED."""

import json
import os
import re
import subprocess
import sys
import textwrap
from datetime import datetime

LIMIT = 5
BD_HOME = "/workspaces/idiomatic"
EVIDENCE = "crates/ryo/docs/contracts/evidence/"
# The verdict file of each review agent, under the evidence directory; a leak review of a branch writes branches/<branch>.json.
REVIEWERS = {"rust-review": "checkpoint/rust-review.json", "contract-review": "checkpoint/contract-review.json",
             "leak-review": "checkpoint/leak-review.json", "tm-steward": "checkpoint/tm-steward.json"}
TRAILING = ".,;:)]}>'\"`"

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


def token(text, name):
    """The value of the `NAME=<value>` token in text, trailing punctuation stripped; None when it is absent or
    appears with two different values (a prompt quoting an earlier review's token is ambiguous, so it holds)."""
    found = {v.rstrip(TRAILING) for v in re.findall(r"(?<![A-Za-z0-9_])" + name + r"=(\S+)", text)}
    return found.pop() if len(found) == 1 else None


def shape_problem(review, agent):
    """The problem with review against the one validator of review evidence, shape() in
    ci/contract_review/service.py (loaded by path, as guard_shell.py does), or None."""
    import importlib.util
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "ci", "contract_review", "service.py")
    try:
        spec = importlib.util.spec_from_file_location("contract_review_service", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.shape(review, agent)
    except ValueError as error:
        return str(error)
    except Exception as error:  # a missing or old service.py holds the stop, never releases it
        return "cannot be validated (" + type(error).__name__ + ": " + str(error) + ")"
    return None


def guard_module():
    """guard_shell.py beside this hook, loaded by path: the push gate's own definitions (stands_for)."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("guard_shell", os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                                                             "guard_shell.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def verdict_problem(path, agent, source, since, tree):
    """Why path is not this run's verdict for source, or None. A TM steward's verdict, like the push gate's
    previous_reviews, may name SOURCE (the landed head it decides) or an ancestor with only evidence changed since
    (guard_shell.stands_for, the one definition)."""
    try:
        if os.path.getmtime(path) < since:
            return path + " was not written during this run"
        with open(path, encoding="utf-8") as handle:
            review = json.load(handle)
    except OSError:
        return path + " does not exist"
    except ValueError:
        return path + " is not JSON"
    problem = shape_problem(review, agent)
    if problem:
        return path + " " + problem
    if agent == "tm-steward":
        try:
            guard = guard_module()
            stands = guard.stands_for(tree, review["source"], source, guard.EVIDENCE)
        except Exception as error:  # a missing or broken guard_shell.py holds the stop, never releases it
            return path + " cannot be judged: guard_shell.py failed (" + type(error).__name__ + ": " + str(error) + ")"
        if not stands:
            return path + " decides " + repr(review["source"]) + ", which does not stand for SOURCE=" + source + \
                " (the landed head, or an ancestor with only evidence changed since)"
    elif review.get("source") != source:
        return path + " reviews " + repr(review.get("source")) + ", not SOURCE=" + source
    return None


def reviewer_stop(data, agent):
    """A review agent stops only after writing the verdict file its prompt names, or on BLOCKED: <question>."""
    path = data.get("agent_transcript_path") or data.get("transcript_path")
    if not path or not os.path.exists(path):
        return hold("its transcript is unreadable, so the run's start is unknown")
    first, last = read_transcript(path)
    last = data.get("last_assistant_message") or last
    if last.strip().startswith("BLOCKED:"):
        return 0
    tree, source, branch = token(first, "INTEGRATION_TREE"), token(first, "SOURCE"), token(first, "BRANCH")
    if "BRANCH=" in first and branch is None:
        return hold("the prompt names BRANCH= with two different values")
    if not tree or not source or not re.fullmatch(r"[0-9a-f]{40}", source):
        return hold("the prompt must name INTEGRATION_TREE=<path> and SOURCE=<40-hex sha> (and BRANCH=<name> for a "
                    "branch review), each once or with one value")
    since = started_at(path)
    if since is None:
        return hold("its transcript has no timestamp, so the run's start is unknown")
    rel = "branches/" + branch.replace("/", "-") + ".json" if agent == "leak-review" and branch else REVIEWERS[agent]
    problem = verdict_problem(os.path.join(tree, EVIDENCE, rel), agent, source, since, tree)
    return hold(problem) if problem else 0


def gate_module():
    """no_subagent_builders.py beside this hook, loaded by path: the one validator of scope-review-<report stem>.json."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("no_subagent_builders",
                                                  os.path.join(os.path.dirname(os.path.abspath(__file__)),
                                                               "no_subagent_builders.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def scope_review_stop(data):
    """A scope review stops only after writing a valid scope-review-<report stem>.json beside the SCOPE= report its prompt
    names, during its run, or on BLOCKED: <question>. Any error on the way holds the stop."""
    try:
        return scope_review_check(data)
    except Exception as error:
        return scope_hold("the check failed (" + type(error).__name__ + ": " + str(error) + ")")


def scope_review_check(data):
    path = data.get("agent_transcript_path") or data.get("transcript_path")
    if not path or not os.path.exists(path):
        return scope_hold("its transcript is unreadable, so the run's start is unknown")
    first, last = read_transcript(path)
    last = data.get("last_assistant_message") or last
    if last.strip().startswith("BLOCKED:"):
        return 0
    since = started_at(path)
    if since is None:
        return scope_hold("its transcript has no timestamp, so the run's start is unknown")
    try:
        gate = gate_module()
    except Exception as error:  # a missing or broken gate holds the stop, never releases it
        return scope_hold("no_subagent_builders.py failed to load (" + type(error).__name__ + ": " + str(error) + ")")
    scope = gate.brief_scope(first)
    review_path = gate.review_path(scope) if scope else None
    if review_path is None:
        return scope_hold("the prompt must name SCOPE=<path to scope-<stem>.md> once")
    try:
        if os.path.getmtime(review_path) < since:
            return scope_hold(review_path + " was not written during this run")
    except OSError:
        return scope_hold(review_path + " does not exist")
    review, problem = gate.load(review_path)
    if problem:
        return scope_hold(problem)
    problem = gate.scope_review_problem(review, scope)
    return scope_hold(review_path + " " + problem) if problem else 0


def scope_hold(why):
    print("Scope review not finished: " + why + ". Append the Scope review section to the scope report, then write "
          "scope-review-<report stem>.json beside it: {\"rows\": [{\"id\", \"result\": PASS|FAIL, \"reason\"}] for every "
          "rubric row (S1 and S10 only for a DEFER or DROP report), \"verdict\", and \"scope_sha256\", the sha256 of "
          "the report as written; or reply BLOCKED: <one question>.", file=sys.stderr)
    return 2


def hold(why):
    print("Review not finished: " + why + ". Write the verdict file (the review evidence schema, reviewer and source "
          "set) the moment the verdict is decided, then the write-up; or reply BLOCKED: <one question>.", file=sys.stderr)
    return 2


def gate_assignment(name):
    """The value node of no_subagent_builders.py's top-level `name = ...`, read from its source without running it
    (a broken gate cannot release a stop through this path); None when it cannot be read."""
    import ast
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "no_subagent_builders.py")
    try:
        with open(path, encoding="utf-8") as handle:
            tree = ast.parse(handle.read())
    except (OSError, SyntaxError, ValueError):
        return None
    for node in tree.body:
        if isinstance(node, ast.Assign) and [getattr(t, "id", None) for t in node.targets] == [name]:
            return node.value
    return None


def state_root():
    """The hooks' state root as no_subagent_builders.py defines it (its STATE default, overridden by
    CLAUDE_HOOKS_STATE); None when that definition cannot be read."""
    import ast
    call = gate_assignment("STATE")
    if (isinstance(call, ast.Call) and len(call.args) == 2 and
            all(isinstance(a, ast.Constant) and isinstance(a.value, str) for a in call.args)):
        return os.environ.get(call.args[0].value, call.args[1].value)
    return None


def accepted_types():
    """The agent types whose stop runs acceptance, no_subagent_builders.py's ACCEPTED literal; None when unreadable."""
    import ast
    node = gate_assignment("ACCEPTED")
    try:
        value = ast.literal_eval(node) if node is not None else None
    except ValueError:
        return None
    return value if isinstance(value, set) and all(isinstance(v, str) for v in value) else None


def no_state():
    print("Stop held: the hooks' state root cannot be read from no_subagent_builders.py; repair it, then stop again.",
          file=sys.stderr)
    return 2


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
    state = state_root()
    if state is None:
        return no_state()
    os.makedirs(state, exist_ok=True)
    counter = os.path.join(state, bead + ".count")
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
    state = state_root()
    if state is None:
        return no_state()
    try:
        os.remove(os.path.join(state, bead + ".count"))
    except OSError:
        pass
    return 0


def main():
    try:
        return stop(json.load(sys.stdin))
    except Exception as error:  # Claude Code releases a stop whose hook crashes; hold it instead
        print("Stop held: acceptance_on_stop.py failed (" + type(error).__name__ + ": " + str(error) + "); "
              "repair the brief or the hook, or reply BLOCKED: <one question>.", file=sys.stderr)
        return 2


def stop(data):
    if data.get("agent_type") == "scope-review":
        return scope_review_stop(data)
    if data.get("agent_type") in REVIEWERS:
        return reviewer_stop(data, data["agent_type"])
    accepted = accepted_types()
    if accepted is None:
        print("Stop held: no_subagent_builders.py's ACCEPTED cannot be read; repair it, then stop again.",
              file=sys.stderr)
        return 2
    if data.get("agent_type") not in accepted:
        return 0
    # On SubagentStop, transcript_path is the parent session's; the agent's own is agent_transcript_path.
    path = data.get("agent_transcript_path") or data.get("transcript_path")
    if not path or not os.path.exists(path):
        return 0
    first, last = read_transcript(path)
    last = (data.get("last_assistant_message") or last).strip()
    try:
        gate = gate_module()
    except Exception as error:  # the brief cannot be read without the gate's parser; hold, never release
        print("Stop held: no_subagent_builders.py failed to load (" + type(error).__name__ + ": " + str(error) +
              "); repair it, then stop again.", file=sys.stderr)
        return 2
    beads, tree = gate.brief_beads(first), gate.brief_worktree(first)
    if not beads or not tree:
        return 0
    if last.startswith("BLOCKED:"):
        return max(success(bead) for bead in beads)
    if not last.startswith("DONE:"):
        return failure(beads[0], "Report must start with DONE: or BLOCKED:")
    for bead in beads:  # every bead the brief carries passes its own acceptance block
        script = acceptance_script(bead)
        if script is None:
            return failure(bead, "Acceptance failed for %s: no ACCEPTANCE: ... END ACCEPTANCE block in the bead "
                           "notes. Reply BLOCKED: <one question>." % bead)
        passed, output = run(script, tree)
        if not passed:
            tail = "\n".join(output.splitlines()[-40:])
            return failure(bead, "Acceptance failed for %s: %s. Fix and run it again, or reply BLOCKED: <one "
                           "question>." % (bead, tail))
    return max(success(bead) for bead in beads)


sys.exit(main())
