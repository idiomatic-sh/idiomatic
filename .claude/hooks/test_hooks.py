"""Tests for the guard_shell, acceptance_on_stop, no_subagent_builders and check_in_on_stop hooks, run with `python3 -I -m unittest`."""

import json
import os
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
LOCK = "flock /workspaces/.cargo-build.lock "
CHECK = " --target-dir /workspaces/idiomatic-check-target"


def run_hook(name, payload, env=None):
    return subprocess.run(["python3", "-I", os.path.join(HERE, name)], input=json.dumps(payload),
                          capture_output=True, text=True, env=env, check=False,
                          cwd=tempfile.gettempdir())


def shell(command, cwd=None):
    payload = {"tool_name": "Bash", "tool_input": {"command": command}}
    if cwd:
        payload["cwd"] = cwd
    return run_hook("guard_shell.py", payload).returncode


def repo(parent, name, branch):
    path = os.path.join(parent, name)
    os.mkdir(path)
    subprocess.run(["git", "-C", path, "init", "-q", "-b", branch], check=True)
    return path


class GuardShell(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.dir = tempfile.TemporaryDirectory()
        cls.integration = repo(cls.dir.name, "integration", "claude/checkpoint-7")
        cls.builder = repo(cls.dir.name, "builder", "claude/p1-coverage-kinds")

    @classmethod
    def tearDownClass(cls):
        cls.dir.cleanup()

    def test_cargo_only_in_integration_trees(self):
        self.assertEqual(shell(LOCK + "cargo test -p x", self.integration), 0)
        self.assertEqual(shell(LOCK + "cargo test -p x", self.builder), 2)
        self.assertEqual(shell(LOCK + "cargo test -p x"), 2)
        self.assertEqual(shell("cd " + self.builder + " && " + LOCK + "cargo build", self.integration), 2)
        self.assertEqual(shell(LOCK + "cargo build --manifest-path " + self.builder + "/Cargo.toml", self.integration), 2)
        self.assertEqual(shell(LOCK + "cargo build -C " + self.integration, self.builder), 2)
        self.assertEqual(shell(LOCK + "cargo build", "/workspaces/ryo-perf-x"), 0)

    def test_hidden_cargo_is_blocked_in_a_builder_tree(self):
        for command in ['bash -c "cargo test"', "echo hi; cargo build", "sh -c 'flock x cargo clippy'"]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)

    def test_cargo_words_inside_quoted_arguments_are_not_commands(self):
        for command in ["bd update ryo-1 --append-notes 'the cargo check JSON; cargo test output'",
                        'git commit -m "run cargo test; then cargo build"', "echo cargo build", "grep 'cargo clippy' log"]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 0)

    def test_cargo_after_a_separator_is_blocked(self):
        for command in ["x; cargo test", "x && cargo build", "x | cargo run", "(cargo check)", "echo $(cargo doc)",
                        "x\ncargo bench", "bash -c 'x; cargo test'", LOCK + "cargo clippy"]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)

    def test_builder_may_check_one_package_into_the_check_target(self):
        crate = os.path.join(self.builder, "crate")
        os.makedirs(crate, exist_ok=True)
        with open(os.path.join(crate, "Cargo.toml"), "w", encoding="utf-8") as handle:
            handle.write("[package]\nname = \"crate\"\n")
        with open(os.path.join(self.builder, "Cargo.toml"), "w", encoding="utf-8") as handle:
            handle.write("[workspace]\nmembers = [\"crate\"]\n")
        for command in ["cargo check -p x" + CHECK, "cargo check --package=x --all-features" + CHECK,
                        "cargo check --manifest-path crate/Cargo.toml" + CHECK, "cd crate && cargo check -px" + CHECK,
                        LOCK + "cargo check -p x" + CHECK]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 0)

    def test_builder_check_without_the_check_target_is_refused(self):
        for command in ["cargo check -p x", LOCK + "cargo check -p x",
                        "cargo check -p x --target-dir /workspaces/idiomatic-benchmark-guards/target",
                        LOCK + "cargo check -p x --target-dir=/workspaces/idiomatic-benchmark-guards/target",
                        "cargo check -p x --target-dir /tmp/other"]:
            with self.subTest(command=command):
                result = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.builder})
                self.assertEqual(result.returncode, 2)
                self.assertIn("cargo check -p <crate> --target-dir /workspaces/idiomatic-check-target", result.stderr)

    def test_builder_check_per_worktree_target_passes(self):
        self.assertEqual(shell("cargo check -p x --target-dir /workspaces/idiomatic-check-target/ryo-w5", self.builder), 0)

    def test_builder_check_two_component_target_is_refused(self):
        for target in ["a/b", "", "..", "a/"]:
            with self.subTest(target=target):
                command = "cargo check -p x --target-dir /workspaces/idiomatic-check-target/" + target
                result = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.builder})
                self.assertEqual(result.returncode, 2)
                self.assertIn("/workspaces/idiomatic-check-target/<worktree name>", result.stderr)

    def test_builder_workspace_wide_check_is_refused(self):
        for command in ["cargo check" + CHECK, "cargo check --workspace -p x" + CHECK, "cargo check --all" + CHECK,
                        "cargo check --manifest-path Cargo.toml" + CHECK, "cargo check --manifest-path nope/Cargo.toml" + CHECK]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)

    def test_builder_test_build_clippy_with_a_package_are_refused(self):
        for command in ["cargo test -p x" + CHECK, "cargo build -p x" + CHECK, "cargo clippy -p x" + CHECK,
                        LOCK + "cargo test -p x", "bash -c 'cargo check -p x" + CHECK + "'"]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)

    def test_integration_tree_forms_are_unchanged(self):
        for command, code in [(LOCK + "cargo check --workspace --all-features --message-format=json", 0),
                              ("cargo check --workspace", 2), (LOCK + "cargo test --workspace", 0),
                              ("cargo check -p x" + CHECK, 0), ("cargo build -p x" + CHECK, 2),
                              (LOCK + "cargo check --target-dir /tmp/other", 2)]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.integration), code)

    def test_each_tree_builds_into_its_own_target(self):
        for command in [LOCK + "cargo test -p x --target-dir /workspaces/idiomatic-target/integration",
                        LOCK + "cargo build --target-dir=/workspaces/idiomatic-target/integration/"]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.integration), 0)

    def test_another_trees_target_and_the_shared_target_are_refused(self):
        for target in ["/workspaces/idiomatic-target/builder", "/workspaces/idiomatic-target",
                       "/workspaces/idiomatic-target/integration/x", "/workspaces/idiomatic-benchmark-guards/target"]:
            with self.subTest(target=target):
                result = run_hook("guard_shell.py", {"tool_input": {"command": LOCK + "cargo test --target-dir " + target},
                                                     "cwd": self.integration})
                self.assertEqual(result.returncode, 2)
                self.assertIn("(here /workspaces/idiomatic-target/integration)", result.stderr)

    def test_measurement_trees_build_only_into_their_own_target(self):
        self.assertEqual(shell(LOCK + "cargo build --target-dir /workspaces/idiomatic-target/ryo-perf-x",
                               "/workspaces/ryo-perf-x"), 0)
        self.assertEqual(shell(LOCK + "cargo build --target-dir /workspaces/idiomatic-target/ryo-perf-y",
                               "/workspaces/ryo-perf-x"), 2)
        self.assertEqual(shell(LOCK + "cargo build --target-dir /workspaces/idiomatic-benchmark-guards/target",
                               "/workspaces/ryo-perf-x"), 2)

    def test_hidden_cargo_is_allowed_in_an_integration_tree(self):
        self.assertEqual(shell(LOCK + 'bash -c "cargo test"', self.integration), 0)

    def test_metadata_tree_version_and_tm_are_allowed_everywhere(self):
        for command in ["cargo metadata", "cargo tree", "cargo --version", "tm check"]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 0)

    def test_blocks(self):
        blocked = [
            "git stash", "git stash pop", "cd x && git stash",
            "while pgrep -f cargo; do sleep 1; done", "until pkill -f foo; do :; done",
            "rm -rf /workspaces/idiomatic-*", "rm -r /tmp/ryo-*", "rm -rf build/?x",
            "cargo test -p x", "cargo +nightly clippy", "echo hi; cargo build",
            "tm check --fix", "tm lint check --fix",
            LOCK + "cargo test --target-dir /tmp/other",
            LOCK + "cargo build --target-dir=/tmp/other",
        ]
        for command in blocked:
            with self.subTest(command=command):
                self.assertEqual(shell(command), 2)

    def test_allows(self):
        cwd = self.integration
        allowed = [
            "git stash list", "git stash push -u -m wip", "git stash apply abc123", "git stash drop", "git stash show",
            "while pgrep -x cargo; do sleep 1; done", "pgrep -f cargo", "# git stash",
            "rm -rf /tmp/ryo-exact-dir", "rm -rf /workspaces/ryo-hooks/build", "rm file*.txt",
            "cargo metadata", "cargo --version", "cargo tree",
            LOCK + "cargo test -p x", "cd . && " + LOCK + "cargo build",
            LOCK + "cargo test --target-dir /workspaces/idiomatic-target/integration",
            "tm check", "tm check --fix cli/tests/replay/imports.rs", "ls",
        ]
        for command in allowed:
            with self.subTest(command=command):
                self.assertEqual(shell(command, cwd), 0)


def git(path, *args):
    return subprocess.run(["git", "-C", path, "-c", "user.name=t", "-c", "user.email=t@t", *args],
                          capture_output=True, text=True, check=True).stdout.strip()


class RustReviewPush(unittest.TestCase):
    REVIEW = "crates/ryo/docs/contracts/evidence/checkpoint/rust-review.json"

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.tree = repo(self.dir.name, "tree", "claude/checkpoint-9")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "base")
        self.base = git(self.tree, "rev-parse", "HEAD")

    def commit_review(self, verdict, source):
        path = os.path.join(self.tree, self.REVIEW)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            json.dump({"checkpoint": 9, "source": source, "verdict": verdict, "items": [],
                       "reviewer": "rust-principal-review", "date": "2026-11-07"}, handle)
        git(self.tree, "add", self.REVIEW)
        git(self.tree, "commit", "-q", "-m", "review")

    def push(self, command="git push origin HEAD:claude/ryo-consolidation"):
        return run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})

    def test_refused_without_the_review(self):
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("carries no " + self.REVIEW, result.stderr)

    def test_refused_on_fail(self):
        self.commit_review("FAIL", self.base)
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("verdict 'FAIL'", result.stderr)

    def test_refused_when_source_is_not_an_ancestor(self):
        git(self.tree, "checkout", "-q", "-b", "side")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "elsewhere")
        elsewhere = git(self.tree, "rev-parse", "HEAD")
        git(self.tree, "checkout", "-q", "claude/checkpoint-9")
        self.commit_review("PASS", elsewhere)
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("not an ancestor of", result.stderr)

    def test_refused_when_not_json(self):
        path = os.path.join(self.tree, self.REVIEW)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write("PASS")
        git(self.tree, "add", self.REVIEW)
        git(self.tree, "commit", "-q", "-m", "review")
        self.assertEqual(self.push().returncode, 2)

    def test_allowed_with_pass_and_an_ancestor_source(self):
        self.commit_review("PASS", self.base)
        for command in ["git push origin HEAD:claude/ryo-consolidation", "git push origin HEAD:claude/ryo-consolidation",
                        "git push -q origin +HEAD:refs/heads/claude/ryo-consolidation",
                        "cd . && git -C . push origin HEAD:claude/ryo-consolidation"]:
            with self.subTest(command=command):
                self.assertEqual(self.push(command).returncode, 0)

    def test_every_refspec_form_is_guarded(self):
        for command in ["git push origin HEAD:claude/ryo-consolidation", "git push -u origin HEAD:claude/ryo-consolidation",
                        "git push origin +x:refs/heads/claude/ryo-consolidation", "x && git push origin HEAD:claude/ryo-consolidation",
                        "git -C " + self.tree + " push origin HEAD:claude/ryo-consolidation"]:
            with self.subTest(command=command):
                self.assertEqual(run_hook("guard_shell.py", {"tool_input": {"command": command}}).returncode
                                 if command.startswith("git -C") else self.push(command).returncode, 2)

    def test_other_branches_are_unaffected(self):
        for command in ["git push origin claude/checkpoint-9", "git push -u origin claude/w4-rust-reviewer",
                        "git push", "git push origin HEAD:claude/ryo-consolidation-notes", "git log claude/ryo-consolidation",
                        "git commit -m 'git push origin HEAD:claude/ryo-consolidation'"]:
            with self.subTest(command=command):
                self.assertEqual(self.push(command).returncode, 0)


class PreviousReviewsPush(unittest.TestCase):
    EVIDENCE = "crates/ryo/docs/contracts/evidence/checkpoint/"
    NAMES = ["contract-review.json", "leak-review.json", "tm-steward.json"]

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.remote = os.path.join(self.dir.name, "remote.git")
        subprocess.run(["git", "init", "-q", "--bare", "-b", "claude/ryo-consolidation", self.remote], check=True)
        self.tree = repo(self.dir.name, "tree", "claude/checkpoint-9")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "previous landed head")
        self.previous = git(self.tree, "rev-parse", "HEAD")
        git(self.tree, "remote", "add", "origin", self.remote)
        git(self.tree, "push", "-q", "origin", "HEAD:claude/ryo-consolidation")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "checkpoint work")
        self.write("rust-review.json", {"verdict": "PASS", "source": git(self.tree, "rev-parse", "HEAD")})

    def write(self, name, body):
        path = os.path.join(self.tree, self.EVIDENCE, name)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(body, handle)
        git(self.tree, "add", path)
        git(self.tree, "commit", "-q", "-m", "evidence " + name)

    def all_reviews(self, source=None, skip=()):
        for name in self.NAMES:
            if name not in skip:
                self.write(name, {"verdict": "BLOCKING", "source": source or self.previous})

    def push(self):
        return run_hook("guard_shell.py", {"tool_input": {"command": "git push origin HEAD:claude/ryo-consolidation"},
                                           "cwd": self.tree})

    def test_allowed_with_all_three_even_when_blocking(self):
        self.all_reviews()
        self.assertEqual(self.push().returncode, 0)

    def test_each_missing_review_is_refused_by_name(self):
        for name in self.NAMES:
            with self.subTest(name=name):
                self.setUp()
                self.all_reviews(skip=(name,))
                result = self.push()
                self.assertEqual(result.returncode, 2)
                self.assertIn("carries no " + self.EVIDENCE + name, result.stderr)
                self.assertIn("subagent", result.stderr)

    def test_stale_source_is_refused(self):
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "elsewhere")
        stale = git(self.tree, "rev-parse", "HEAD")
        self.all_reviews(source=stale)
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("is not the previous landed head", result.stderr)

    def test_missing_verdict_is_refused(self):
        self.all_reviews()
        self.write("leak-review.json", {"source": self.previous})
        self.assertIn('no "verdict"', self.push().stderr)

    def test_bootstrap_head_needs_only_the_contract_review(self):
        # the bootstrap sha cannot be forged, so point the constant at this remote head
        import importlib.util
        spec = importlib.util.spec_from_file_location("guard_shell", os.path.join(HERE, "guard_shell.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.BOOTSTRAP_HEAD = self.previous[:10]
        words = ["git", "push", "origin", "HEAD:claude/ryo-consolidation"]
        self.assertIn("contract-review.json", module.previous_reviews(words, self.tree))
        self.write("contract-review.json", {"verdict": "PASS", "source": self.previous})
        self.assertIsNone(module.previous_reviews(words, self.tree))


class PerfGate(unittest.TestCase):
    DIR = "crates/ryo/docs/contracts/evidence/perf/p6/"

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.tree = repo(self.dir.name, "tree", "claude/ryo-consolidation")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "base")
        self.base = git(self.tree, "rev-parse", "HEAD")
        git(self.tree, "checkout", "-q", "-b", "claude/p6-speed")

    def write(self, name, body):
        path = os.path.join(self.tree, self.DIR + name)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(body, handle)
        git(self.tree, "add", self.DIR + name)
        git(self.tree, "commit", "-q", "-m", "docs: evidence " + name)

    def diagnosis(self, source=None, verdict="refuted", levers=("l1",)):
        self.write("diagnosis.json", {"source": source or self.base, "hypotheses": [{"claim": "lock", "verdict": verdict}],
                                      "levers": [{"id": l, "evidence": "e"} for l in levers]})

    def perf(self, message="perf(engine): cache facts (lever l1)"):
        git(self.tree, "commit", "-q", "--allow-empty", "-m", message)

    def push(self):
        return run_hook("guard_shell.py", {"tool_input": {"command": "git push origin claude/p6-speed"}, "cwd": self.tree})

    def test_redirections_are_not_push_operands(self):
        self.perf()
        for tail in (" 2>&1", " 2>/dev/null", " >out.log", " > out.log 2>&1", " >> out.log"):
            with self.subTest(tail=tail):
                done = run_hook("guard_shell.py", {"tool_input": {"command": "git push origin claude/p6-speed" + tail},
                                                   "cwd": self.tree})
                self.assertEqual(done.returncode, 2)
                self.assertIn("push of claude/p6-speed refused", done.stderr)

    def test_remote_consolidation_perf_commits_are_not_pushed(self):
        git(self.tree, "checkout", "-q", "-b", "claude/other", self.base)
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "perf(engine): remote only 3s")
        git(self.tree, "update-ref", "refs/remotes/origin/claude/ryo-consolidation", "HEAD")
        git(self.tree, "checkout", "-q", "claude/p6-speed")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "fix: unrelated")
        self.assertEqual(self.push().returncode, 0)

    def test_stale_diagnosis_from_an_older_iteration_is_refused(self):
        git(self.tree, "checkout", "-q", "claude/ryo-consolidation")
        self.DIR = "crates/ryo/docs/contracts/evidence/perf/p5/"
        self.diagnosis()
        self.DIR = "crates/ryo/docs/contracts/evidence/perf/p6/"
        git(self.tree, "checkout", "-q", "claude/p6-speed")
        git(self.tree, "merge", "-q", "--ff-only", "claude/ryo-consolidation")
        self.perf()
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("added in the pushed range", result.stderr)

    def test_after_must_be_identical_and_name_a_commit_in_range(self):
        self.diagnosis()
        self.perf("perf(engine): cache facts (l1), 12s to 8s")
        head = git(self.tree, "rev-parse", "HEAD")
        for body in ({"identical_output": False, "source": head}, {"identical_output": True},
                     {"identical_output": True, "source": self.base}):
            with self.subTest(body=body):
                self.write("after.json", {"before": [], "after": [], **body})
                self.assertEqual(self.push().returncode, 2)

    def test_the_pushed_ref_is_checked_not_head(self):
        self.perf()
        git(self.tree, "checkout", "-q", "claude/ryo-consolidation")
        for command in ["git push origin claude/p6-speed", "git push origin claude/p6-speed:claude/p6-speed"]:
            with self.subTest(command=command):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)

    def test_bare_push_and_head_push_check_the_current_branch(self):
        self.perf()
        for command in ["git push", "git push origin", "git push origin HEAD", "git push -u origin HEAD"]:
            with self.subTest(command=command):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)

    def test_wrappers_do_not_hide_the_push(self):
        self.perf()
        for command in ["env git push origin claude/p6-speed", "command git push origin claude/p6-speed",
                        "env -i A=1 git push origin claude/p6-speed", "bash -c 'git push origin claude/p6-speed'",
                        "sh -c \"env git push origin claude/p6-speed\"", "nohup git push origin claude/p6-speed"]:
            with self.subTest(command=command):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)

    def test_no_perf_commit_passes(self):
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "fix: 3s typo")
        self.assertEqual(self.push().returncode, 0)

    def test_perf_without_diagnosis_is_refused(self):
        self.perf()
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("perf-diagnosis subagent", result.stderr)
        self.assertIn("diagnosis.json", result.stderr)

    def test_diagnosis_without_verdicts_is_refused(self):
        self.diagnosis(verdict="maybe")
        self.perf()
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("no hypothesis with a verdict", result.stderr)

    def test_diagnosis_without_levers_is_refused(self):
        self.diagnosis(levers=())
        self.perf()
        self.assertIn("no lever", self.push().stderr)

    def test_stale_source_is_refused(self):
        git(self.tree, "checkout", "-q", "-b", "side", self.base)
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "elsewhere")
        elsewhere = git(self.tree, "rev-parse", "HEAD")
        git(self.tree, "checkout", "-q", "claude/p6-speed")
        self.diagnosis(source=elsewhere)
        self.perf()
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("not an ancestor of the first perf commit", result.stderr)

    def test_claimed_number_without_after_is_refused(self):
        self.diagnosis()
        self.perf("perf(engine): cache facts (l1), 12s to 8s")
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("after.json", result.stderr)

    def test_all_present_passes(self):
        self.diagnosis()
        self.perf("perf(engine): cache facts (l1), 12s to 8s")
        self.write("after.json", {"before": [], "after": [], "identical_output": True,
                                  "source": git(self.tree, "rev-parse", "HEAD~1")})
        self.assertEqual(self.push().returncode, 0)

    def test_number_free_perf_needs_only_the_diagnosis(self):
        self.diagnosis()
        self.perf()
        self.assertEqual(self.push().returncode, 0)

    def test_commits_already_in_consolidation_are_not_rechecked(self):
        git(self.tree, "checkout", "-q", "claude/ryo-consolidation")
        self.perf()
        git(self.tree, "checkout", "-q", "claude/p6-speed")
        git(self.tree, "merge", "-q", "--ff-only", "claude/ryo-consolidation")
        self.assertEqual(self.push().returncode, 0)


class ConsolidationRef(unittest.TestCase):
    REVIEW = RustReviewPush.REVIEW

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.tree = repo(self.dir.name, "tree", "claude/ryo-consolidation")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "base")

    def run_in(self, command):
        return run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})

    def test_bare_and_head_pushes_from_consolidation_are_gated(self):
        for command in ["git push", "git push origin", "git push origin HEAD", "env git push origin HEAD",
                        "bash -c 'git push origin HEAD'"]:
            with self.subTest(command=command):
                done = self.run_in(command)
                self.assertEqual(done.returncode, 2)
                self.assertIn("rust-review subagent", done.stderr)

    def test_the_pushed_ref_is_gated_not_head(self):
        git(self.tree, "checkout", "-q", "-b", "claude/checkpoint-9")
        path = os.path.join(self.tree, self.REVIEW)
        os.makedirs(os.path.dirname(path))
        with open(path, "w", encoding="utf-8") as handle:
            json.dump({"verdict": "PASS", "source": git(self.tree, "rev-parse", "HEAD")}, handle)
        git(self.tree, "add", self.REVIEW)
        git(self.tree, "commit", "-q", "-m", "review")
        self.assertEqual(self.run_in("git push origin claude/ryo-consolidation").returncode, 2)
        self.assertEqual(self.run_in("git push origin HEAD:claude/ryo-consolidation").returncode, 0)

    def test_previous_review_block_names_its_subagents(self):
        import importlib.util
        spec = importlib.util.spec_from_file_location("guard_shell", os.path.join(HERE, "guard_shell.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertEqual([agent for _, agent in module.PREVIOUS_REVIEWS], ["contract-review", "leak-review", "tm-steward"])


class BranchMerge(unittest.TestCase):
    EVIDENCE = "crates/ryo/docs/contracts/evidence/branches/claude-w1-x.json"

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.tree = repo(self.dir.name, "tree", "claude/checkpoint-9")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "base")
        git(self.tree, "checkout", "-q", "-b", "claude/w1-x")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "work")
        self.head = git(self.tree, "rev-parse", "HEAD")
        git(self.tree, "checkout", "-q", "claude/checkpoint-9")

    def evidence(self, **fields):
        body = {"source": self.head, "verdict": "PASS", "reviewer": "leak-review"}
        body.update(fields)
        path = os.path.join(self.tree, self.EVIDENCE)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(body, handle)
        git(self.tree, "add", self.EVIDENCE)
        git(self.tree, "commit", "-q", "-m", "docs: leak review")

    def merge(self, ref="claude/w1-x"):
        return run_hook("guard_shell.py", {"tool_input": {"command": "git merge --no-ff -m x " + ref}, "cwd": self.tree})

    def run_in(self, command):
        return run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})

    def test_shell_expanded_or_unresolvable_refs_are_refused(self):
        self.evidence()
        for command in ['B=claude/w1-x; git merge "$B"', "git merge $(git rev-parse claude/w1-x)",
                        "git merge claude/w1-*", "git merge claude/nope", "git merge", "git merge `echo claude/w1-x`"]:
            with self.subTest(command=command):
                self.assertEqual(self.run_in(command).returncode, 2)

    def test_wrapped_merges_are_gated(self):
        for command in ["env git merge claude/w1-x", "command git merge claude/w1-x",
                        "bash -c 'git merge claude/w1-x'", "sh -c \"env FOO=1 git merge claude/w1-x\""]:
            with self.subTest(command=command):
                self.assertEqual(self.run_in(command).returncode, 2)

    def test_pull_is_refused_in_a_checkpoint_tree_only(self):
        for command in ["git pull", "git pull . claude/w1-x", "env git pull origin claude/w1-x"]:
            with self.subTest(command=command):
                self.assertEqual(self.run_in(command).returncode, 2)
        git(self.tree, "checkout", "-q", "claude/w1-x")
        self.assertEqual(self.run_in("git pull").returncode, 0)

    def test_a_moved_branch_old_sha_is_refused(self):
        self.evidence()
        git(self.tree, "checkout", "-q", "claude/w1-x")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "more work")
        git(self.tree, "checkout", "-q", "claude/checkpoint-9")
        self.assertEqual(self.run_in("git merge " + self.head).returncode, 2)
        self.assertEqual(self.run_in("git merge claude/w1-x").returncode, 2)

    def test_detached_head_at_a_checkpoint_counts(self):
        git(self.tree, "checkout", "-q", "--detach")
        self.assertEqual(self.run_in("git merge claude/w1-x").returncode, 2)

    def test_merge_control_options_pass(self):
        self.assertEqual(self.run_in("git merge --abort").returncode, 0)

    def test_missing_file_is_refused_naming_file_and_agent(self):
        done = self.merge()
        self.assertEqual(done.returncode, 2)
        self.assertIn(self.EVIDENCE, done.stderr)
        self.assertIn("leak-review subagent", done.stderr)

    def test_stale_source_is_refused(self):
        self.evidence(source="0" * 40)
        self.assertEqual(self.merge().returncode, 2)

    def test_fail_verdict_is_refused(self):
        self.evidence(verdict="FAIL")
        self.assertEqual(self.merge().returncode, 2)

    def test_wrong_reviewer_or_missing_verdict_is_refused(self):
        self.evidence(reviewer="someone")
        self.assertEqual(self.merge().returncode, 2)

    def test_pass_is_allowed_by_name_or_sha(self):
        self.evidence()
        self.assertEqual(self.merge().returncode, 0)
        self.assertEqual(self.merge(self.head).returncode, 0)

    def test_exempt_refs(self):
        git(self.tree, "branch", "claude/ryo-consolidation", self.head)
        git(self.tree, "branch", "claude/checkpoint-8", self.head)
        git(self.tree, "branch", "claude/ancestor", "HEAD")
        for ref in ("claude/ryo-consolidation", "claude/checkpoint-8", "claude/ancestor"):
            with self.subTest(ref=ref):
                self.assertEqual(self.merge(ref).returncode, 0)

    def test_other_trees_are_not_gated(self):
        git(self.tree, "checkout", "-q", "-b", "claude/w2-y")
        self.assertEqual(self.merge().returncode, 0)


class Acceptance(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.bin = os.path.join(self.dir.name, "bin")
        os.mkdir(self.bin)
        self.work = os.path.join(self.dir.name, "tree")
        os.mkdir(self.work)

    def fake_bd(self, script):
        notes = "intro\nACCEPTANCE:\n" + script + "\nEND ACCEPTANCE\nafter"
        path = os.path.join(self.bin, "bd")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write("#!/bin/sh\ncat <<'NOTES'\n" + json.dumps([{"id": "ryo-t1", "notes": notes}]) + "\nNOTES\n")
        os.chmod(path, 0o755)

    def stop(self, brief, report):
        transcript = os.path.join(self.dir.name, "t.jsonl")
        rows = [{"type": "user", "message": {"role": "user", "content": brief}},
                {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "working"}]}},
                {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": report}]}}]
        with open(transcript, "w", encoding="utf-8") as handle:
            handle.write("\n".join(json.dumps(r) for r in rows))
        env = dict(os.environ, PATH=self.bin + os.pathsep + os.environ["PATH"],
                   CLAUDE_HOOKS_STATE=os.path.join(self.dir.name, "state"))
        return run_hook("acceptance_on_stop.py", {"transcript_path": transcript, "stop_hook_active": False}, env)

    def brief(self):
        return "BEAD: ryo-t1\nWORKTREE: " + self.work + "\nDo the work."

    def test_done_and_pass(self):
        self.fake_bd("test -d .")
        self.assertEqual(self.stop(self.brief(), "DONE: R1 ok").returncode, 0)

    def test_done_and_fail(self):
        self.fake_bd("echo nope; false")
        result = self.stop(self.brief(), "DONE: R1 ok")
        self.assertEqual(result.returncode, 2)
        self.assertIn("Acceptance failed for ryo-t1", result.stderr)

    def test_long_backslash_block_round_trips(self):
        out = os.path.join(self.dir.name, "out")
        long = "x" * 100
        self.fake_bd("printf '%s' 'a\\\\b " + long + "' \\\n> " + out)
        self.assertEqual(self.stop(self.brief(), "DONE: ok").returncode, 0)
        with open(out, encoding="utf-8") as handle:
            self.assertEqual(handle.read(), "a\\\\b " + long)

    def test_blocked(self):
        self.fake_bd("false")
        self.assertEqual(self.stop(self.brief(), "BLOCKED: which crate?").returncode, 0)

    def test_not_a_builder(self):
        self.fake_bd("false")
        self.assertEqual(self.stop("Just research this.", "DONE: ok").returncode, 0)

    def test_missing_prefix(self):
        self.fake_bd("true")
        result = self.stop(self.brief(), "All finished.")
        self.assertEqual(result.returncode, 2)
        self.assertIn("DONE: or BLOCKED:", result.stderr)

    def test_stop_allowed_after_five_failures(self):
        self.fake_bd("false")
        codes = [self.stop(self.brief(), "DONE: x").returncode for _ in range(5)]
        self.assertEqual(codes, [2, 2, 2, 2, 0])


class NoSubagentBuilders(unittest.TestCase):
    SUB = "/home/u/.claude/projects/p/s1/subagents/agent-a1.jsonl"
    MAIN = "/home/u/.claude/projects/p/s1.jsonl"

    def spawn(self, tool, path):
        return run_hook("no_subagent_builders.py", {"tool_name": tool, "tool_input": {}, "transcript_path": path})

    def test_refused_from_subagent(self):
        result = self.spawn("Agent", self.SUB)
        self.assertEqual(result.returncode, 2)
        self.assertIn("Blocked: a builder never spawns subagents", result.stderr)

    def test_allowed_from_main_session(self):
        self.assertEqual(self.spawn("Agent", self.MAIN).returncode, 0)

    def test_allowed_for_other_tool(self):
        self.assertEqual(self.spawn("Bash", self.SUB).returncode, 0)

    def test_allowed_on_malformed_input(self):
        result = subprocess.run(["python3", "-I", os.path.join(HERE, "no_subagent_builders.py")],
                                input="not json", capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0)


class CheckInOnStop(unittest.TestCase):
    NAMES = ["Roadmap", "Running", "Changed since last check-in", "Cycle metrics",
             "Decisions I took without you, and anything outside what we agreed",
             "Suggested process improvements", "Needs you", "Next"]

    def checkin(self, order=None, skip=None):
        parts = ["**CHECK-IN 12:00 UTC**"]
        for n in order or range(1, 9):
            if n != skip:
                parts.append("**%d. %s**\n\n| a | b |\n|---|---|\n| 1 | 2 |" % (n, self.NAMES[n - 1]))
        return "\n\n".join(parts)

    def stop(self, rows, path=None, active=False):
        from datetime import datetime, timedelta, timezone
        with tempfile.TemporaryDirectory() as tmp:
            transcript = os.path.join(tmp, "s.jsonl")
            with open(transcript, "w", encoding="utf-8") as handle:
                for minutes, text in rows:
                    stamp = (datetime.now(timezone.utc) - timedelta(minutes=minutes)).isoformat()
                    handle.write(json.dumps({"type": "assistant", "timestamp": stamp,
                                             "message": {"role": "assistant",
                                                         "content": [{"type": "text", "text": text}]}}) + "\n")
            return run_hook("check_in_on_stop.py", {"transcript_path": path or transcript,
                                                    "stop_hook_active": active})

    def test_valid_checkin_passes(self):
        self.assertEqual(self.stop([(0, self.checkin())]).returncode, 0)

    def test_missing_table_blocks(self):
        result = self.stop([(0, self.checkin(skip=4))])
        self.assertEqual(result.returncode, 2)
        self.assertIn("missing table 4 (Cycle metrics)", result.stderr)

    def test_out_of_order_blocks(self):
        result = self.stop([(0, self.checkin(order=[1, 3, 2, 4, 5, 6, 7, 8]))])
        self.assertEqual(result.returncode, 2)
        self.assertIn("Blocked: the CHECK-IN is missing table", result.stderr)

    LISTS = ("**Right now: merging the wave**\n\n**Phases**\n\n- [x] design\n- [ ] build\n- [ ] land\n\n")

    def test_opening_checklists_before_table_one_pass(self):
        text = self.checkin().replace("\n\n**1.", "\n\n" + self.LISTS + "**1.", 1)
        self.assertEqual(self.stop([(0, text)]).returncode, 0)

    def test_opening_checklists_without_table_one_block(self):
        text = self.checkin(skip=1).replace("\n\n**2.", "\n\n" + self.LISTS + "**2.", 1)
        result = self.stop([(0, text)])
        self.assertEqual(result.returncode, 2)
        self.assertIn("missing table 1 (Roadmap)", result.stderr)

    def test_heading_without_table_blocks(self):
        text = self.checkin().replace("| a | b |\n|---|---|\n| 1 | 2 |", "none", 1)
        self.assertEqual(self.stop([(0, text)]).returncode, 2)

    def test_recent_checkin_then_other_message_passes(self):
        self.assertEqual(self.stop([(30, self.checkin()), (1, "working")]).returncode, 0)

    def test_old_checkin_blocks(self):
        result = self.stop([(90, self.checkin()), (1, "working")])
        self.assertEqual(result.returncode, 2)
        self.assertIn("more than 60 minutes", result.stderr)

    def test_no_checkin_blocks(self):
        self.assertEqual(self.stop([(1, "working")]).returncode, 2)

    def test_subagent_transcript_passes(self):
        path = "/home/u/.claude/projects/p/s1/subagents/agent-a1.jsonl"
        self.assertEqual(self.stop([(1, "working")], path=path).returncode, 0)

    def test_stop_hook_active_passes(self):
        self.assertEqual(self.stop([(1, "working")], active=True).returncode, 0)

    def test_malformed_input_passes(self):
        result = subprocess.run(["python3", "-I", os.path.join(HERE, "check_in_on_stop.py")],
                                input="not json", capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
