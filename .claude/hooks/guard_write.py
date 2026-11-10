"""PreToolUse hook for Write, Edit, MultiEdit and NotebookEdit.

Review evidence is written only into a claude/checkpoint-N worktree (guard_shell.evidence_write_problem). Tastemaker
manifests are closed for extension: the edit is diffed as TOML facts by closed_effect's model, and any addition or
loosening removal is refused, as is an added `ignore: idiomatic.` line in any file, unless the owner's allowlist
admits it; writes to the allowlist itself are refused. It fails closed on its own errors for evidence-named and closed
paths and ignore-bearing text, and lets every other path through.
"""

import importlib.util
import json
import os
import re
import subprocess
import sys

NAMED = re.compile(r"(rust-review|contract-review|leak-review|tm-steward)[^/]*\.json$|asymptote\.json$|evidence/branches/")
# A plain-text copy of closed_effect's CLOSED_FILES and IGNORE_MARK, used only when a module fails to load: the
# fallback must not depend on the module whose failure it handles (as NAMED copies the evidence names).
FALLBACK = re.compile(r"(tastemaker|closed_for_extension)\.toml|ignore:\s*idiomatic\.")
HOOKS = os.path.dirname(os.path.abspath(__file__))


def load(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HOOKS, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def new_texts(given):
    """Every text the tool call writes: content, new_string, each edit's new_string, new_source."""
    texts = [given.get("content"), given.get("new_string"), given.get("new_source")]
    texts += [e.get("new_string") for e in given.get("edits") or [] if isinstance(e, dict)]
    return [t for t in texts if isinstance(t, str)]


def replaced(text, old, new, every):
    """text with old replaced (every occurrence or the first), or None when old is absent."""
    if old == "":
        return new if text == "" else None
    if old not in text:
        return None
    return text.replace(old, new) if every else text.replace(old, new, 1)


def after_text(tool, given, before):
    """The file's text after the tool call, or None when an Edit's or MultiEdit's old_string is absent."""
    if tool == "Write" or "content" in given:
        return given.get("content") or ""
    if tool == "NotebookEdit" or "new_source" in given:
        return given.get("new_source") or ""
    edits = given.get("edits") if tool == "MultiEdit" or "edits" in given else [given]
    text = before
    for edit in edits or []:
        text = replaced(text, edit.get("old_string") or "", edit.get("new_string") or "", edit.get("replace_all"))
        if text is None:
            return None
    return text


def top_level(full):
    """The git top level holding full, or None."""
    place = os.path.dirname(full) or "/"
    while place != "/" and not os.path.isdir(place):
        place = os.path.dirname(place)
    done = subprocess.run(["git", "-C", place, "rev-parse", "--show-toplevel"], capture_output=True, text=True,
                          check=False)
    top = done.stdout.strip() if done.returncode == 0 else ""
    return top if top and full.startswith(top + "/") else None


def closed_problem(closed, tool, given, full):
    """The closed-for-extension refusal for this tool call, or None."""
    name = os.path.basename(full)
    if name == closed.ALLOWLIST:
        return closed.allowlist_problem(full)
    try:
        with open(full, encoding="utf-8") as handle:
            before = handle.read()
    except (OSError, ValueError):
        before = None
    after = after_text(tool, given, before or "")
    if after is None:
        if name in closed.CLOSED_FILES or any(closed.IGNORE_MARK.search(t) for t in new_texts(given)):
            return full + ": old_string not found; the hook cannot diff this edit. Read the file and match its text"
        return None
    if name != closed.MANIFEST and not closed.marks(after) - closed.marks(before or ""):
        return None
    # the text before, and the committed texts an edit may take back (HEAD, the other side of a merge in progress)
    top = top_level(full)
    shown = os.path.relpath(full, top) if top else full
    bases = [before] + (closed.committed_texts(top, shown) if top else [])
    listed = closed.allowlist()
    if name == closed.MANIFEST:
        found = closed.manifest_problem(full, shown, bases, after, closed.entries(listed, "manifest"))
        if found:
            return found
    return closed.ignore_problem(full, shown, [b or "" for b in bases], after, closed.entries(listed, "ignore"))


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
        shell, closed = load("guard_shell"), load("closed_effect")
        here = data.get("cwd") or os.getcwd()
        full = os.path.normpath(os.path.join(here, os.path.expanduser(path)))
        found = closed_problem(closed, data.get("tool_name"), given, full) or shell.evidence_write_problem(path, here)
    except Exception as error:
        if NAMED.search(path) or FALLBACK.search(path) or any(FALLBACK.search(t) for t in new_texts(given)):
            print("Blocked: guard failed closed: " + type(error).__name__ + ": " + str(error), file=sys.stderr)
            return 2
        return 0
    if found:
        print("Blocked: " + found, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
