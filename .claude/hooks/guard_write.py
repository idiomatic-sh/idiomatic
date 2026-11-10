"""PreToolUse hook for Write, Edit, MultiEdit and NotebookEdit.

Review evidence is written only into a claude/checkpoint-N worktree (guard_shell.evidence_write_problem). Tastemaker
manifests are closed for extension: the edit is diffed as TOML facts and any addition or loosening removal is refused,
an added in-source `ignore: idiomatic.` line is refused anywhere, and the owner's allowlist (closed_for_extension.toml
beside this hook, edited by hand in idiomatic-internal) admits named changes. Writes to the allowlist are refused.
It fails closed on its own errors for evidence-named and closed paths and ignore-bearing text, and lets every other
path through.
"""

import collections
import importlib.util
import json
import os
import re
import subprocess
import sys
import tomllib

NAMED = re.compile(r"(rust-review|contract-review|leak-review|tm-steward)[^/]*\.json$|asymptote\.json$|evidence/branches/")
# A plain-text copy of guard_shell's CLOSED_FILES and IGNORE_MARK, used only when guard_shell fails to load: the
# fallback must not depend on the module whose failure it handles (as NAMED copies the evidence names).
FALLBACK = re.compile(r"(tastemaker|closed_for_extension)\.toml|ignore:\s*idiomatic\.")
ENTRY_ARRAYS = ("templates", "rules", "specs")
ENTRY = "<entry>"
TIGHTENING_LISTS = ("exclude", "except", "allow", "rules-exclude")
SCRIPT_SUFFIXES = (".py", ".pl", ".rb", ".js")
SHELL_SUFFIXES = (".sh", ".bash", ".zsh")
SHELL_SHEBANG = re.compile(r"#!\S*/(env\s+)?(ba|z|da|k)?sh\b")


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


def canonical(value):
    return json.dumps(value, sort_keys=True, default=str)


def facts(doc):
    """Counter of (scope, key path, canonical value, list element) over every leaf of a manifest, plus one entry
    fact per templates/rules/specs entry; list positions are erased, lists of tables walked into."""
    found = collections.Counter()

    def walk(scope, path, value, listed):
        if isinstance(value, dict):
            for key, inner in value.items():
                walk(scope, path + (key,), inner, False)
        elif isinstance(value, list):
            for item in value:
                if isinstance(item, dict):
                    walk(scope, path, item, False)
                else:
                    found[(scope, path, canonical(item), True)] += 1
        else:
            found[(scope, path, canonical(value), listed)] += 1

    for key, value in doc.items():
        if key in ENTRY_ARRAYS and isinstance(value, list):
            for entry in value:
                if not isinstance(entry, dict):
                    walk(key, (), entry, True)
                    continue
                name = str(entry.get(key[:-1]))
                alias = entry.get("alias")
                scope = key + "[" + name + (" as " + str(alias) if alias is not None else "") + "]"
                found[(scope, (), canonical(ENTRY), False)] += 1
                walk(scope, (), entry, False)
        else:
            walk("", (key,), value, False)
    return found


def fact_name(scope, path):
    return ".".join(([scope] if scope else []) + list(path))


def exempt(scope, path):
    """A description, or the top-level tastemaker version: free to change either way."""
    return (bool(path) and path[-1] == "description") or (scope == "" and path == ("tastemaker",))


def tightening(scope, path, value, listed):
    """A removal that narrows what passes: an exclude/except/allow/rules-exclude element outside params, or a
    single-file template placement (its file, template, alias and entry)."""
    if exempt(scope, path):
        return True
    if listed and path and path[-1] in TIGHTENING_LISTS and "params" not in path:
        return True
    if scope.startswith("templates["):
        if path in ((), ("template",), ("alias",)):
            return True
        if path == ("file",):
            raw = json.loads(value)
            return isinstance(raw, str) and not re.search(r"[*?\[{]", raw)
    return False


def manifest_changes(before_doc, after_doc):
    """("added"|"removed", fact, value) for every refused change, in order."""
    before, after = facts(before_doc), facts(after_doc)
    refused = []
    for (scope, path, value, listed), count in (after - before).items():
        if not exempt(scope, path):
            refused += [("added", fact_name(scope, path), json.loads(value))] * count
    for (scope, path, value, listed), count in (before - after).items():
        if not tightening(scope, path, value, listed):
            refused += [("removed", fact_name(scope, path), json.loads(value))] * count
    return refused


def ignore_additions(shell, before, after):
    """The stripped lines carrying an ignore mark that the after text has more of than the before text."""
    def marked(text):
        return collections.Counter(line.strip() for line in text.split("\n") if shell.IGNORE_MARK.search(line))
    added = marked(after) - marked(before)
    return [line for line, count in added.items() for _ in range(count)]


def content_problem(shell, path, text):
    """A diff naming a closed file, or a script that names one and writes a file: a shell script by any shell
    writer (guard_shell's WRITER), any other script by an interpreter's write."""
    names = any(name in text for name in shell.CLOSED_FILES)
    if names and any(line.startswith(("+++ ", "--- ")) and any(n in line for n in shell.CLOSED_FILES)
                     for line in text.split("\n")):
        return path + " is a patch touching a closed file; edit the manifest itself with the Edit tool"
    if names and (path.endswith(SHELL_SUFFIXES) or SHELL_SHEBANG.match(text)):
        writes = shell.WRITER.search(text)
    else:
        writes = (path.endswith(SCRIPT_SUFFIXES) or text.startswith("#!")) and shell.PY_WRITE.search(text)
    if names and writes:
        return path + " is a script that writes a closed file; edit the manifest itself with the Edit tool"
    return None


def allowlist(shell, hooks_dir):
    """The owner's allowlist; absent or unparsable admits nothing."""
    try:
        with open(os.path.join(hooks_dir, shell.ALLOWLIST), "rb") as handle:
            doc = tomllib.load(handle)
    except (OSError, ValueError):
        return {}
    return doc if isinstance(doc, dict) else {}


def same_path(full, path):
    return isinstance(path, str) and path != "" and (full == path or full.endswith("/" + path.lstrip("/")))


def complete(entry, keys):
    return isinstance(entry, dict) and all(k in entry for k in keys) and \
        isinstance(entry.get("reason"), str) and entry["reason"].strip() != ""


def admitted_change(entries, full, change):
    kind, fact, value = change
    return any(complete(e, ("path", "change", "fact", "value", "reason")) and same_path(full, e["path"]) and
               e["change"] == kind and e["fact"] == fact and canonical(e["value"]) == canonical(value)
               for e in entries if isinstance(e, dict))


def admitted_line(entries, full, line):
    return any(complete(e, ("path", "line", "reason")) and same_path(full, e["path"]) and
               isinstance(e["line"], str) and e["line"].strip() == line for e in entries if isinstance(e, dict))


def toml_value(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(v) for v in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(json.dumps(k) + " = " + toml_value(v) for k, v in value.items()) + " }"
    if isinstance(value, str):
        return json.dumps(value)
    return value.isoformat() if hasattr(value, "isoformat") else json.dumps(str(value))


def shown_path(full):
    """The path as the allowlist names it: relative to the git top level, else absolute."""
    done = subprocess.run(["git", "-C", os.path.dirname(full) or "/", "rev-parse", "--show-toplevel"],
                          capture_output=True, text=True, check=False)
    top = done.stdout.strip() if done.returncode == 0 else ""
    return os.path.relpath(full, top) if top and full.startswith(top + "/") else full


def refusal(full, what, blocks):
    return (full + ": " + what + ". The owner admits a change by committing its block, with a reason, to " +
            "closed_for_extension.toml" + " beside the hooks in idiomatic-internal:\n\n" + "\n\n".join(blocks))


def manifest_problem(full, before, after, entries):
    try:
        after_doc = tomllib.loads(after)
    except ValueError as error:
        return full + " would not parse as TOML (" + str(error) + "); a manifest edit must leave it valid"
    try:
        before_doc = tomllib.loads(before)
    except ValueError:
        before_doc = {}
    left = [c for c in manifest_changes(before_doc, after_doc) if not admitted_change(entries, full, c)]
    if not left:
        return None
    where = shown_path(full)
    blocks = ["[[manifest]]\npath = " + json.dumps(where) + "\nchange = " + json.dumps(kind) + "\nfact = " +
              json.dumps(fact) + "\nvalue = " + toml_value(value) + '\nreason = ""' for kind, fact, value in left]
    return refusal(full, "a Tastemaker manifest is closed for extension: an agent may remove an exclude, except, "
                   "allow or rules-exclude element or a single-file template placement, and edit descriptions, "
                   "comments and order; every addition and every other removal is the owner's. Refused: " +
                   "; ".join(kind + " " + fact + " = " + toml_value(value) for kind, fact, value in left), blocks)


def ignore_problem(shell, full, before, after, entries):
    left = [line for line in ignore_additions(shell, before, after) if not admitted_line(entries, full, line)]
    if not left:
        return None
    where = shown_path(full)
    blocks = ["[[ignore]]\npath = " + json.dumps(where) + "\nline = " + json.dumps(line) + '\nreason = ""'
              for line in left]
    return refusal(full, "an added in-source ignore comment is the owner's (" + "; ".join(left) + "); fix the "
                   "finding, or fix the rule with a negative fixture", blocks)


def closed_problem(shell, tool, given, full, path):
    """The closed-for-extension refusal for this tool call, or None."""
    name = os.path.basename(full)
    if name == shell.ALLOWLIST:
        return shell.closed_write_problem(path)
    try:
        with open(full, encoding="utf-8") as handle:
            before = handle.read()
    except (OSError, ValueError):
        before = ""
    after = after_text(tool, given, before)
    if after is None:
        if name in shell.CLOSED_FILES or any(shell.IGNORE_MARK.search(t) for t in new_texts(given)):
            return full + ": old_string not found; the hook cannot diff this edit. Read the file and match its text"
        return None
    entries = allowlist(shell, os.path.dirname(os.path.abspath(__file__)))
    lines = entries.get("ignore") if isinstance(entries.get("ignore"), list) else []
    if name in shell.CLOSED_FILES:
        changes = entries.get("manifest") if isinstance(entries.get("manifest"), list) else []
        return manifest_problem(full, before, after, changes) or ignore_problem(shell, full, before, after, lines)
    return ignore_problem(shell, full, before, after, lines) or content_problem(shell, full, after)


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
        here = data.get("cwd") or os.getcwd()
        full = os.path.normpath(os.path.join(here, os.path.expanduser(path)))
        found = closed_problem(module, data.get("tool_name"), given, full, path) or \
            module.evidence_write_problem(path, here)
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
