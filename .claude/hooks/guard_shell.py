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
ASYMPTOTE = EVIDENCE + "asymptote.json"
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


WRAPPERS = {"env", "time", "exec", "nohup", "command", "nice", "timeout", "stdbuf", "xargs", "flock", "ionice",
            "unbuffer", "script", "watch"}
WRAPPER_VALUES = {"env": {"-u", "-n", "-C", "--unset", "--chdir"}, "nice": {"-n", "--adjustment"},
                  "timeout": {"-s", "--signal", "-k", "--kill-after"}, "stdbuf": {"-i", "-o", "-e"},
                  "xargs": {"-n", "-I", "-P", "-L", "-s", "-d", "-E", "-a", "--max-args", "--max-procs", "--delimiter",
                            "--max-lines", "--max-chars", "--arg-file", "--eof"},
                  "flock": {"-w", "-E", "--timeout", "--conflict-exit-code"}, "exec": {"-a"},
                  "ionice": {"-c", "-n", "-p", "-P", "-u"}, "watch": {"-n", "--interval"}, "script": {"-I", "-O", "-B", "-T", "-m"}}
SHELL_KEYWORDS = {"if", "then", "elif", "else", "do", "while", "until", "for", "!", "{", "(", "time"}
ASSIGNMENT = r"[A-Za-z_][A-Za-z0-9_]*=.*"
GATED_WRAPPERS = {"eval", "sudo"}


def after_wrapper(words):
    """words minus the leading wrapper command word and what it consumes: its options and their values, assignments,
    timeout's duration, flock's lock operand. script -c, watch with one quoted command and env -S become sh -c / words."""
    wrapper, words = words[0], words[1:]
    values = WRAPPER_VALUES.get(wrapper, set())
    command = None
    while words and (words[0].startswith("-") or re.fullmatch(ASSIGNMENT, words[0])):
        flag = words[0]
        if wrapper == "env" and flag in ("-S", "--split-string") and len(words) > 1:
            return tokens(words[1]) + words[2:]
        if wrapper == "script" and flag in ("-c", "--command") and len(words) > 1:
            command = words[1]
        words = words[2:] if flag in values or (wrapper == "script" and flag in ("-c", "--command")) else words[1:]
    if wrapper == "script":
        return ["sh", "-c", command] if command is not None else []
    if wrapper in ("timeout", "flock") and words:
        words = words[1:]
    if wrapper == "watch" and len(words) == 1 and " " in words[0]:
        return ["sh", "-c", words[0]]
    return words


def peel(words):
    """The command word list behind leading assignments, shell keywords and grouping, and every wrapper (the one
    unwrapping every caller uses). It is always a suffix of words."""
    while words:
        if re.fullmatch(ASSIGNMENT, words[0]) or words[0] in SHELL_KEYWORDS and words[0] != "time":
            words = words[1:]
        elif words[0] in WRAPPERS:
            words = after_wrapper(words)
        else:
            break
    return words


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
    words = peel(words)
    if not words:
        return False
    inner = shell_command(words)
    if inner is not None:
        return has_cargo_command(inner)
    if words[0] == "cargo":
        rest = [w for w in words[1:] if not w.startswith("+")]
        return next((w for w in rest if not w.startswith("-")), "") in CARGO_BUILDS
    return False


def has_cargo_command(text):
    return any(runs_cargo_verb(tokens(segment)) for segment in segments(text))


REDIRECT = re.compile(r"^(\d*|&)[<>]")


def without_redirections(words):
    """Drop shell redirections (2>&1, >file, 2>/dev/null, and a bare operator with its target)."""
    out, i = [], 0
    while i < len(words):
        if REDIRECT.match(words[i]):
            bare = re.fullmatch(r"(\d*|&)(>>|>|<<<|<<-|<<|<)", words[i]) is not None
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
    while found and (re.fullmatch(ASSIGNMENT, found[0]) or found[0] in SHELL_KEYWORDS and found[0] != "time"):
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
    """Index of `cargo` when the segment runs it directly, behind any wrappers."""
    inner = peel(words)
    return len(words) - len(inner) if inner and inner[0] == "cargo" else None


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
        elif words[i] in ("-c", "--git-dir", "--work-tree", "--namespace", "--config-env") and i + 1 < len(words):
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


PUSH_OPTION_VALUES = {"-o", "--push-option", "--receive-pack", "--exec", "--repo", "--recurse-submodules"}


def push_targets(tree, push):
    """(remote, source, destination) for each refspec a push names; no refspec pushes HEAD to the current branch."""
    operands, skip = [], False
    for a in push[1]:
        if skip:
            skip = False
        elif a in PUSH_OPTION_VALUES:
            skip = True
        elif not a.startswith("-"):
            operands.append(a)
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


def full_sha(value):
    """True for a 40-character hex sha; evidence names its source this way, never by ref name."""
    return isinstance(value, str) and re.fullmatch(r"[0-9a-f]{40}", value) is not None


def service_module():
    """ci/contract_review/service.py, loaded by path (stdlib only): the one validator of review evidence."""
    import importlib.util
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "ci", "contract_review", "service.py")
    spec = importlib.util.spec_from_file_location("contract_review_service", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validated(call):
    """None when call(service module) passes, else the problem text; a missing or old service.py refuses."""
    try:
        call(service_module())
    except ValueError as error:
        return str(error)
    except Exception as error:  # a missing or old service.py must refuse, never pass
        return "cannot be validated: ci/contract_review/service.py failed to load or lacks the validator (" + \
            type(error).__name__ + ": " + str(error) + ")"
    return None


def review_shape(review, reviewer):
    """The problem with a review evidence file against the shared schema, or None.

    The one definition is shape() in ci/contract_review/service.py, loaded by path (stdlib only), so this hook,
    run with python3 -I, and the contract-review CI job cannot drift. The schema is
    crates/ryo/docs/contracts/review/README.md#review-evidence-schema.
    """
    return validated(lambda module: module.shape(review, reviewer))


def candidate_problem(tree, sha, head, leak):
    """The refusal text when the leak-review.json (parsed) or a branch result carries candidates the previous
    checkpoint's tm-steward.json did not decide, else None. Candidates come from that checkpoint's leak-review.json
    and the evidence/branches/*.json files added or modified since the previous landed head; decisions never carry forward."""
    reviews = [(EVIDENCE + "leak-review.json", leak)]
    code, listing = git_out(tree, "diff", "--name-status", "--no-renames", head + ".." + sha, "--", MERGE_BRANCHES)
    for line in (listing.split("\n") if code == 0 else []):
        status, _, name = line.partition("\t")
        if status not in ("A", "M") or not name.endswith(".json"):
            continue
        code, text = git_out(tree, "show", sha + ":" + name)
        try:
            reviews.append((name, json.loads(text)))
        except ValueError:
            return name + " at " + sha[:10] + " is not JSON"
    ids, plants = [], {}
    for name, review in reviews:
        if isinstance(review, dict) and "enforcement_candidates" in review:
            problem = validated(lambda module: module.candidates(review["enforcement_candidates"]))
            if problem:
                return name + " " + problem
            ids += [(name, i) for i in service_ids(review)]
            plants.update(service_ids(review))
    code, text = git_out(tree, "show", sha + ":" + EVIDENCE + "tm-steward.json")
    steward = json.loads(text) if code == 0 else {}
    left = []
    problem = validated(lambda module: left.extend(module.undecided(steward, [i for _, i in ids])))
    if problem:
        return problem
    wrong = []
    problem = validated(lambda module: wrong.extend(module.plant_mismatch(steward, plants)))
    if problem:
        return problem
    if wrong:
        return "tm-steward.json plant_counts do not number the plants of " + ", ".join(wrong) + " one for one"
    if left:
        where = {i: n for n, i in ids}
        return ("tm-steward.json decides no candidate_decisions entry for " +
                ", ".join(i + " (from " + where[i] + ")" for i in left))
    return None


def service_ids(review):
    out = {}
    validated(lambda module: out.update(module.candidate_ids(review)))
    return out


def bulk_push(words, here):
    """git push --all / --mirror would push refs no gate has looked at; this repository refuses them."""
    push = git_verb(words, "push")
    if push is not None and any(a in ("--all", "--mirror", "--branches") for a in push[1]):
        return "git push --all/--mirror/--branches pushes refs no gate checked; push each branch by name"
    if push is not None:
        for operand in (a for a in push[1] if not a.startswith("-")):
            if SHELL_EXPANDED.search(operand) or operand.lstrip("+") == ":":
                return ("git push operand " + operand + " is shell-expanded, a pattern or the matching refspec; "
                        "write the branch name")
    return None


def gated_wrapper(words, here):
    """eval and sudo are not unwrapped: they refuse in front of a gated git verb rather than hide it from the gates."""
    words = peel(words)
    if words and words[0] in GATED_WRAPPERS and gated_text(" ".join(words[1:])):
        return (words[0] + " in front of a gated git push, merge or pull hides it from the guards; "
                "run the git command directly")
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
        shape = review_shape(review, "rust-review")
        if shape:
            return refused + RUST_REVIEW + " at " + source + " " + shape + "; run the rust-review subagent again"
        if review.get("verdict") != "PASS":
            verdict = review.get("verdict")
            return (refused + RUST_REVIEW + " at " + source + " has verdict " + repr(verdict) +
                    ", not \"PASS\"; repair its findings and run the rust-review subagent again, or the owner "
                    "overrules and pushes")
        reviewed = review.get("source")
        if not full_sha(reviewed):
            return refused + RUST_REVIEW + " at " + source + " names no reviewed \"source\" as a full 40-character sha"
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
            shape = review_shape(review, agent)
            if shape:
                return refused + EVIDENCE + name + " at " + source + " " + shape + "; " + run
            reviewed = review.get("source")
            if not full_sha(reviewed) or \
                    not stands_for(tree, reviewed, head, EVIDENCE):
                return (refused + "the source " + repr(reviewed) + " in " + EVIDENCE + name +
                        " is not the previous landed head " + head +
                        " (or an ancestor with only evidence changed since); " + run)
            if name == "leak-review.json":
                leak = review
        if len(needed) == len(PREVIOUS_REVIEWS):
            problem = candidate_problem(tree, sha, head, leak)
            if problem:
                return (refused + problem + "; the tm-steward must decide every enforcement candidate of " + head +
                        " (decision compiler|strengthen|new with plant counts of at least 1, or none with a reason)")
        code, text = git_out(tree, "show", sha + ":" + ASYMPTOTE)
        if code == 0:
            run = "rerun the integrator's quick checks on " + head
            try:
                shape = review_shape(json.loads(text), "integrator")
            except ValueError:
                shape = "is not JSON"
            if shape:
                return refused + ASYMPTOTE + " at " + source + " " + shape + "; " + run
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
    found = rev(tree, source) if full_sha(source) else None
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
        if not full_sha(source) or \
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
        if not isinstance(review, dict) or not full_sha(review.get("source")) or review.get("source") != sha:
            return refused + "the source in " + path + " is not the branch head " + sha + run
        shape = review_shape(review, "leak-review")
        if shape:
            return refused + path + " " + shape + run
        if review["verdict"] != "PASS":
            return (refused + path + " has verdict " + repr(review["verdict"]) +
                    "; repair the findings on the branch, then run the leak-review subagent on its new head")
    return None




def unwrapped(words, depth=0):
    """The command word lists behind every wrapper (peel) and bash/sh/zsh -c strings."""
    words = peel(words)
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


GATED_TEXT = re.compile(r"\bgit\b[\s\S]*\b(push|merge|pull)\b")


def gated_text(command):
    """True when the command, with quotes and backslashes removed, names git push, merge or pull; cannot raise."""
    plain = str(command).replace("'", "").replace('"', "").replace("\\", "")
    return GATED_TEXT.search(plain) is not None
GATES = (gated_wrapper, bulk_push, rust_review, previous_reviews, perf_gate, branch_review)


EVIDENCE_NAME = re.compile(r"(rust-review|contract-review|leak-review|tm-steward)[^/]*\.json|asymptote\.json")
EVIDENCE_TAIL = "/crates/ryo/docs/contracts/evidence/"
CHECKPOINT_BRANCH = re.compile(r"claude/checkpoint-\d+")
WRITE_REDIRECT = re.compile(r"(?<![<&0-9])\d?>>?\s*(?!&)(\"[^\"]+\"|'[^']+'|[^\s;&|<>()]+)")


def evidence_write_problem(path, here):
    """The refusal when path is a review-evidence file outside a claude/checkpoint-N worktree's evidence directory.

    Evidence files are the reviewers' committed record, so they are written straight into the open checkpoint
    integration worktree; a scratchpad copy is never read by the push hooks. Shared by the Write/Edit hook.
    """
    expanded = os.path.expandvars(os.path.expanduser(path))
    unresolved = re.search(r"\$[A-Za-z_{]", expanded) is not None
    full = os.path.normpath(os.path.join(here, expanded))
    parts = full.split("/")
    in_branches = any(a == "evidence" and b == "branches" for a, b in zip(parts, parts[1:-1]))
    if parts[-1].endswith("-committed-copy.json") and not in_branches:
        return None  # a reviewer's read-only copy of committed evidence
    if not (EVIDENCE_NAME.fullmatch(parts[-1]) or in_branches):
        return None
    if unresolved and re.match(r"\$\{?[A-Za-z_]\w*\}?/", expanded) and EVIDENCE_TAIL in full:
        return None  # a variable-rooted path into an evidence directory; the root cannot be checked here
    at = full.find(EVIDENCE_TAIL)
    if at >= 0:
        root = full[:at] or "/"
        done = subprocess.run(["git", "-C", root, "symbolic-ref", "--short", "HEAD"],
                              capture_output=True, text=True, check=False)
        if done.returncode == 0 and CHECKPOINT_BRANCH.fullmatch(done.stdout.strip()):
            return None
    return (full + " is review evidence outside a claude/checkpoint-N worktree's crates/ryo/docs/contracts/evidence/; "
            "write it into <INTEGRATION_TREE>/crates/ryo/docs/contracts/evidence/, never a scratchpad")


def written_paths(segment, words):
    """The files a shell segment writes by > , >>, tee, cp, mv or install (a copy into a directory counts the
    source's file name under that directory)."""
    found = [m.group(1).strip("'\"") for m in WRITE_REDIRECT.finditer(segment)]
    if len(words) > 2 and words[0] == "git" and "show" in words[1:3] and any(re.match(r"[^\s:]+:\S", w) for w in words[2:]):
        found = []  # git show <rev>:<committed path> > file is a read copy of committed content
    for inner in unwrapped(words):
        if not inner:
            continue
        operands = [a for a in inner[1:] if not a.startswith("-")]
        if inner[0] == "tee":
            found += operands
        elif inner[0] in ("sed", "perl") and any(re.fullmatch(r"-[A-Za-z]*i\S*", a) for a in inner[1:]) or "--in-place" in inner:
            found += operands
        elif inner[0] == "sponge":
            found += operands
        elif inner[0] == "dd":
            found += [a[3:] for a in inner[1:] if a.startswith("of=")]
        elif inner[0] in ("python", "python3") and "-c" in inner:
            code = inner[inner.index("-c") + 1] if inner.index("-c") + 1 < len(inner) else ""
            if re.search(r"open\([^)]*['\"][wa]b?\+?['\"]|json\.dump|write_text|write_bytes", code):
                found += [m.group(2) for m in re.finditer(r"(['\"])([^'\"]+)\1", code)]
        elif inner[0] in ("cp", "mv", "install", "rsync", "ln") and operands:
            target = operands[-1]
            if "-t" in inner or any(a.startswith("--target-directory") for a in inner):
                for i, a in enumerate(inner):
                    if a == "-t" and i + 1 < len(inner):
                        target = inner[i + 1]
                    elif a.startswith("--target-directory="):
                        target = a.split("=", 1)[1]
                sources = operands
            else:
                sources = operands[:-1]
            found += [target] + [os.path.join(target, os.path.basename(src)) for src in sources]
    return found


def evidence_writes(segment, words, here):
    for path in written_paths(segment, words):
        found = evidence_write_problem(path, here)
        if found:
            return found
    return None


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
        for found in (stash(words), removal(words), cargo_rules(words, before, dirs), tm_fix(words, clean), evidence_writes(segment, words, here)):
            if found:
                return found
        for inner in unwrapped(words):
            for gate in GATES:
                found = gate(inner, here)
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
    try:
        found = reason(command, data.get("cwd"))
    except Exception as error:
        # Claude Code treats a hook crash as non-blocking, so a guard that breaks on a git push, merge or pull
        # must refuse it. The scan is plain text and cannot raise; any other command keeps the crash.
        if gated_text(command):
            print("Blocked: guard failed closed: " + type(error).__name__ + ": " + str(error), file=sys.stderr)
            return 2
        raise
    if found:
        print("Blocked: " + found, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
