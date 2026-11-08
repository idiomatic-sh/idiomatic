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
EVIDENCE_ROOT = "crates/ryo/docs/contracts/evidence/"
EVIDENCE = EVIDENCE_ROOT + "checkpoint/"
RUST_REVIEW = EVIDENCE + "rust-review.json"
BOOTSTRAP_HEAD = "171789169e"  # checkpoint 329: landed before the leak and TM agents existed
PREVIOUS_REVIEWS = (("contract-review.json", "contract-review"), ("leak-review.json", "leak-review"),
                    ("tm-steward.json", "tm-steward"))
MERGE_BRANCHES = "crates/ryo/docs/contracts/evidence/branches/"


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


def shell_command(words):
    """The command string of `bash|sh|zsh -c` (also -lc, -ec ...), or None."""
    if not words or words[0] not in ("bash", "sh", "zsh"):
        return None
    for i, w in enumerate(words[1:], 1):
        if re.fullmatch(r"-[A-Za-z]*c[A-Za-z]*", w):
            return words[i + 1] if i + 1 < len(words) else None
    return None


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
    inner = shell_command(words)
    if inner is not None:
        return has_cargo_command(inner)
    if words[0] == "cargo":
        rest = [w for w in words[1:] if not w.startswith("+")]
        return next((w for w in rest if not w.startswith("-")), "") in CARGO_BUILDS
    return False


def has_cargo_command(text):
    return any(runs_cargo_verb(tokens(segment)) for segment in segments(text))


REDIRECT = re.compile(r"^\d*[<>]")


def without_redirections(words):
    """Drop shell redirections (2>&1, >file, 2>/dev/null, and a bare operator with its target)."""
    out, i = [], 0
    while i < len(words):
        if REDIRECT.match(words[i]):
            bare = re.fullmatch(r"\d*(>>|>|<)", words[i]) is not None
            i += 2 if bare else 1
        else:
            out.append(words[i])
            i += 1
    return out


def tokens(segment):
    try:
        found = shlex.split(segment)
    except ValueError:
        found = segment.split()
    found = without_redirections(found)
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


def git_verb(words, verb):
    """The -C directory and the operands of `git <verb>`, or None when the words are not one."""
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
    if i >= len(words) or words[i] != verb:
        return None
    return directory, words[i + 1:]


def git_out(directory, *args):
    done = subprocess.run(["git", "-C", directory, *args], capture_output=True, text=True, check=False)
    return done.returncode, done.stdout


def rev(tree, ref):
    """The commit sha a ref resolves to, or None."""
    if not ref or ref.startswith("-"):
        return None
    code, out = git_out(tree, "rev-parse", "--verify", "-q", ref + "^{commit}")
    return out.strip() if code == 0 and out.strip() else None


def current_branch(tree):
    return git_out(tree, "symbolic-ref", "--short", "HEAD")[1].strip()


def push_targets(tree, push):
    """(remote, source, destination) for each refspec a push names; no refspec pushes HEAD to the current branch."""
    operands = [a for a in push[1] if not a.startswith("-")]
    remote = operands[0] if operands else "origin"
    current = current_branch(tree)
    if len(operands) < 2:
        return [(remote, "HEAD", current)]
    found = []
    for spec in operands[1:]:
        spec = spec.lstrip("+")
        source, colon, destination = spec.partition(":")
        if not source:
            continue
        if not colon:
            destination = current if source == "HEAD" else source
        found.append((remote, source, destination.replace("refs/heads/", "", 1)))
    return found


def consolidation_targets(tree, push):
    return [t for t in push_targets(tree, push) if t[2] == CONSOLIDATION]


def bulk_push(words, here):
    """git push --all / --mirror would push refs no gate has looked at; this repository refuses them."""
    push = git_verb(words, "push")
    if push is not None and any(a in ("--all", "--mirror", "--branches") for a in push[1]):
        return "git push --all/--mirror/--branches pushes refs no gate checked; push each branch by name"
    return None


def rust_review(words, here):
    """A push of claude/ryo-consolidation needs the pushed ref's rust-review.json at PASS with a source it contains."""
    push = git_verb(words, "push")
    if push is None:
        return None
    tree = os.path.join(here, push[0]) if push[0] else here
    for _, source, _ in consolidation_targets(tree, push):
        refused = "push of " + CONSOLIDATION + " refused: "
        sha = rev(tree, source)
        if sha is None:
            return refused + "the pushed ref " + source + " does not resolve to a commit"
        code, text = git_out(tree, "show", sha + ":" + RUST_REVIEW)
        if code != 0:
            return (refused + source + " carries no " + RUST_REVIEW + "; run the rust-review subagent "
                    "on this tree and commit its PASS")
        try:
            review = json.loads(text)
        except ValueError:
            return refused + RUST_REVIEW + " at " + source + " is not JSON"
        if not isinstance(review, dict) or review.get("verdict") != "PASS":
            verdict = review.get("verdict") if isinstance(review, dict) else None
            return (refused + RUST_REVIEW + " at " + source + " has verdict " + repr(verdict) +
                    ", not \"PASS\"; repair its findings and run the rust-review subagent again, or the owner "
                    "overrules and pushes")
        reviewed = review.get("source")
        if not isinstance(reviewed, str) or not reviewed or reviewed.startswith("-"):
            return refused + RUST_REVIEW + " at " + source + " names no reviewed \"source\" sha"
        if git_out(tree, "merge-base", "--is-ancestor", reviewed, sha)[0] != 0:
            return (refused + "the reviewed source " + reviewed + " in " + RUST_REVIEW +
                    " is not an ancestor of " + source + "; run the rust-review subagent on the tree being pushed")
    return None


def previous_head(tree, remote):
    """The remote claude/ryo-consolidation head before this push, or None when it cannot be read."""
    code, out = git_out(tree, "ls-remote", remote, "refs/heads/" + CONSOLIDATION)
    if code != 0 or not out.strip():
        code, out = git_out(tree, "rev-parse", "--verify", "-q", "refs/remotes/" + remote + "/" + CONSOLIDATION)
    words = out.split()
    return words[0] if code == 0 and words else None


def stands_for(tree, source, head, prefix):
    """True when source is head, or an ancestor of head with only files under prefix changed since."""
    if source == head:
        return True
    if git_out(tree, "merge-base", "--is-ancestor", source, head)[0] != 0:
        return False
    code, out = git_out(tree, "diff", "--name-only", source, head)
    return code == 0 and all(line.startswith(prefix) for line in out.split())


def previous_reviews(words, here):
    """A push landing a checkpoint needs the previous landed head's contract, leak and TM steward reviews.

    Verdicts never block (a BLOCKING or FAIL is repaired in this checkpoint); absence or a stale source does.
    At the bootstrap head only contract-review.json is required: the leak and TM agents did not exist then.
    The reviews are read from the pushed ref.
    """
    push = git_verb(words, "push")
    if push is None:
        return None
    tree = os.path.join(here, push[0]) if push[0] else here
    for remote, source, _ in consolidation_targets(tree, push):
        sha = rev(tree, source)
        head = previous_head(tree, remote)
        if sha is None or head is None:
            continue
        refused = "push of " + CONSOLIDATION + " refused: "
        needed = PREVIOUS_REVIEWS[:1] if head.startswith(BOOTSTRAP_HEAD) else PREVIOUS_REVIEWS
        for name, agent in needed:
            run = "run the " + agent + " subagent on " + head
            code, text = git_out(tree, "show", sha + ":" + EVIDENCE + name)
            if code != 0:
                return (refused + source + " carries no " + EVIDENCE + name + " for the previous landed head " + head +
                        "; " + run)
            try:
                review = json.loads(text)
            except ValueError:
                return refused + EVIDENCE + name + " at " + source + " is not JSON; " + run
            if not isinstance(review, dict) or "verdict" not in review:
                return refused + EVIDENCE + name + " at " + source + " has no \"verdict\"; " + run
            reviewed = review.get("source")
            if not isinstance(reviewed, str) or not reviewed or reviewed.startswith("-") or \
                    not stands_for(tree, reviewed, head, EVIDENCE):
                return (refused + "the source " + repr(reviewed) + " in " + EVIDENCE + name +
                        " is not the previous landed head " + head +
                        " (or an ancestor with only evidence changed since); " + run)
    return None


PERF_DIR = "crates/ryo/docs/contracts/evidence/perf/"
PERF_SUBJECT = re.compile(r"perf(\([^)]*\))?!?:")
PERF_NUMBER = re.compile(r"\d\s?(ms|s|MB|GB|%)(?!\w)")


def excluded_refs(tree, remote):
    return [ref for ref in ("refs/heads/" + CONSOLIDATION, "refs/remotes/" + remote + "/" + CONSOLIDATION)
            if git_out(tree, "rev-parse", "--verify", "-q", ref)[0] == 0]


def pushed_commits(tree, remote, branch, sha):
    """(sha, message) of the commits a push adds, oldest first; commits in claude/ryo-consolidation are never included."""
    base = None
    code, out = git_out(tree, "rev-parse", "--verify", "-q", "refs/remotes/" + remote + "/" + branch)
    if code == 0 and out.strip():
        base = out.strip()
    exclude = excluded_refs(tree, remote)
    if base is None:
        if not exclude:
            return []
        base = git_out(tree, "merge-base", sha, exclude[0])[1].strip()
        if not base:
            return []
    args = ["log", "--reverse", "--format=%H%x1f%B%x1e", base + ".." + sha]
    if exclude:
        args += ["--not", *exclude]
    code, out = git_out(tree, *args)
    rows = [row.strip("\n").split("\x1f", 1) for row in out.split("\x1e") if row.strip()]
    return [(row[0].strip(), row[1] if len(row) > 1 else "") for row in rows]


def branch_range(tree, remote, sha):
    """(commits, added files) of the pushed branch's own range: everything the ref has that consolidation lacks."""
    tail = ["--not", *excluded_refs(tree, remote)] if excluded_refs(tree, remote) else []
    commits = set(git_out(tree, "rev-list", sha, *tail)[1].split())
    added = set(git_out(tree, "log", "--diff-filter=A", "--name-only", "--format=", sha, *tail)[1].split("\n"))
    return commits, added


def perf_gate(words, here):
    """A push with a `perf` commit needs a diagnosis added in the pushed range (and, for claimed numbers, an after.json)."""
    push = git_verb(words, "push")
    if push is None:
        return None
    tree = os.path.join(here, push[0]) if push[0] else here
    for remote, source, branch in push_targets(tree, push):
        sha = rev(tree, source)
        if sha is None:
            continue
        perf = [(c, msg) for c, msg in pushed_commits(tree, remote, branch, sha)
                if PERF_SUBJECT.match(msg.lstrip().split("\n", 1)[0])]
        if not perf:
            continue
        found = perf_problem(tree, remote, sha, branch, perf)
        if found:
            return found
    return None


def names_pushed_work(tree, source, sha, commits):
    """True when source is a commit of the pushed range that is sha or has only evidence files changed before sha."""
    found = rev(tree, source) if isinstance(source, str) else None
    return found is not None and found in commits and stands_for(tree, found, sha, EVIDENCE_ROOT)


def perf_problem(tree, remote, sha, branch, perf):
    first = perf[0][0]
    refused = "push of " + branch + " refused: perf commit " + first[:10] + " "
    point = "; run the perf-diagnosis subagent"
    commits, added = branch_range(tree, remote, sha)
    diagnoses = sorted(f for f in added if re.fullmatch(re.escape(PERF_DIR) + r"[^/]+/diagnosis\.json", f))
    problems = []
    for path in diagnoses:
        code, text = git_out(tree, "show", sha + ":" + path)
        try:
            doc = json.loads(text) if code == 0 else None
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
            code, text = git_out(tree, "show", sha + ":" + after)
            try:
                record = json.loads(text) if code == 0 else None
            except ValueError:
                record = None
            if not isinstance(record, dict) or not all(k in record for k in ("before", "after", "identical_output")):
                problems.append("a perf commit claims a number but " + after + " lacks before, after and identical_output")
            elif record["identical_output"] is not True:
                problems.append(after + " has identical_output other than true")
            elif not names_pushed_work(tree, record.get("source"), sha, commits):
                problems.append(after + " has no \"source\" that is the pushed sha, or a commit in the pushed range "
                                "with only " + EVIDENCE_ROOT + " files changed since")
            else:
                return None
    if not diagnoses:
        return refused + "has no " + PERF_DIR + "<iteration>/diagnosis.json added in the pushed range" + point
    return refused + "; ".join(problems) + point


MERGE_VALUE_OPTIONS = {"-m", "-F", "-s", "-X", "--into-name", "--file", "--strategy", "--strategy-option"}
MERGE_CONTROL = {"--abort", "--continue", "--quit"}
CHECKPOINT = re.compile(r"claude/checkpoint-\d+")
SHELL_EXPANDED = re.compile(r"[$`*?{}\\]")


def git_merge(words):
    """The -C directory, operand refs and option words of a `git merge`, or None."""
    found = git_verb(words, "merge")
    if found is None:
        return None
    refs, options, skip = [], [], False
    for w in found[1]:
        if skip:
            skip = False
        elif w in MERGE_VALUE_OPTIONS:
            skip = True
        elif w.startswith("-"):
            options.append(w)
        else:
            refs.append(w)
    return found[0], refs, options


def checkpoint_tree(tree):
    """True on a claude/checkpoint-N branch, or on a detached HEAD sitting at the tip of one."""
    code, current = git_out(tree, "symbolic-ref", "--short", "HEAD")
    if code == 0:
        return CHECKPOINT.fullmatch(current.strip()) is not None
    head = rev(tree, "HEAD")
    if head is None:
        return False
    names = git_out(tree, "for-each-ref", "--points-at", head, "--format=%(refname)", "refs/heads", "refs/remotes")[1]
    return any(CHECKPOINT.fullmatch(re.sub(r"^refs/(heads|remotes/[^/]+)/", "", n)) for n in names.split())


def branch_names(tree, ref, sha):
    """The claude/* branch names whose current head is sha: the ref's own spelling, else every branch there."""
    plain = re.sub(r"^(refs/(heads|remotes/[^/]+)/|[^/]+/(?=claude/))", "", ref)
    if re.fullmatch(r"claude/[^~^@:]+", plain):
        return [plain]
    names = []
    for line in git_out(tree, "for-each-ref", "--points-at", sha, "--format=%(refname)", "refs/heads/claude",
                        "refs/remotes")[1].split():
        name = re.sub(r"^refs/(heads|remotes/[^/]+)/", "", line)
        if name.startswith("claude/") and name not in names:
            names.append(name)
    return names


def branch_review(words, here):
    """In a checkpoint tree a merge needs the leak-review PASS for the branch head merged; pull is refused."""
    pull = git_verb(words, "pull")
    merge = git_merge(words)
    if pull is None and merge is None:
        return None
    tree = os.path.join(here, (pull or merge)[0]) if (pull or merge)[0] else here
    if not checkpoint_tree(tree):
        return None
    if pull is not None:
        return ("git pull is refused in a checkpoint tree: it merges refs this hook cannot check; fetch, then "
                "git merge a claude/* branch that carries its leak-review PASS")
    if any(o in MERGE_CONTROL for o in merge[2]):
        return None
    if not merge[1]:
        return "git merge in a checkpoint tree must name the branch to merge; a bare merge takes its upstream unchecked"
    for ref in merge[1]:
        refused = "merge of " + ref + " refused: "
        if SHELL_EXPANDED.search(ref):
            return refused + "the ref is shell-expanded or a pattern; write the branch name"
        sha = rev(tree, ref)
        if sha is None:
            return refused + "the ref does not resolve to a commit"
        if git_out(tree, "merge-base", "--is-ancestor", sha, "HEAD")[0] == 0:
            continue
        names = branch_names(tree, ref, sha)
        if not names:
            return refused + sha + " is not the current head of any claude/* branch (a moved branch's old sha is stale)"
        if any(INTEGRATION.fullmatch(n) for n in names):
            continue
        name = names[0]
        path = MERGE_BRANCHES + name.replace("/", "-") + ".json"
        run = "; run the leak-review subagent on " + sha + " diff-scoped and commit its PASS"
        refused = "merge of " + name + " refused: "
        code, text = git_out(tree, "show", "HEAD:" + path)
        if code != 0:
            return refused + "HEAD carries no " + path + run
        try:
            review = json.loads(text)
        except ValueError:
            return refused + path + " at HEAD is not JSON" + run
        if not isinstance(review, dict) or review.get("source") != sha:
            return refused + "the source in " + path + " is not the branch head " + sha + run
        if review.get("reviewer") != "leak-review" or "verdict" not in review:
            return refused + path + " lacks \"reviewer\": \"leak-review\" or a \"verdict\"" + run
        if review["verdict"] != "PASS":
            return (refused + path + " has verdict " + repr(review["verdict"]) +
                    "; repair the findings on the branch, then run the leak-review subagent on its new head")
    return None


OPTION_VALUES = {"-u", "-n", "-C", "-S", "--unset", "--chdir"}


def unwrapped(words, depth=0):
    """The command word lists behind env, command, time, nohup, nice, exec, flock and bash/sh/zsh -c wrappers."""
    while True:
        while words and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", words[0]):
            words = words[1:]
        if words and words[0] in WRAPPERS:
            words = words[1:]
            while words and (words[0].startswith("-") or re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*=.*", words[0])):
                words = words[2:] if words[0] in OPTION_VALUES else words[1:]
        elif words and words[0] == "flock":
            rest = words[1:]
            while rest and rest[0].startswith("-"):
                rest = rest[2:] if rest[0] in ("-w", "-E", "--timeout", "--conflict-exit-code") else rest[1:]
            words = rest[1:]
        else:
            break
    inner = shell_command(words) if words else None
    if inner is not None and depth < 4:
        found = []
        for segment in segments(inner):
            found += unwrapped(tokens(segment), depth + 1)
        return found
    return [words]


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
        for found in (stash(words), removal(words), cargo_rules(words, before, dirs), tm_fix(words, clean)):
            if found:
                return found
        for inner in unwrapped(words):
            for found in (bulk_push(inner, here), rust_review(inner, here), previous_reviews(inner, here), perf_gate(inner, here),
                          branch_review(inner, here)):
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
