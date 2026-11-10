"""Tastemaker manifests are closed for extension: the fact model both edit paths share, and the Bash effect check.

PreToolUse and PostToolUse hook for Bash. It judges what a command did, never how it was spelled. Before the
command it snapshots each repository the command may touch (the working directory's, the project's, and the one of
every absolute path its text names): the whole working tree, tracked and untracked files, as a git tree written
through a temporary index, plus HEAD and the owner's allowlist. After the command it snapshots again and diffs the
two trees. A changed tastemaker.toml is diffed as TOML facts (`manifest_changes`, the model guard_write uses for
the Write and Edit tools); an extension or loosening is reverted to its content before the command and refused. A
new `ignore: idiomatic.` line in any file is reverted the same way. Content that git brought from commits that
existed before the command (a checkout, merge, rebase or reset) is not the command's own writing and passes. The
owner's allowlist (closed_for_extension.toml beside this hook, edited by hand in idiomatic-internal) admits named
changes; any change to it is reverted and refused. A snapshot that cannot be read refuses, restoring nothing.
"""

import collections
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import time
import tomllib

MANIFEST = "tastemaker.toml"
ALLOWLIST = "closed_for_extension.toml"
CLOSED_FILES = (MANIFEST, ALLOWLIST)
IGNORE_MARK = re.compile(r"ignore:\s*idiomatic\.")
ENTRY_ARRAYS = ("templates", "rules", "specs")
ENTRY = "<entry>"
TIGHTENING_LISTS = ("exclude", "except", "allow", "rules-exclude")
HOOKS = os.path.dirname(os.path.abspath(__file__))
ABSOLUTE = re.compile(r"(?<![\w.~$-])/[^\s'\"`;|&<>(){}$*?\\]+")
TEXT_LIMIT = 4 * 1024 * 1024


# The fact model.

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


def marks(text):
    return collections.Counter(line.strip() for line in text.split("\n") if IGNORE_MARK.search(line))


def ignore_additions(before, after):
    """The stripped lines carrying an ignore mark that the after text has more of than the before text."""
    added = marks(after) - marks(before)
    return [line for line, count in added.items() for _ in range(count)]


# The owner's allowlist.

def allowlist(hooks_dir=HOOKS):
    """The owner's allowlist; absent or unparsable admits nothing."""
    try:
        with open(os.path.join(hooks_dir, ALLOWLIST), "rb") as handle:
            doc = tomllib.load(handle)
    except (OSError, ValueError):
        return {}
    return doc if isinstance(doc, dict) else {}


def entries(doc, kind):
    found = doc.get(kind)
    return [e for e in found if isinstance(e, dict)] if isinstance(found, list) else []


def same_path(full, path):
    return isinstance(path, str) and path != "" and (full == path or full.endswith("/" + path.lstrip("/")))


def complete(entry, keys):
    return all(k in entry for k in keys) and isinstance(entry.get("reason"), str) and entry["reason"].strip() != ""


def admitted_change(listed, full, change):
    kind, fact, value = change
    return any(complete(e, ("path", "change", "fact", "value", "reason")) and same_path(full, e["path"]) and
               e["change"] == kind and e["fact"] == fact and canonical(e["value"]) == canonical(value)
               for e in listed)


def admitted_line(listed, full, line):
    return any(complete(e, ("path", "line", "reason")) and same_path(full, e["path"]) and
               isinstance(e["line"], str) and e["line"].strip() == line for e in listed)


def allowlist_problem(full):
    return (full + " is the owner's allowlist of changes admitted to closed files; agents never write it, "
            "the owner edits it by hand in idiomatic-internal")


# Refusals.

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


def refusal(full, what, blocks):
    return (full + ": " + what + ". The owner admits a change by committing its block, with a reason, to " +
            ALLOWLIST + " beside the hooks in idiomatic-internal:\n\n" + "\n\n".join(blocks))


def parsed(text):
    """The TOML document of a text, {} for an absent file, None when it does not parse."""
    if text is None:
        return {}
    try:
        return tomllib.loads(text)
    except ValueError:
        return None


def manifest_problem(full, shown, bases, after, listed):
    """The refusal for a manifest whose text becomes after, or None. A change is refused only when it is refused
    against every base (the text before, and any text committed before the command that git brought in)."""
    if after is not None and after in bases:
        return None
    after_doc = parsed(after)
    if after_doc is None:
        return full + " would not parse as TOML; a manifest edit must leave it valid"
    left = None
    for base in bases:
        changes = collections.Counter((kind, fact, canonical(value)) for kind, fact, value in
                                      manifest_changes(parsed(base) or {}, after_doc))
        left = changes if left is None else left & changes
    left = [(kind, fact, json.loads(value)) for (kind, fact, value), count in (left or {}).items()
            for _ in range(count)]
    left = [c for c in left if not admitted_change(listed, full, c)]
    if not left:
        return None
    blocks = ["[[manifest]]\npath = " + json.dumps(shown) + "\nchange = " + json.dumps(kind) + "\nfact = " +
              json.dumps(fact) + "\nvalue = " + toml_value(value) + '\nreason = ""' for kind, fact, value in left]
    return refusal(full, "a Tastemaker manifest is closed for extension: an agent may remove an exclude, except, "
                   "allow or rules-exclude element or a single-file template placement, and edit descriptions, "
                   "comments and order; every addition and every other removal is the owner's. Refused: " +
                   "; ".join(kind + " " + fact + " = " + toml_value(value) for kind, fact, value in left), blocks)


def ignore_problem(full, shown, bases, after, listed):
    """The refusal for ignore lines the after text adds against every base, or None."""
    left = None
    for base in bases:
        added = marks(after) - marks(base or "")
        left = added if left is None else left & added
    left = [line for line, count in (left or {}).items() for _ in range(count)]
    left = [line for line in left if not admitted_line(listed, full, line)]
    if not left:
        return None
    blocks = ["[[ignore]]\npath = " + json.dumps(shown) + "\nline = " + json.dumps(line) + '\nreason = ""'
              for line in left]
    return refusal(full, "an added in-source ignore comment is the owner's (" + "; ".join(left) + "); fix the "
                   "finding, or fix the rule with a negative fixture", blocks)


# The Bash effect check.

def git(root, *args, data=None, env=None):
    done = subprocess.run(["git", "-C", root, *args], input=data, capture_output=True, check=False,
                          env=dict(os.environ, **env) if env else None)
    return done.returncode, done.stdout


def state_dir():
    """The hooks' state root as no_subagent_builders.py defines it, with this hook's own directory under it."""
    spec = importlib.util.spec_from_file_location("no_subagent_builders", os.path.join(HOOKS, "no_subagent_builders.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return os.path.join(module.STATE, "closed-effect")


def snapshot_key(data):
    found = data.get("tool_use_id")
    if isinstance(found, str) and re.fullmatch(r"[\w-]+", found):
        return found
    text = json.dumps([data.get("session_id"), data.get("cwd"), (data.get("tool_input") or {}).get("command")])
    return hashlib.sha256(text.encode()).hexdigest()


def roots(data):
    """The git top levels the command may touch: its working directory's, the project's, and those of the absolute
    paths its text names (each walked up to an existing directory)."""
    command = str((data.get("tool_input") or {}).get("command") or "")
    places = [data.get("cwd") or os.getcwd(), os.environ.get("CLAUDE_PROJECT_DIR") or ""]
    places += ABSOLUTE.findall(command)
    seen, found = set(), []
    for place in places:
        place = os.path.abspath(place) if place else ""
        while place and not os.path.isdir(place):
            place = os.path.dirname(place)
        if not place or place in seen:
            continue
        seen.add(place)
        code, out = git(place, "rev-parse", "--show-toplevel")
        top = out.decode().strip() if code == 0 else ""
        if top and top not in found:
            found.append(top)
    return found


def tree_of(root, scratch):
    """(HEAD sha or None, the sha of a git tree holding every tracked and untracked file as it is now)."""
    code, out = git(root, "rev-parse", "-q", "--verify", "HEAD")
    head = out.decode().strip() if code == 0 else None
    code, out = git(root, "rev-parse", "--path-format=absolute", "--git-path", "index")
    index = out.decode().strip()
    temp = os.path.join(scratch, "index-" + hashlib.sha256(root.encode()).hexdigest()[:16] + "-" + str(os.getpid()))
    try:
        if code == 0 and os.path.isfile(index):
            with open(index, "rb") as source, open(temp, "wb") as target:
                target.write(source.read())
        env = {"GIT_INDEX_FILE": temp}
        code, out = git(root, "add", "-A", env=env)
        if code != 0:
            raise RuntimeError("git add -A into a temporary index failed in " + root)
        code, out = git(root, "write-tree", env=env)
        if code != 0:
            raise RuntimeError("git write-tree failed in " + root)
        return head, out.decode().strip()
    finally:
        if os.path.exists(temp):
            os.remove(temp)


def read_allowlist_text():
    try:
        with open(os.path.join(HOOKS, ALLOWLIST), encoding="utf-8") as handle:
            return handle.read()
    except FileNotFoundError:
        return None


def prune(scratch):
    """Snapshots older than a day: commands whose PostToolUse never ran (the Bash guard refused them)."""
    for name in os.listdir(scratch):
        path = os.path.join(scratch, name)
        try:
            if time.time() - os.path.getmtime(path) > 86400:
                os.remove(path)
        except OSError:
            pass


def before(data):
    """PreToolUse: write the snapshot, or a record of why it failed. Never refuses."""
    scratch = state_dir()
    os.makedirs(scratch, exist_ok=True)
    path = os.path.join(scratch, snapshot_key(data) + ".json")
    try:
        prune(scratch)
        record = {"allowlist": read_allowlist_text(),
                  "roots": {root: list(tree_of(root, scratch)) + [ref_tips(root)] for root in roots(data)}}
    except Exception as error:
        record = {"error": type(error).__name__ + ": " + str(error)}
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(record, handle)
    return 0


def blobs(root, specs):
    """The contents of <tree-ish>:<path> specs in one git cat-file --batch run; None for a missing one."""
    if not specs:
        return {}
    code, out = git(root, "cat-file", "--batch", data=("\n".join(specs) + "\n").encode())
    found, at = {}, 0
    for spec in specs:
        end = out.index(b"\n", at)
        header = out[at:end]
        at = end + 1
        if header.endswith((b" missing", b" ambiguous")):
            found[spec] = None
            continue
        kind, size = header.split()[-2:]
        found[spec] = out[at:at + int(size)] if kind == b"blob" else None
        at += int(size) + 1
    return found


CONFLICTED = re.compile(r"^<<<<<<< ", re.MULTILINE)
IN_PROGRESS = ("MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD", "REBASE_HEAD")


def committed(root):
    """HEAD and the other side of a merge, cherry-pick, revert or rebase in progress: committed content a
    resolution may take."""
    found = []
    for name in ("HEAD",) + IN_PROGRESS:
        code, out = git(root, "rev-parse", "-q", "--verify", name + "^{commit}")
        if code == 0 and out.strip():
            found.append(out.decode().strip())
    return found


def committed_texts(root, path):
    """The texts of path (relative to root) at each committed() commit; None where it is absent."""
    revs = committed(root)
    found = blobs(root, [rev + ":" + path for rev in revs])
    return [text(found[rev + ":" + path]) if found[rev + ":" + path] is not None else None for rev in revs]


def ref_tips(root, pattern=None):
    """The commit shas every ref (or the refs under pattern) and HEAD point at."""
    code, out = git(root, "for-each-ref", "--format=%(objectname)", *([pattern] if pattern else []))
    found = set(out.decode().split()) if code == 0 else set()
    code, out = git(root, "rev-parse", "-q", "--verify", "HEAD")
    if code == 0 and out.strip() and not pattern:
        found.add(out.decode().strip())
    return sorted(found)


def tips(root, refs, head2):
    """The commits that existed before the command whose content the working tree may now hold: the new HEAD unless
    the command made it, and every parent of a commit the command made that it did not make. A commit the command
    made is one no ref reached before the command and no remote-tracking ref reaches after it."""
    if head2 is None:
        return []
    known = set(refs) | set(ref_tips(root, "refs/remotes"))
    code, out = git(root, "rev-list", "--parents", head2, "--stdin", data="".join("^" + k + "\n" for k in known).encode())
    if code != 0:
        raise RuntimeError("git rev-list failed in " + root)
    made = {}
    for line in out.decode().split("\n"):
        words = line.split()
        if words:
            made[words[0]] = words[1:]
    found = [] if head2 in made else [head2]
    for parents in made.values():
        found += [p for p in parents if p not in made and p not in found]
    return found


def text(content):
    """Decoded text of a blob, or None when absent, binary or too large to judge."""
    if content is None or len(content) > TEXT_LIMIT or b"\0" in content[:8000]:
        return None
    return content.decode("utf-8", errors="replace")


def restore(root, path, content):
    full = os.path.join(root, path)
    if content is None:
        if os.path.lexists(full):
            os.remove(full)
        return
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, "wb") as handle:
        handle.write(content)


def check_root(root, tree1, refs, scratch, listed):
    head2, tree2 = tree_of(root, scratch)
    if tree1 == tree2:
        return []
    code, out = git(root, "diff-tree", "-r", "-z", "--no-renames", "--name-only", tree1, tree2)
    changed = [p.decode() for p in out.split(b"\0") if p]
    commits = tips(root, refs, head2)
    operation = [c for c in committed(root)[1:] if c not in commits]
    commits += operation
    now = blobs(root, [tree2 + ":" + p for p in changed])
    candidates = [p for p in changed if os.path.basename(p) == MANIFEST or
                  (text(now[tree2 + ":" + p]) is not None and marks(text(now[tree2 + ":" + p])))]
    old = blobs(root, [rev + ":" + p for p in candidates for rev in [tree1] + commits])
    problems = []
    for path in candidates:
        full = os.path.join(root, path)
        after = now[tree2 + ":" + path]
        bases = [old[rev + ":" + path] for rev in [tree1] + commits]
        found = None
        if os.path.basename(path) == MANIFEST and operation and CONFLICTED.search(text(after) or ""):
            continue  # a merge, cherry-pick, revert or rebase stopped on this file; its resolution is checked
        if os.path.basename(path) == MANIFEST:
            found = manifest_problem(full, path, [text(b) if b is not None else None for b in bases],
                                     text(after) if after is not None else None, entries(listed, "manifest"))
        if not found and text(after) is not None:
            found = ignore_problem(full, path, [text(b) or "" for b in bases], text(after), entries(listed, "ignore"))
        if found:
            restore(root, path, old[tree1 + ":" + path])
            problems.append(found)
    return problems


REVERTED = ("The command ran, and each file above is restored to its content before the command (any other change "
            "the command made to that file is reverted with it; a new file is removed). If the command staged or "
            "committed the change, the index or the commit still holds it: commit the restored file. Make manifest "
            "edits with the Edit or Write tool, which diffs them before writing.")


def after(data):
    """PostToolUse: refuse and revert what the command extended or loosened."""
    scratch = state_dir()
    path = os.path.join(scratch, snapshot_key(data) + ".json")
    try:
        with open(path, encoding="utf-8") as handle:
            record = json.load(handle)
        os.remove(path)
    except (OSError, ValueError) as error:
        record = {"error": "no snapshot was taken before the command (" + type(error).__name__ + ")"}
    if "error" in record:
        print("Blocked: the closed-for-extension snapshot before this command failed: " + record["error"] +
              ". Nothing was restored: check `git status` and `git diff` for an added manifest fact or ignore line "
              "and undo it with the Edit tool.", file=sys.stderr)
        return 2
    problems = []
    if read_allowlist_text() != record["allowlist"]:
        restore(HOOKS, ALLOWLIST, None if record["allowlist"] is None else record["allowlist"].encode())
        problems.append(allowlist_problem(os.path.join(HOOKS, ALLOWLIST)))
    listed = allowlist()
    for root, (head1, tree1, refs) in record["roots"].items():
        problems += check_root(root, tree1, refs, scratch, listed)
    if problems:
        print("Blocked: " + "\n\n".join(problems) + "\n\n" + REVERTED, file=sys.stderr)
        return 2
    return 0


def main():
    try:
        data = json.load(sys.stdin)
    except ValueError:
        return 0
    if not isinstance(data, dict) or data.get("tool_name") not in (None, "Bash"):
        return 0
    if data.get("hook_event_name") == "PreToolUse":
        return before(data)
    if data.get("hook_event_name") != "PostToolUse":
        return 0
    try:
        return after(data)
    except Exception as error:
        print("Blocked: the closed-for-extension check after this command failed: " + type(error).__name__ + ": " +
              str(error) + ". Nothing was restored: check `git status` and `git diff` for an added manifest fact or "
              "ignore line and undo it with the Edit tool.", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
