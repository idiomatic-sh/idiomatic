"""PreToolUse hook for Bash: blocks the shell habits that have cost this repository work."""

import json
import os
import re
import shlex
import subprocess
import sys

LOCK = "/workspaces/.cargo-build.lock"
TARGET_ROOT = "/workspaces/idiomatic-target"
CHECK_TARGET = "/workspaces/idiomatic-check-target"
FIXTURE_FIX = "cli/tests/replay/imports.rs"
CARGO_BUILDS = {"build", "test", "bench", "clippy", "run", "check", "doc"}
INTEGRATION = re.compile(r"claude/(checkpoint-\d+|ryo-consolidation)")
MEASUREMENT = "/workspaces/ryo-perf-"
CHECK_FORM = ("cargo check -p <crate> --target-dir " + CHECK_TARGET + " or --target-dir " + CHECK_TARGET + "/<worktree name>")
BUILDERS_AUTHOR = ("Builders author; the integrator compiles. cargo builds only in an integration worktree "
                   "(branch claude/checkpoint-N); builders may run " + CHECK_FORM)
COMMENT = re.compile(r"(^|\s)#.*$", re.MULTILINE)
CONSOLIDATION = "claude/ryo-consolidation"
RUST_REVIEW = "crates/ryo/docs/contracts/evidence/checkpoint/rust-review.json"


def segments(text):
    """Split at command separators outside quotes; quotes stay in the segment."""
    out, current, quote, i = [], [], None, 0
    while i < len(text):
        c = text[i]
        if c == "\\" and quote != "'" and i + 1 < len(text):
            current.append(text[i:i + 2])
            i += 2
            continue
        if quote:
            quote = None if c == quote else quote
        elif c in "'\"":
            quote = c
        elif c in ";\n|()`" or (c == "&" and text[i - 1:i] != ">" and text[i + 1:i + 2] != ">"):
            out.append("".join(current))
            current = []
            i += 1
            continue
        current.append(c)
        i += 1
    out.append("".join(current))
    return out


WRAPPERS = {"env", "time", "exec", "nohup", "command", "nice"}


def runs_cargo_verb(words):
    """True when the word list is a command that runs cargo with a build verb, directly or through a wrapper."""
    while words and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", words[0]):
        words = words[1:]
    while words and words[0] in WRAPPERS:
        words = words[1:]
    if not words:
        return False
    if words[0] == "flock":
        rest = words[1:]
        while rest and rest[0].startswith("-"):
            rest = rest[2:] if rest[0] in ("-w", "-E", "--timeout", "--conflict-exit-code") else rest[1:]
        return runs_cargo_verb(rest[1:])
    if words[0] in ("bash", "sh", "zsh") and "-c" in words and words.index("-c") + 1 < len(words):
        return has_cargo_command(words[words.index("-c") + 1])
    if words[0] == "cargo":
        rest = [w for w in words[1:] if not w.startswith("+")]
        return next((w for w in rest if not w.startswith("-")), "") in CARGO_BUILDS
    return False


def has_cargo_command(text):
    return any(runs_cargo_verb(tokens(segment)) for segment in segments(text))


def tokens(segment):
    try:
        found = shlex.split(segment)
    except ValueError:
        found = segment.split()
    while found and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", found[0]):
        found = found[1:]
    return found


def stash(words):
    if len(words) < 2 or words[0] != "git":
        return None
    rest = [w for w in words[1:] if not w.startswith("-")] if words[1].startswith("-") else words[1:]
    if not rest or rest[0] != "stash":
        return None
    if len(rest) == 1 or rest[1] == "pop":
        return "bare git stash/stash pop shares one stack across worktrees; commit, or use git stash push -u -m"
    return None


def removal(words):
    if not words or words[0] != "rm":
        return None
    flags = [w for w in words[1:] if w.startswith("-")]
    if not any(re.search(r"[rR]", f) or f == "--recursive" for f in flags):
        return None
    for target in (w for w in words[1:] if not w.startswith("-")):
        if "*" in target or "?" in target:
            return "recursive rm with a glob; delete only exact paths you created"
    return None


def cargo_index(words):
    """Index of `cargo` when the segment runs it directly, behind an optional flock."""
    if not words:
        return None
    if words[0] == "flock":
        for i, w in enumerate(words):
            if w == "cargo":
                return i
        return None
    return 0 if words[0] == "cargo" else None


def target_dir(rest):
    """The --target-dir value the cargo arguments name, or None."""
    for i, w in enumerate(rest):
        if w.startswith("--target-dir="):
            return w.split("=", 1)[1]
        if w == "--target-dir":
            return rest[i + 1] if i + 1 < len(rest) else ""
    return None


def is_check_target(target):
    """The check dir itself or one path component under it."""
    if target == CHECK_TARGET:
        return True
    name = (target or "")[len(CHECK_TARGET) + 1:] if (target or "").startswith(CHECK_TARGET + "/") else ""
    return name not in ("", ".", "..") and "/" not in name


def tree_name(directory):
    """The name of the worktree a directory is in: its git top level's basename, else the directory's own."""
    code, out = git_out(directory, "rev-parse", "--show-toplevel")
    return os.path.basename(out.strip() if code == 0 and out.strip() else os.path.abspath(directory))


def own_target(dirs):
    """The one cargo target dir of the worktree the command runs in, or None when it names several trees."""
    names = {tree_name(d) for d in dirs}
    return TARGET_ROOT + "/" + names.pop() if len(names) == 1 else None


def cargo_rules(words, before, dirs):
    at = cargo_index(words)
    if at is None:
        return None
    rest = [w for w in words[at + 1:] if not w.startswith("+")]
    sub = next((w for w in rest if not w.startswith("-")), "")
    target = target_dir(rest)
    target = os.path.normpath(target) if target else target
    checking = sub == "check" and is_check_target(target)
    own = own_target(dirs)
    if target is not None and target != own and not checking:
        return ("cargo --target-dir must be " + TARGET_ROOT + "/<this worktree's name>" +
                (" (here " + own + ")" if own else "") +
                ": one target per worktree, so no build reuses another tree's units (or " + CHECK_TARGET +
                " or " + CHECK_TARGET + "/<worktree name> for cargo check)")
    if sub in CARGO_BUILDS and not checking and LOCK not in " ".join(words[:at]) and LOCK not in before:
        return "cargo " + sub + " must run as: flock " + LOCK + " cargo " + sub
    return None


def one_crate_manifest(path):
    try:
        with open(path, encoding="utf-8") as handle:
            return any(line.strip() == "[package]" for line in handle)
    except OSError:
        return False


def builder_check(words, here):
    """Outside an integration tree only a package-scoped cargo check into the check-only target dir runs."""
    at = cargo_index(words)
    if at is None:
        return BUILDERS_AUTHOR
    rest = [w for w in words[at + 1:] if not w.startswith("+")]
    if next((w for w in rest if not w.startswith("-")), "") != "check":
        return BUILDERS_AUTHOR
    if not is_check_target(target_dir(rest)):
        return "cargo check outside an integration tree must run as: " + CHECK_FORM
    if any(w in ("--workspace", "--all") for w in rest):
        return "a workspace-wide cargo check is the integrator's; run: " + CHECK_FORM
    packaged = any(w in ("-p", "--package") or w.startswith("--package=") or (w.startswith("-p") and len(w) > 2)
                   for w in rest)
    manifests = [rest[i + 1] if w == "--manifest-path" and i + 1 < len(rest) else w.split("=", 1)[1]
                 for i, w in enumerate(rest) if w == "--manifest-path" or w.startswith("--manifest-path=")]
    if not packaged and not (manifests and all(one_crate_manifest(os.path.join(here, m)) for m in manifests)):
        return "a workspace-wide cargo check is the integrator's; run: " + CHECK_FORM
    return None


def tm_fix(words, command):
    if not words or words[0].rsplit("/", 1)[-1] != "tm":
        return None
    if "check" in words and "--fix" in words and FIXTURE_FIX not in command:
        return "tm check --fix is allowed only for the fixture " + FIXTURE_FIX
    return None


def git_push(words):
    """The -C directory and the arguments of a `git push`, or None when the words are not one."""
    if len(words) < 2 or words[0] != "git":
        return None
    directory, i = None, 1
    while i < len(words) and words[i].startswith("-"):
        if words[i] == "-C" and i + 1 < len(words):
            directory = words[i + 1]
            i += 2
        elif words[i] in ("-c", "--git-dir", "--work-tree") and i + 1 < len(words):
            i += 2
        else:
            i += 1
    if i >= len(words) or words[i] != "push":
        return None
    return directory, words[i + 1:]


def names_consolidation(args):
    """True when a refspec argument's destination is claude/ryo-consolidation."""
    for arg in (a for a in args if not a.startswith("-")):
        destination = arg.lstrip("+").rsplit(":", 1)[-1]
        if destination in (CONSOLIDATION, "refs/heads/" + CONSOLIDATION):
            return True
    return False


def git_out(directory, *args):
    done = subprocess.run(["git", "-C", directory, *args], capture_output=True, text=True, check=False)
    return done.returncode, done.stdout


def rust_review(words, here):
    """A push of claude/ryo-consolidation needs HEAD's rust-review.json at PASS with a source HEAD contains."""
    push = git_push(words)
    if push is None or not names_consolidation(push[1]):
        return None
    tree = os.path.join(here, push[0]) if push[0] else here
    refused = "push of " + CONSOLIDATION + " refused: "
    code, text = git_out(tree, "show", "HEAD:" + RUST_REVIEW)
    if code != 0:
        return (refused + "HEAD carries no " + RUST_REVIEW + "; run the rust-principal-review agent "
                "(.agents/agents/rust-principal-review.md) on this tree and commit its PASS")
    try:
        review = json.loads(text)
    except ValueError:
        return refused + RUST_REVIEW + " at HEAD is not JSON"
    if not isinstance(review, dict) or review.get("verdict") != "PASS":
        verdict = review.get("verdict") if isinstance(review, dict) else None
        return (refused + RUST_REVIEW + " at HEAD has verdict " + repr(verdict) +
                ", not \"PASS\"; repair its findings and review again, or the owner overrules and pushes")
    source = review.get("source")
    if not isinstance(source, str) or not source or source.startswith("-"):
        return refused + RUST_REVIEW + " at HEAD names no reviewed \"source\" sha"
    if git_out(tree, "merge-base", "--is-ancestor", source, "HEAD")[0] != 0:
        return (refused + "the reviewed source " + source + " in " + RUST_REVIEW +
                " is not an ancestor of HEAD; review the tree being pushed")
    return None


def integration_tree(directory):
    if os.path.abspath(directory).startswith(MEASUREMENT):
        return True
    done = subprocess.run(["git", "-C", directory, "symbolic-ref", "--short", "HEAD"],
                          capture_output=True, text=True, check=False)
    return done.returncode == 0 and INTEGRATION.fullmatch(done.stdout.strip()) is not None


def working_dirs(clean, cwd):
    """The directory the command runs in, plus any -C or --manifest-path directory it names."""
    here = cwd or os.getcwd()
    for segment in segments(clean):
        words = tokens(segment)
        if len(words) > 1 and words[0] == "cd":
            here = os.path.join(here, os.path.expanduser(words[1]))
    found = [here]
    for match in re.finditer(r"(?:^|\s)-C\s+(\S+)|--manifest-path[= ](\S+)", clean):
        if match.group(1):
            found.append(os.path.join(here, match.group(1).strip("'\"")))
        else:
            found.append(os.path.dirname(os.path.join(here, match.group(2).strip("'\""))))
    return found


def reason(command, cwd=None):
    clean = COMMENT.sub("", command)
    if re.search(r"\b(while|until|for)\b", clean) and re.search(r"\b(pgrep|pkill)\b[^;&|\n]*\s-[A-Za-z]*f", clean):
        return "self-matching wait loop (pgrep/pkill -f in a loop); use pid files or pgrep -x"
    if has_cargo_command(clean) and not all(integration_tree(d) for d in working_dirs(clean, cwd)):
        here = working_dirs(clean, cwd)[0]
        for segment in segments(clean):
            words = tokens(segment)
            found = builder_check(words, here) if runs_cargo_verb(words) else None
            if found:
                return found
    before, here, dirs = "", cwd or os.getcwd(), working_dirs(clean, cwd)
    for segment in segments(clean):
        words = tokens(segment)
        if len(words) > 1 and words[0] == "cd":
            here = os.path.join(here, os.path.expanduser(words[1]))
        for found in (stash(words), removal(words), cargo_rules(words, before, dirs), tm_fix(words, clean),
                      rust_review(words, here)):
            if found:
                return found
        before += segment + " "
    return None


def main():
    try:
        data = json.load(sys.stdin)
    except ValueError:
        return 0
    command = (data.get("tool_input") or {}).get("command") or ""
    found = reason(command, data.get("cwd"))
    if found:
        print("Blocked: " + found, file=sys.stderr)
        return 2
    return 0


sys.exit(main())
