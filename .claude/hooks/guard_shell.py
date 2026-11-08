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
EVIDENCE = "crates/ryo/docs/contracts/evidence/checkpoint/"
RUST_REVIEW = EVIDENCE + "rust-review.json"
BOOTSTRAP_HEAD = "171789169e"  # checkpoint 329: landed before the leak and TM agents existed
PREVIOUS_REVIEWS = (("contract-review.json", "contract-review"), ("leak-review.json", "leak-review"),
                    ("tm-steward.json", "tm-specialist"))


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


def previous_head(tree, remote):
    """The remote claude/ryo-consolidation head before this push, or None when it cannot be read."""
    code, out = git_out(tree, "ls-remote", remote, "refs/heads/" + CONSOLIDATION)
    if code != 0 or not out.strip():
        code, out = git_out(tree, "rev-parse", "--verify", "-q", "refs/remotes/" + remote + "/" + CONSOLIDATION)
    words = out.split()
    return words[0] if code == 0 and words else None


def stands_for(tree, source, head):
    """True when source is head, or an ancestor of head with only evidence files changed since."""
    if source == head:
        return True
    if git_out(tree, "merge-base", "--is-ancestor", source, head)[0] != 0:
        return False
    code, out = git_out(tree, "diff", "--name-only", source, head)
    return code == 0 and all(line.startswith(EVIDENCE) for line in out.split())


def previous_reviews(words, here):
    """A push landing a checkpoint needs the previous landed head's contract, leak and TM steward reviews in HEAD.

    Verdicts never block (a BLOCKING or FAIL is repaired in this checkpoint); absence or a stale source does.
    At the bootstrap head only contract-review.json is required: the leak and TM agents did not exist then.
    """
    push = git_push(words)
    if push is None or not names_consolidation(push[1]):
        return None
    tree = os.path.join(here, push[0]) if push[0] else here
    operands = [a for a in push[1] if not a.startswith("-")]
    head = previous_head(tree, operands[0]) if operands else None
    if head is None:
        return None
    refused = "push of " + CONSOLIDATION + " refused: "
    needed = PREVIOUS_REVIEWS[:1] if head.startswith(BOOTSTRAP_HEAD) else PREVIOUS_REVIEWS
    for name, agent in needed:
        run = "run the " + agent + " agent (.agents/agents/" + agent + ".md) on " + head
        code, text = git_out(tree, "show", "HEAD:" + EVIDENCE + name)
        if code != 0:
            return refused + "HEAD carries no " + EVIDENCE + name + " for the previous landed head " + head + "; " + run
        try:
            review = json.loads(text)
        except ValueError:
            return refused + EVIDENCE + name + " at HEAD is not JSON; " + run
        if not isinstance(review, dict) or "verdict" not in review:
            return refused + EVIDENCE + name + " at HEAD has no \"verdict\"; " + run
        source = review.get("source")
        if not isinstance(source, str) or not source or source.startswith("-") or not stands_for(tree, source, head):
            return (refused + "the source " + repr(source) + " in " + EVIDENCE + name + " is not the previous landed head " +
                    head + " (or an ancestor with only evidence changed since); " + run)
    return None


PERF_DIR = "crates/ryo/docs/contracts/evidence/perf/"
PERF_SUBJECT = re.compile(r"perf(\([^)]*\))?!?:")
PERF_NUMBER = re.compile(r"\d\s?(ms|s|MB|GB|%)(?!\w)")
PERF_AGENT = ".agents/agents/perf-diagnosis.md"


def pushed_branch(tree, push):
    """The remote and destination branch of a push: its operands, or the current branch when it names none."""
    operands = [a for a in push[1] if not a.startswith("-")]
    remote = operands[0] if operands else "origin"
    if len(operands) > 1:
        return remote, operands[-1].lstrip("+").rsplit(":", 1)[-1].replace("refs/heads/", "", 1)
    return remote, git_out(tree, "symbolic-ref", "--short", "HEAD")[1].strip()


def pushed_commits(tree, remote, branch):
    """(sha, message) of the commits a push adds, oldest first; commits in claude/ryo-consolidation are never included."""
    base = None
    code, out = git_out(tree, "rev-parse", "--verify", "-q", "refs/remotes/" + remote + "/" + branch)
    if code == 0 and out.strip():
        base = out.strip()
    exclude = []
    for ref in ("refs/heads/" + CONSOLIDATION, "refs/remotes/" + remote + "/" + CONSOLIDATION):
        if git_out(tree, "rev-parse", "--verify", "-q", ref)[0] == 0:
            exclude.append(ref)
    if base is None:
        if not exclude:
            return []
        base = git_out(tree, "merge-base", "HEAD", exclude[0])[1].strip()
        if not base:
            return []
    args = ["log", "--reverse", "--format=%H%x1f%B%x1e", base + "..HEAD"]
    for ref in exclude:
        args += ["--not", ref]
    code, out = git_out(tree, *args)
    rows = [row.strip("\n").split("\x1f", 1) for row in out.split("\x1e") if row.strip()]
    return [(row[0].strip(), row[1] if len(row) > 1 else "") for row in rows]


def perf_gate(words, here):
    """A push with a `perf` commit needs a perf diagnosis (and, for claimed numbers, an after.json) in the pushed tree."""
    push = git_push(words)
    if push is None:
        return None
    tree = os.path.join(here, push[0]) if push[0] else here
    remote, branch = pushed_branch(tree, push)
    perf = [(sha, msg) for sha, msg in pushed_commits(tree, remote, branch)
            if PERF_SUBJECT.match(msg.lstrip().split("\n", 1)[0])]
    if not perf:
        return None
    first = perf[0][0]
    refused = "push of " + branch + " refused: perf commit " + first[:10] + " "
    point = "; run the perf-diagnosis agent (" + PERF_AGENT + ")"
    code, listing = git_out(tree, "ls-tree", "-r", "--name-only", "HEAD", PERF_DIR)
    diagnoses = [f for f in listing.split("\n") if re.fullmatch(re.escape(PERF_DIR) + r"[^/]+/diagnosis\.json", f)]
    problems = []
    for path in diagnoses:
        try:
            doc = json.loads(git_out(tree, "show", "HEAD:" + path)[1])
        except ValueError:
            problems.append(path + " is not JSON")
            continue
        doc = doc if isinstance(doc, dict) else {}
        source = doc.get("source")
        hypotheses = doc.get("hypotheses")
        if not isinstance(source, str) or not source or source.startswith("-") or \
                git_out(tree, "merge-base", "--is-ancestor", source, first)[0] != 0:
            problems.append(path + " has a \"source\" that is not an ancestor of the first perf commit")
        elif not any(isinstance(h, dict) and h.get("verdict") in ("confirmed", "refuted")
                     for h in (hypotheses if isinstance(hypotheses, list) else [])):
            problems.append(path + " has no hypothesis with a verdict (confirmed or refuted)")
        elif not doc.get("levers") or not isinstance(doc.get("levers"), list):
            problems.append(path + " has no lever")
        else:
            if not PERF_NUMBER.search("\n".join(msg for _, msg in perf)):
                return None
            after = path.rsplit("/", 1)[0] + "/after.json"
            code, text = git_out(tree, "show", "HEAD:" + after)
            try:
                record = json.loads(text) if code == 0 else None
            except ValueError:
                record = None
            if isinstance(record, dict) and all(k in record for k in ("before", "after", "identical_output")):
                return None
            problems.append("a perf commit claims a number but " + after + " lacks before, after and identical_output")
    if not diagnoses:
        return refused + "has no " + PERF_DIR + "<iteration>/diagnosis.json in the pushed tree" + point
    return refused + "; ".join(problems) + point


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
                      rust_review(words, here), previous_reviews(words, here), perf_gate(words, here)):
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


if __name__ == "__main__":
    sys.exit(main())
