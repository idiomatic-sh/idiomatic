"""Tests for the guard_shell, acceptance_on_stop, no_subagent_builders and check_in_on_stop hooks, run with `python3 -I -m unittest`."""

import json
import re
import os
import shutil
import subprocess
import tempfile
import tomllib
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
LOCK = "flock /workspaces/.cargo-build.lock "
CHECK = " --target-dir /workspaces/idiomatic-check-target"
OWN = " --target-dir /workspaces/idiomatic-target/builder"


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


GOOD_CHECKS = [{"id": "1", "verdict": "PASS", "evidence": "x.rs:1"}]


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

    def test_cargo_in_a_heredoc_body_is_read_as_a_command(self):
        self.assertEqual(shell("cat > notes.md <<-EOF\n\tcargo build\n\tEOF\ncargo build", self.builder), 2)
        self.assertEqual(shell("bash <<'EOF'\ncargo build\nEOF", self.builder), 2)
        body = "\ncargo build\nEOF"
        for command in ["cat <<EOF | bash" + body, "cat <<'EOF' | sh" + body, "sudo bash <<EOF" + body,
                        "sudo -u root bash <<EOF" + body, "<<EOF bash" + body, "<< EOF bash" + body,
                        "cat <<EOF | xargs -I{} sh -c {}" + body, "env A=1 timeout 60 zsh <<EOF" + body,
                        "cat <<EOF |\ncargo build\nEOF\nbash", "cat <<EOF \\\n| dash" + body,
                        "echo hi # see <<EOF\ncargo build", "echo $((1<<EOF))\ncargo build",
                        "git commit -m \"notes about\nthe <<EOF form\" && true\ncargo build",
                        "read -r base <<<main\ncargo build", "cat <<<'hello'\ncargo build",
                        "read -r x <<< \"word\"\ncargo build", "cat <<EOF | fish" + body,
                        "cat <<EOF | source /dev/stdin" + body, "cat <<EOF | . /dev/stdin" + body]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)
        push = "\ngit push origin HEAD:main"
        for command in ["x=$(cat <<EOF\nhi\nEOF)" + push, "x=$(cat <<'EOF'\nhi\nEOF)" + push,
                        "echo \"$(printf '%s' \"the <<EOF form\")\"" + push,
                        "git commit -m \"$(printf '%s' \"the <<EOF form\")\"" + push, "let x=1<<y" + push]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)

    def test_a_here_string_hides_no_later_line(self):
        self.assertEqual(shell("read -r base <<<\"main\"\ngit push origin HEAD:main", self.builder), 2)

    def test_clippy_runs_only_in_an_integration_tree(self):
        copy = tempfile.mkdtemp(prefix="ryo-review-", dir="/tmp")
        self.addCleanup(shutil.rmtree, copy)
        clippy = LOCK + "cargo clippy --workspace --all-targets --target-dir " + copy + "/tgt"
        tree = self.builder
        for command in [clippy, "cd " + tree + " && " + clippy + "; cd " + copy,
                        "cd " + tree + " && " + clippy + " && cd " + copy, "cd " + tree + " && " + clippy + "\ncd " + copy,
                        "(cd " + tree + " && " + clippy + "); cd " + copy, "pushd " + tree + " && " + clippy + "; popd",
                        "builtin cd " + tree + " && " + clippy, "command cd " + tree + " && " + clippy]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, copy), 2)

    def test_a_builders_check_behind_one_unwrapping_is_allowed(self):
        check = "cargo check -p x --target-dir /workspaces/idiomatic-check-target/ryo-w5"
        for command in ["timeout 300 " + check, "nice -n 5 " + check, "env RUSTFLAGS=-Dwarnings " + check,
                        "timeout -s KILL 300 nice -n 5 env A=1 " + check, "FOO=1 " + check]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 0)
        for command in ["timeout 300 cargo build", "nice -n 5 cargo test -p x", "env A=1 cargo check --workspace"]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)

    def test_a_build_behind_timeout_in_an_integration_tree_still_needs_the_flock(self):
        self.assertEqual(shell("timeout 300 cargo build", self.integration), 2)
        self.assertEqual(shell("timeout 300 " + LOCK + "cargo build", self.integration), 0)
        self.assertEqual(shell("if cargo build; then :; fi", self.integration), 2)

    def test_cargo_behind_keywords_and_remaining_wrappers_is_blocked_in_a_builder_tree(self):
        for command in ["if cargo build; then :; fi", "! cargo build", "{ cargo build; }", "unbuffer cargo build",
                        "ionice -c 2 cargo build", "exec -a x cargo build", "env -S 'cargo build'",
                        "script -c 'cargo build' /dev/null", "watch 'cargo build'", "time cargo build"]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)

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
                        LOCK + "cargo test -p x", "bash -c 'cargo check -p x" + CHECK + "'",
                        LOCK + "cargo build -p x" + OWN, LOCK + "cargo clippy -p x" + OWN]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, self.builder), 2)

    def lane(self):
        """A builder worktree whose branch changes package x (untracked) over a base holding package y."""
        lane = repo(self.dir.name, "lane-" + str(len(os.listdir(self.dir.name))), "claude/p2-lane")
        git = ["git", "-C", lane, "-c", "user.email=t@example.com", "-c", "user.name=t"]
        os.makedirs(os.path.join(lane, "y"))
        with open(os.path.join(lane, "y", "Cargo.toml"), "w", encoding="utf-8") as handle:
            handle.write('[package]\nname = "y"\n')
        subprocess.run(git + ["add", "-A"], check=True)
        subprocess.run(git + ["commit", "-q", "-m", "base"], check=True)
        subprocess.run(git + ["update-ref", "refs/remotes/origin/claude/ryo-consolidation", "HEAD"], check=True)
        os.makedirs(os.path.join(lane, "x", "src"))
        with open(os.path.join(lane, "x", "Cargo.toml"), "w", encoding="utf-8") as handle:
            handle.write('[package]\nname = "x"\n')
        with open(os.path.join(lane, "x", "src", "lib.rs"), "w", encoding="utf-8") as handle:
            handle.write("")
        return lane, " --target-dir /workspaces/idiomatic-target/" + os.path.basename(lane)

    def test_builder_runs_one_changed_packages_tests_into_its_own_target_directly_behind_the_lock(self):
        lane, own = self.lane()
        for command in [LOCK + "cargo nextest run -p x" + own, LOCK + "cargo test -p x" + own,
                        LOCK + "cargo test -p x --lib some::test" + own, "timeout 600 " + LOCK + "cargo nextest run -p x" + own,
                        LOCK + "cargo +nightly test --package=x" + own + "/", LOCK + "cargo test -p x" + own + " -- --nocapture",
                        LOCK + "cargo-nextest nextest run -p x" + own, LOCK + "/usr/local/cargo/bin/cargo test -px" + own,
                        "flock -w 600 /workspaces/.cargo-build.lock cargo test -p x" + own,
                        "flock --timeout 600 /workspaces/.cargo-build.lock cargo test -p x" + own,
                        "flock --wait 600 /workspaces/.cargo-build.lock cargo test -p x" + own,
                        "flock --wait=600 /workspaces/.cargo-build.lock cargo test -p x" + own,
                        "flock -xw 600 /workspaces/.cargo-build.lock cargo nextest run -p x" + own,
                        LOCK + "cargo --config net.offline=true test -p x" + own]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, lane), 0)

    def test_builder_tests_escaping_the_lane_are_refused(self):
        lane, own = self.lane()
        for command in [
                # the lock: missing, a substring elsewhere, another file, shared
                "cargo nextest run -p x" + own, "echo /workspaces/.cargo-build.lock; cargo test -p x" + own,
                "env Z=/workspaces/.cargo-build.lock cargo test -p x" + own,
                "flock /workspaces/.cargo-build.lockx cargo test -p x" + own,
                "flock -s /workspaces/.cargo-build.lock cargo test -p x" + own,
                "flock -w 5 -s /workspaces/.cargo-build.lock cargo test -p x" + own,
                "flock -sw 5 /workspaces/.cargo-build.lock cargo test -p x" + own,
                "flock -sw 5 /workspaces/.cargo-build.lock cargo test --workspace" + own,
                "flock --shared /workspaces/.cargo-build.lock cargo test -p x" + own,
                "flock --wait 5 -s /workspaces/.cargo-build.lock cargo test -p x" + own,
                "flock --wait 5 /workspaces/.cargo-build.lock sh -c 'cargo test --workspace'" + own,
                # a --config value read as the verb
                LOCK + "cargo --config check test --workspace" + CHECK,
                "cargo --config check test -p x" + CHECK,
                # the target
                LOCK + "cargo nextest run -p x", LOCK + "cargo nextest run -p x" + CHECK + "/" + os.path.basename(lane),
                LOCK + "cargo test -p x --target-dir /workspaces/idiomatic-target/other",
                # the packages: none, the workspace, a pattern, several, one the branch does not change, after --
                LOCK + "cargo nextest run" + own, LOCK + "cargo test --workspace" + own, LOCK + "cargo test --all" + own,
                LOCK + "cargo nextest run --workspace -p x" + own, LOCK + "cargo test -p '*'" + own,
                LOCK + "cargo nextest run -p 'x*'" + own, LOCK + "cargo test -p x -p y" + own, LOCK + "cargo test -p y" + own,
                LOCK + "cargo test" + own + " -- -p x", LOCK + "cargo test" + own + " -- --package x",
                LOCK + "cargo nextest run" + own + " -- -p x",
                # aliases, plugins and other verbs
                LOCK + "cargo t --workspace" + own, LOCK + "cargo b --workspace" + own, LOCK + "cargo coverage-all" + own,
                LOCK + "cargo-nextest nextest run --workspace" + own, LOCK + "cargo build -p x" + own,
                LOCK + "cargo clippy -p x" + own, LOCK + "cargo nextest list -p x" + own]:
            with self.subTest(command=command):
                self.assertEqual(shell(command, lane), 2)

    def test_a_file_moved_out_of_a_package_still_counts_as_that_packages_change(self):
        lane, own = self.lane()
        git = ["git", "-C", lane, "-c", "user.email=t@example.com", "-c", "user.name=t"]
        os.makedirs(os.path.join(lane, "y", "src"))
        with open(os.path.join(lane, "y", "src", "lib.rs"), "w", encoding="utf-8") as handle:
            handle.write("pub fn y() {}\n")
        subprocess.run(git + ["add", "-A", "y"], check=True)
        subprocess.run(git + ["commit", "-q", "-m", "y source"], check=True)
        subprocess.run(git + ["update-ref", "refs/remotes/origin/claude/ryo-consolidation", "HEAD"], check=True)
        subprocess.run(git + ["add", "-A", "x"], check=True)
        subprocess.run(git + ["mv", "y/src/lib.rs", "x/src/moved.rs"], check=True)
        subprocess.run(git + ["commit", "-q", "-m", "move"], check=True)
        self.assertEqual(shell(LOCK + "cargo test -p y" + own, lane), 0)

    def test_a_builder_tree_without_a_known_base_runs_no_tests(self):
        self.assertEqual(shell(LOCK + "cargo test -p x" + OWN, self.builder), 2)

    def test_a_push_to_main_is_refused_in_every_spelling(self):
        main = repo(self.dir.name, "on-main", "main")
        tracking = repo(self.dir.name, "tracking-main", "claude/p3-tracking")
        subprocess.run(["git", "-C", tracking, "config", "branch.claude/p3-tracking.remote", "origin"], check=True)
        subprocess.run(["git", "-C", tracking, "config", "branch.claude/p3-tracking.merge", "refs/heads/main"], check=True)
        configured = repo(self.dir.name, "configured-push", "claude/p3-configured")
        subprocess.run(["git", "-C", configured, "config", "remote.origin.push", "HEAD:refs/heads/main"], check=True)
        for command, where in [("git push origin main", self.builder), ("git push origin HEAD:refs/heads/main", self.builder),
                               ("git push -u origin +HEAD:master", self.builder), ("git push", main), ("git push origin HEAD", main),
                               ("git -C " + main + " push origin", self.builder), ("git push -d origin main", self.builder),
                               ("git push origin :main", self.builder), ("git push origin +:refs/heads/main", self.builder),
                               ("git push origin HEAD:heads/main", self.builder),
                               ("git -c push.default=upstream push", self.builder),
                               ("git -c remote.origin.push=HEAD:refs/heads/main push origin", self.builder),
                               ("git push", tracking), ("git push origin", configured)]:
            with self.subTest(command=command, where=where):
                self.assertEqual(shell(command, where), 2)
        self.assertEqual(shell("git push origin claude/p1-coverage-kinds", self.builder), 0)
        self.assertEqual(shell("git push origin HEAD:claude/p3-configured", configured), 0)

    def test_integration_tree_nextest_needs_the_lock(self):
        self.assertEqual(shell(LOCK + "cargo nextest run --workspace", self.integration), 0)
        self.assertEqual(shell("cargo nextest run --workspace", self.integration), 2)

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
            json.dump({"checkpoint": 9, "source": source, "verdict": verdict, "checks": GOOD_CHECKS,
                       "reviewer": "rust-review", "date": "2026-11-07"}, handle)
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

    def test_a_ref_name_as_source_is_refused(self):
        for source in ("HEAD", "claude/checkpoint-9", self.base[:12]):
            with self.subTest(source=source):
                self.commit_review("PASS", source)
                self.assertEqual(self.push().returncode, 2)

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


class FailsClosed(unittest.TestCase):
    """A gated command whose guard breaks is refused (exit 2); a crash would be non-blocking and let it run."""

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.root = os.path.join(self.dir.name, "hooks-tree")
        os.makedirs(os.path.join(self.root, ".claude", "hooks"))
        shutil.copy(os.path.join(HERE, "guard_shell.py"), os.path.join(self.root, ".claude", "hooks"))
        self.tree = repo(self.dir.name, "tree", "claude/checkpoint-9")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "base")
        base = git(self.tree, "rev-parse", "HEAD")
        path = os.path.join(self.tree, RustReviewPush.REVIEW)
        os.makedirs(os.path.dirname(path))
        with open(path, "w", encoding="utf-8") as handle:
            json.dump({"source": base, "verdict": "PASS", "reviewer": "rust-review", "checks": GOOD_CHECKS}, handle)
        git(self.tree, "add", path)
        git(self.tree, "commit", "-q", "-m", "review")

    def run_copy(self, command):
        hook = os.path.join(self.root, ".claude", "hooks", "guard_shell.py")
        return subprocess.run(["python3", "-I", hook], input=json.dumps({"tool_input": {"command": command}, "cwd": self.tree}),
                              capture_output=True, text=True, check=False, cwd=tempfile.gettempdir())

    PUSH = "git push origin HEAD:claude/ryo-consolidation"

    def add_service(self, text):
        folder = os.path.join(self.root, "ci", "contract_review")
        os.makedirs(folder)
        with open(os.path.join(folder, "service.py"), "w", encoding="utf-8") as handle:
            handle.write(text)

    def test_the_copy_passes_with_the_real_service(self):
        shutil.copytree(os.path.join(HERE, "..", "..", "ci", "contract_review"), os.path.join(self.root, "ci", "contract_review"))
        self.assertEqual(self.run_copy(self.PUSH).returncode, 0)

    def test_a_missing_service_refuses_the_push(self):
        done = self.run_copy(self.PUSH)
        self.assertEqual(done.returncode, 2)
        self.assertIn("service.py", done.stderr)

    def test_a_service_without_shape_refuses_the_push(self):
        self.add_service("VERDICTS = ()\n")
        done = self.run_copy(self.PUSH)
        self.assertEqual(done.returncode, 2)
        self.assertIn("service.py", done.stderr)

    def inject(self, old, new):
        path = os.path.join(self.root, ".claude", "hooks", "guard_shell.py")
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
        self.assertIn(old, text)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(text.replace(old, new, 1))

    def test_an_exception_in_any_stage_refuses_a_push_and_a_merge_but_not_other_commands(self):
        stages = {"a gate": ("GATES = (", "def broken(inner, here):\n    raise RuntimeError('injected')\n\n\nGATES = (broken, "),
                  "stash": ("def stash(words):", "def stash(words):\n    raise RuntimeError('injected')\n\n\ndef _stash(words):"),
                  "segments": ("def segments(text):", "def segments(text):\n    raise RuntimeError('injected')\n\n\ndef _segments(text):")}
        for label, (old, new) in stages.items():
            with self.subTest(label):
                self.setUp()
                self.inject(old, new)
                for command in (self.PUSH, "git merge claude/w1-x", "cd /tmp && git pull"):
                    done = self.run_copy(command)
                    self.assertEqual(done.returncode, 2, command)
                    self.assertIn("guard failed closed: RuntimeError: injected", done.stderr)
                self.assertEqual(self.run_copy("ls").returncode, 1)

    def test_a_nul_byte_in_a_cargo_argument_still_refuses_a_push_in_the_same_command(self):
        done = self.run_copy("cd /tmp/a\x00b && cargo build && " + self.PUSH)
        self.assertEqual(done.returncode, 2)
        self.assertIn("guard failed closed", done.stderr)

    def test_quoted_or_escaped_spellings_of_the_verb_are_still_refused_when_the_guard_breaks(self):
        self.inject("def segments(text):", "def segments(text):\n    raise RuntimeError('injected')\n\n\ndef _segments(text):")
        for command in ("gi''t merge x", 'git mer""ge x', "g\\it merge x", "git m\\erge x", "git 'pu'sh origin"):
            with self.subTest(command=command):
                self.assertEqual(self.run_copy(command).returncode, 2)


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
        self.write("rust-review.json", self.body("PASS", "rust-review", git(self.tree, "rev-parse", "HEAD")))

    @staticmethod
    def body(verdict, reviewer, source):
        return {"source": source, "verdict": verdict, "reviewer": reviewer, "checks": GOOD_CHECKS}

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
                self.write(name, self.body("BLOCKING", name[:-5], source or self.previous))

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
        self.write("leak-review.json", {"source": self.previous, "reviewer": "leak-review", "checks": GOOD_CHECKS})
        self.assertIn("no known verdict", self.push().stderr)

    def test_malformed_review_evidence_is_refused(self):
        good = self.body("PASS", "leak-review", self.previous)
        bad = {"short source": dict(good, source=self.previous[:12]),
               "unknown verdict": dict(good, verdict="MAYBE"),
               "wrong reviewer": dict(good, reviewer="someone"),
               "no reviewer": {k: v for k, v in good.items() if k != "reviewer"},
               "checks not a list": dict(good, checks="none"),
               "empty checks": dict(good, checks=[]),
               "check is a string": dict(good, checks=["x"]),
               "check is a number": dict(good, checks=[1]),
               "check without evidence": dict(good, checks=[{"id": "1", "verdict": "PASS", "evidence": ""}]),
               "check with unknown verdict": dict(good, checks=[{"id": "1", "verdict": "OK", "evidence": "e"}]),
               "FAIL check under PASS": dict(good, checks=[{"id": "1", "verdict": "FAIL", "evidence": "e"}]),
               "no checks": {k: v for k, v in good.items() if k != "checks"}}
        for label, body in bad.items():
            with self.subTest(label):
                self.setUp()
                self.all_reviews(skip=("leak-review.json",))
                self.write("leak-review.json", body)
                result = self.push()
                self.assertEqual(result.returncode, 2)
                self.assertIn("leak-review.json", result.stderr)

    def test_asymptote_record_is_validated_when_present(self):
        self.all_reviews()
        self.write("asymptote.json", self.body("PASS", "someone", self.previous))
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("asymptote.json", result.stderr)
        self.write("asymptote.json", self.body("PASS", "integrator", self.previous))
        self.assertEqual(self.push().returncode, 0)

    def test_bootstrap_head_needs_only_the_contract_review(self):
        # the bootstrap sha cannot be forged, so point the constant at this remote head
        import importlib.util
        spec = importlib.util.spec_from_file_location("guard_shell", os.path.join(HERE, "guard_shell.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.BOOTSTRAP_HEAD = self.previous[:10]
        words = ["git", "push", "origin", "HEAD:claude/ryo-consolidation"]
        self.assertIn("contract-review.json", module.previous_reviews(words, self.tree))
        self.write("contract-review.json", self.body("PASS", "contract-review", self.previous))
        self.assertIsNone(module.previous_reviews(words, self.tree))

    def candidate(self, klass="pub leak", signature="pub fn"):
        import hashlib
        slug = re.sub(r"[^a-z0-9]+", "-", klass.lower()).strip("-")
        return {"id": slug + "-" + hashlib.sha256(signature.encode()).hexdigest()[:8], "class": klass,
                "signature": signature, "scope": ["crates/a"], "suggested_mechanism": "new rule",
                "plants": [{"file": "a.rs", "line": 1, "text": "pub fn x() {}"}]}

    def with_candidates(self, leak=(), branch=(), decisions=()):
        self.all_reviews(skip=("leak-review.json", "tm-steward.json"))
        self.write("leak-review.json", dict(self.body("PASS", "leak-review", self.previous),
                                            enforcement_candidates=list(leak)))
        if branch:
            path = os.path.join(self.tree, "crates/ryo/docs/contracts/evidence/branches/claude-w1-x.json")
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w", encoding="utf-8") as handle:
                json.dump(dict(self.body("PASS", "leak-review", self.previous), enforcement_candidates=list(branch)), handle)
            git(self.tree, "add", path)
            git(self.tree, "commit", "-q", "-m", "branch evidence")
        self.write("tm-steward.json", dict(self.body("PASS", "tm-steward", self.previous),
                                           candidate_decisions=list(decisions)))

    @staticmethod
    def decision(candidate, **fields):
        return dict({"id": candidate["id"], "decision": "new", "mechanism": "rule@1", "plant_counts": [1], "reason": "x"},
                    **fields)

    def test_no_candidates_is_allowed(self):
        self.with_candidates()
        self.assertEqual(self.push().returncode, 0)

    def test_every_candidate_decided_is_allowed(self):
        one, two = self.candidate(), self.candidate("other", "let x")
        self.with_candidates(leak=[one], branch=[two], decisions=[
            self.decision(one), self.decision(two, decision="none", plant_counts=[], reason="a one-off class that costs more to enforce")])
        self.assertEqual(self.push().returncode, 0)

    def test_a_missing_decision_is_refused(self):
        one, two = self.candidate(), self.candidate("other", "let x")
        self.with_candidates(leak=[one], branch=[two], decisions=[self.decision(one)])
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn(two["id"], result.stderr)
        self.assertIn("claude-w1-x.json", result.stderr)

    def test_a_strengthen_decision_with_a_zero_plant_is_refused(self):
        one = self.candidate()
        self.with_candidates(leak=[one], decisions=[self.decision(one, decision="strengthen", plant_counts=[1, 0])])
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("plant counts of at least 1", result.stderr)

    def test_a_none_decision_without_a_reason_is_refused(self):
        one = self.candidate()
        self.with_candidates(leak=[one], decisions=[self.decision(one, decision="none", plant_counts=[], reason="too short")])
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("without a real reason", result.stderr)

    def test_a_decision_whose_plant_counts_do_not_match_the_plants_is_refused(self):
        one = self.candidate()
        self.with_candidates(leak=[one], decisions=[self.decision(one, plant_counts=[1, 1])])
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("one for one", result.stderr)

    def test_a_branch_file_modified_in_the_checkpoint_with_a_new_candidate_is_refused(self):
        old, new = self.candidate(), self.candidate("later", "let y")
        path = os.path.join(self.tree, "crates/ryo/docs/contracts/evidence/branches/claude-old.json")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(dict(self.body("PASS", "leak-review", self.previous), enforcement_candidates=[old]), handle)
        git(self.tree, "add", path)
        git(self.tree, "commit", "-q", "-m", "old branch evidence")
        git(self.tree, "push", "-q", "origin", "HEAD:claude/ryo-consolidation")
        self.setUp_after_landing()
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(dict(self.body("PASS", "leak-review", self.previous), enforcement_candidates=[old, new]), handle)
        git(self.tree, "add", path)
        git(self.tree, "commit", "-q", "-m", "re-reviewed branch evidence")
        self.with_candidates(decisions=[self.decision(old)])
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn(new["id"], result.stderr)

    BRANCH = "crates/ryo/docs/contracts/evidence/branches/claude-old.json"

    def branch_history(self, at_head, steps):
        """Push with a branch file holding at_head at the previous landed head, then the steps in the checkpoint
        (a candidate list writes the file, "delete" removes it, "rename" moves it); every candidate is decided."""
        path = os.path.join(self.tree, self.BRANCH)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        def put(candidates, where=path):
            with open(where, "w", encoding="utf-8") as handle:
                json.dump(dict(self.body("PASS", "leak-review", self.previous), enforcement_candidates=candidates), handle)
            git(self.tree, "add", where)
            git(self.tree, "commit", "-q", "-m", "branch evidence")
        if at_head is not None:
            put(at_head)
            git(self.tree, "push", "-q", "origin", "HEAD:claude/ryo-consolidation")
            self.setUp_after_landing()
        everyone = [self.candidate(), self.candidate("other", "let z")]
        for step in steps:
            if step == "delete":
                git(self.tree, "rm", "-q", self.BRANCH)
                git(self.tree, "commit", "-q", "-m", "delete branch evidence")
            elif step == "rename":
                git(self.tree, "mv", self.BRANCH, self.BRANCH.replace("old", "renamed"))
                git(self.tree, "commit", "-q", "-m", "rename branch evidence")
            else:
                put(step)
        self.with_candidates(decisions=[self.decision(c) for c in everyone])
        return self.push()

    def assert_dropped(self, result):
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("append-only", result.stderr)

    def test_dropping_a_candidate_when_a_branch_file_is_modified_is_refused(self):
        self.assert_dropped(self.branch_history([self.candidate()], [[]]))

    def test_dropping_a_candidate_within_the_checkpoint_is_refused(self):
        self.assert_dropped(self.branch_history(None, [[self.candidate()], []]))

    def test_replacing_the_candidates_with_new_ones_only_is_refused(self):
        self.assert_dropped(self.branch_history([self.candidate()], [[self.candidate("other", "let z")]]))

    def test_deleting_a_branch_file_with_candidates_is_refused(self):
        self.assert_dropped(self.branch_history([self.candidate()], ["delete"]))

    def test_renaming_a_branch_file_with_candidates_is_refused(self):
        self.assert_dropped(self.branch_history([self.candidate()], ["rename"]))

    def test_adding_then_deleting_a_branch_file_inside_the_checkpoint_is_refused(self):
        self.assert_dropped(self.branch_history(None, [[self.candidate()], "delete"]))

    def test_adding_candidates_across_versions_is_allowed(self):
        old = self.candidate()
        result = self.branch_history([old], [[old, self.candidate("other", "let z")]])
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_carrying_a_candidate_forward_as_fixed_is_allowed(self):
        old = self.candidate()
        result = self.branch_history([old], [[dict(old, instance_status="fixed")]])
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_a_candidate_without_plants_is_refused(self):
        one = dict(self.candidate(), plants=[])
        self.with_candidates(leak=[one], decisions=[self.decision(one)])
        self.assertEqual(self.push().returncode, 2)

    def test_a_branch_file_older_than_the_previous_landed_head_needs_no_decision(self):
        # a branch result already present at the previous landed head was decided then; decisions never carry forward
        one = self.candidate()
        path = os.path.join(self.tree, "crates/ryo/docs/contracts/evidence/branches/claude-old.json")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(dict(self.body("PASS", "leak-review", self.previous), enforcement_candidates=[one]), handle)
        git(self.tree, "add", path)
        git(self.tree, "commit", "-q", "-m", "old branch evidence")
        git(self.tree, "push", "-q", "origin", "HEAD:claude/ryo-consolidation")
        self.setUp_after_landing()
        self.with_candidates()
        self.assertEqual(self.push().returncode, 0)

    def setUp_after_landing(self):
        self.previous = git(self.tree, "rev-parse", "HEAD")
        git(self.tree, "commit", "-q", "--allow-empty", "-m", "next checkpoint work")
        self.write("rust-review.json", self.body("PASS", "rust-review", git(self.tree, "rev-parse", "HEAD")))

    def test_a_candidate_whose_id_does_not_match_its_signature_is_refused(self):
        one = dict(self.candidate(), id="pub-leak-00000000")
        self.with_candidates(leak=[one], decisions=[self.decision(one)])
        self.assertEqual(self.push().returncode, 2)


class EvidenceWrites(unittest.TestCase):
    """Review evidence is written only into a claude/checkpoint-N worktree's evidence directory."""
    SCRATCH = "/tmp/claude-scratch/leak-review.json"
    TAIL = "crates/ryo/docs/contracts/evidence/"

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.tree = repo(self.dir.name, "ryo-332", "claude/checkpoint-332")
        self.other = repo(self.dir.name, "ryo-w1", "claude/w1-x")

    def write(self, path):
        return run_hook("guard_write.py", {"tool_name": "Write", "tool_input": {"file_path": path}}).returncode

    def test_a_bash_heredoc_to_the_scratchpad_is_refused(self):
        self.assertEqual(shell("cat > " + self.SCRATCH + " <<'EOF'\n{}\nEOF"), 2)
        self.assertEqual(shell("echo {} | tee /tmp/claude-scratch/tm-steward-9.json"), 2)

    def test_a_bash_cp_to_the_scratchpad_is_refused(self):
        self.assertEqual(shell("cp leak-review.json /tmp/claude-scratch/"), 2)
        self.assertEqual(shell("mv x.json " + self.SCRATCH), 2)
        self.assertEqual(shell("cp x.json " + self.other + "/" + self.TAIL + "branches/x.json"), 2)

    def test_a_write_to_the_scratchpad_is_refused(self):
        self.assertEqual(self.write(self.SCRATCH), 2)
        self.assertEqual(self.write("/tmp/claude-scratch/evidence/branches/x.json"), 2)
        self.assertEqual(self.write(self.other + "/" + self.TAIL + "checkpoint/rust-review.json"), 2)

    def test_a_write_into_a_checkpoint_worktrees_evidence_directory_is_allowed(self):
        self.assertEqual(self.write(self.tree + "/" + self.TAIL + "branches/x.json"), 0)
        self.assertEqual(self.write(self.tree + "/" + self.TAIL + "checkpoint/leak-review.json"), 0)
        self.assertEqual(shell("cat > " + self.tree + "/" + self.TAIL + "checkpoint/leak-review.json <<EOF\n{}\nEOF"), 0)
        self.assertEqual(shell("cp x.json " + self.tree + "/" + self.TAIL + "branches/x.json"), 0)

    def test_in_place_editors_dd_rsync_ln_sponge_and_python_to_the_scratchpad_are_refused(self):
        for command in ("sed -i s/a/b/ /tmp/claude-scratch/leak-review.json",
                        "perl -pi -e 's/a/b/' /tmp/claude-scratch/rust-review.json",
                        "echo {} | sponge /tmp/claude-scratch/tm-steward.json",
                        "dd if=x of=/tmp/claude-scratch/contract-review.json",
                        "rsync x.json /tmp/claude-scratch/leak-review.json",
                        "ln -s x.json /tmp/claude-scratch/leak-review.json",
                        "python3 -c \"open('/tmp/claude-scratch/leak-review.json', 'w').write('{}')\"",
                        "python3 -c \"import json; json.dump({}, open('/tmp/claude-scratch/asymptote.json'))\""):
            with self.subTest(command=command):
                self.assertEqual(shell(command), 2)

    def test_a_variable_rooted_path_into_an_evidence_directory_is_allowed_but_an_outside_one_is_refused(self):
        tail = "crates/ryo/docs/contracts/evidence/checkpoint/leak-review.json"
        self.assertEqual(shell('cat > "$INTEGRATION_TREE_UNSET/' + tail + '" <<EOF\n{}\nEOF'), 0)
        self.assertEqual(shell('cat > "$UNSET_SCRATCH/leak-review.json" <<EOF\n{}\nEOF'), 2)
        env = dict(os.environ, INTEGRATION_TREE=self.tree)
        done = run_hook("guard_shell.py", {"tool_name": "Bash", "tool_input": {"command": 'cp x.json "$INTEGRATION_TREE/' + tail + '"'}}, env=env)
        self.assertEqual(done.returncode, 0)
        env = dict(os.environ, INTEGRATION_TREE=self.other)
        done = run_hook("guard_shell.py", {"tool_name": "Bash", "tool_input": {"command": 'cp x.json "$INTEGRATION_TREE/' + tail + '"'}}, env=env)
        self.assertEqual(done.returncode, 2)

    def test_a_committed_copy_of_evidence_may_be_written_to_the_scratchpad(self):
        self.assertEqual(shell("git show HEAD:x > /tmp/claude-scratch/leak-review-committed-copy.json"), 0)
        self.assertEqual(self.write("/tmp/claude-scratch/leak-review-committed-copy.json"), 0)

    def test_a_git_show_of_a_committed_path_redirected_to_the_scratchpad_is_allowed(self):
        self.assertEqual(shell("git show abc123:" + self.TAIL + "checkpoint/rust-review.json > /tmp/claude-scratch/rust-review.json"), 0)
        self.assertEqual(shell("cat x > /tmp/claude-scratch/rust-review.json"), 2)

    def test_the_committed_settings_run_the_write_hook_for_every_editing_tool(self):
        with open(os.path.join(HERE, "..", "settings.json"), encoding="utf-8") as handle:
            entries = json.load(handle)["hooks"]["PreToolUse"]
        found = [e for e in entries if "guard_write.py" in json.dumps(e)]
        self.assertEqual([e["matcher"] for e in found], ["Write|Edit|MultiEdit|NotebookEdit"])

    def test_an_unrelated_scratchpad_file_is_allowed(self):
        self.assertEqual(self.write("/tmp/claude-scratch/notes.json"), 0)
        self.assertEqual(shell("echo hi > /tmp/claude-scratch/notes.md"), 0)
        self.assertEqual(shell("cp a b 2>&1 > /dev/null"), 0)

    def test_the_write_hook_fails_closed_on_its_own_error_for_evidence_paths_only(self):
        copy = os.path.join(self.dir.name, "hooks")
        shutil.copytree(HERE, copy, ignore=shutil.ignore_patterns("__pycache__", "test_*"))
        with open(os.path.join(copy, "guard_shell.py"), "a", encoding="utf-8") as handle:
            handle.write("\nraise RuntimeError('injected')\n")
        def run(path):
            return subprocess.run(["python3", "-I", os.path.join(copy, "guard_write.py")], capture_output=True, text=True,
                                  input=json.dumps({"tool_input": {"file_path": path}}), check=False)
        self.assertEqual(run(self.SCRATCH).returncode, 2)
        self.assertEqual(run("/tmp/claude-scratch/notes.json").returncode, 0)


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

    def test_redirections_with_no_remote_or_branch_named_still_hit_the_perf_gate(self):
        git(self.tree, "update-ref", "refs/remotes/origin/claude/ryo-consolidation", self.base)
        git(self.tree, "branch", "-q", "-m", "claude/ryo-consolidation", "claude/elsewhere")  # no local consolidation ref
        self.perf()
        for command in ("git push", "git push -q 2>&1", "git push &>log", "git push >log 2>&1", "git push -q >log 2>&1"):
            with self.subTest(command=command):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)
                self.assertIn("push of claude/p6-speed refused", done.stderr)

    def test_stray_tokens_do_not_become_push_operands(self):
        git(self.tree, "update-ref", "refs/remotes/origin/claude/ryo-consolidation", self.base)
        git(self.tree, "branch", "-q", "-m", "claude/ryo-consolidation", "claude/elsewhere")
        self.perf()
        rows = {"here-string": "git push <<< word", "here-string unspaced": "git push <<<word",
                "heredoc spaced": "git push << EOF\nEOF", "heredoc unspaced": "git push <<EOF\nEOF",
                "-o value": "git push -o ci.skip", "--push-option value": "git push --push-option ci.skip",
                "--receive-pack value": "git push --receive-pack /bin/x", "--exec value": "git push --exec /bin/x",
                "--repo value": "git push --repo origin2", "timeout": "timeout 5 git push",
                "timeout with options": "timeout -s KILL 5 git push", "stdbuf": "stdbuf -oL git push",
                "stdbuf separate": "stdbuf -o L -e0 git push", "xargs": "xargs git push",
                "xargs with options": "xargs -n 1 git push"}
        for label, command in rows.items():
            with self.subTest(label):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)
                self.assertIn("push of claude/p6-speed refused", done.stderr)

    def test_keywords_grouping_and_remaining_wrappers_do_not_hide_a_push(self):
        git(self.tree, "update-ref", "refs/remotes/origin/claude/ryo-consolidation", self.base)
        git(self.tree, "branch", "-q", "-m", "claude/ryo-consolidation", "claude/elsewhere")
        self.perf()
        rows = {"if": "if git push; then echo ok; fi", "bang": "! git push", "brace group": "{ git push; }",
                "while": "while git push; do echo; done", "until": "until git push; do :; done",
                "for": "for i in 1; do git push; done", "elif": "if false; then :; elif git push; then :; fi",
                "namespace value": "git --namespace ns push", "config-env value": "git --config-env a=B push",
                "recurse-submodules value": "git push --recurse-submodules check",
                "exec -a": "exec -a name git push", "env -S": "env -S 'git push'", "ionice": "ionice -c 2 -n 4 git push",
                "unbuffer": "unbuffer git push", "script -c": "script -c 'git push' /dev/null",
                "watch quoted": "watch 'git push'", "watch words": "watch -n 5 git push",
                "flock wrapper": "flock /tmp/x git push", "time": "time git push"}
        for label, command in rows.items():
            with self.subTest(label):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)
                self.assertIn("push of claude/p6-speed refused", done.stderr)
        for command in ("eval 'git push'", "sudo git push"):
            with self.subTest(command):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)
                self.assertIn("hides it from the guards", done.stderr)

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

    def test_a_later_number_claim_cannot_ride_an_older_after(self):
        self.diagnosis()
        with open(os.path.join(self.tree, "engine.rs"), "w", encoding="utf-8") as handle:
            handle.write("p11\n")
        git(self.tree, "add", "engine.rs")
        git(self.tree, "commit", "-q", "-m", "perf(engine): p11, 12s to 8s")
        self.write("after.json", {"before": [], "after": [], "identical_output": True,
                                  "source": git(self.tree, "rev-parse", "HEAD")})
        self.assertEqual(self.push().returncode, 0)
        with open(os.path.join(self.tree, "engine.rs"), "w", encoding="utf-8") as handle:
            handle.write("p13\n")
        git(self.tree, "commit", "-q", "-am", "perf(engine): p13, 12s to 4s")
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("only crates/ryo/docs/contracts/evidence/ files changed since", result.stderr)

    def test_after_source_must_be_a_full_sha_not_a_ref_name(self):
        self.diagnosis()
        with open(os.path.join(self.tree, "engine.rs"), "w", encoding="utf-8") as handle:
            handle.write("p13\n")
        git(self.tree, "add", "engine.rs")
        git(self.tree, "commit", "-q", "-m", "perf(engine): p13, 12s to 4s")
        for source in ("HEAD", "claude/p6-speed", "HEAD~0", "HEAD^{}", git(self.tree, "rev-parse", "HEAD")[:12]):
            with self.subTest(source=source):
                self.write("after.json", {"before": [], "after": [], "identical_output": True, "source": source})
                self.assertEqual(self.push().returncode, 2)

    def test_expanded_or_matching_push_operands_are_refused(self):
        for command in ['git push origin "$(git branch --show-current)"', "git push origin :", "git push origin +:",
                        "git push origin claude/*:claude/*", "git push origin $B"]:
            with self.subTest(command=command):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)

    def test_push_all_and_mirror_are_refused(self):
        for command in ["git push --all origin", "git push --mirror", "env git push origin --all"]:
            with self.subTest(command=command):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)

    def test_login_shell_c_is_unwrapped(self):
        self.perf()
        for command in ["bash -lc 'git push origin claude/p6-speed'", "sh -ec 'git push origin claude/p6-speed'"]:
            with self.subTest(command=command):
                done = run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})
                self.assertEqual(done.returncode, 2)
        done = run_hook("guard_shell.py", {"tool_input": {"command": "bash -lc 'cargo build'"}, "cwd": self.tree})
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
            json.dump({"verdict": "PASS", "reviewer": "rust-review", "checks": GOOD_CHECKS,
                       "source": git(self.tree, "rev-parse", "HEAD")}, handle)
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
        body = {"source": self.head, "verdict": "PASS", "reviewer": "leak-review", "checks": GOOD_CHECKS}
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

    def test_merges_behind_keywords_wrappers_and_global_option_values_are_gated(self):
        for command in ["if git merge claude/w1-x; then echo; fi", "! git merge claude/w1-x", "{ git merge claude/w1-x; }",
                        "while git merge claude/w1-x; do :; done", "for i in 1; do git merge claude/w1-x; done",
                        "git --namespace ns merge claude/w1-x", "git --config-env a=B merge claude/w1-x",
                        "timeout 5 git merge claude/w1-x", "ionice -c 2 git merge claude/w1-x",
                        "script -c 'git merge claude/w1-x' /dev/null", "eval 'git merge claude/w1-x'",
                        "sudo git merge claude/w1-x"]:
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

    def test_malformed_branch_evidence_is_refused(self):
        for label, fields in {"no checks": {"checks": "x"}, "empty checks": {"checks": []}, "FAIL check under PASS": {"checks": [{"id": "1", "verdict": "FAIL", "evidence": "e"}]}, "unknown verdict": {"verdict": "MAYBE"},
                              "short source": {"source": self.head[:12]}}.items():
            with self.subTest(label):
                self.evidence(**fields)
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


def write_transcript(path, brief, report):
    """A transcript whose first user message is brief and whose last assistant message is report."""
    rows = [{"type": "user", "timestamp": "2026-11-10T00:00:00Z", "message": {"role": "user", "content": brief}},
            {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "working"}]}},
            {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": report}]}}]
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(json.dumps(r) for r in rows))


def malformed_briefs(tree):
    """Builder briefs without an exact `BEAD: <id>` and `WORKTREE: /abs/path` line: the leak review's six worktree
    spellings (a copied 2026-09-23 brief among them) and two bead spellings."""
    rest = "\nSCOPE={scope}\nR1 do the work."
    return {"no worktree line": "BEAD: ryo-x" + rest,
            "Worktree:": "BEAD: ryo-x\nWorktree: " + tree + rest,
            "WORKTREE=": "BEAD: ryo-x\nWORKTREE=" + tree + rest,
            "bold": "BEAD: ryo-x\n**WORKTREE:** " + tree + rest,
            "indented": "BEAD: ryo-x\n  WORKTREE: " + tree + rest,
            "copied old brief": "BEAD: ryo-x\nWork in your own git worktree, on a branch named for the bead." + rest,
            "relative worktree": "BEAD: ryo-x\nWORKTREE: ryo-criteria" + rest,
            "Bead:": "Bead: ryo-x\nWORKTREE: " + tree + rest}


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
            handle.write("#!/bin/sh\nif [ \"$2\" != ryo-t1 ]; then echo '[]'; exit 0; fi\ncat <<'NOTES'\n" +
                         json.dumps([{"id": "ryo-t1", "notes": notes}]) + "\nNOTES\n")
        os.chmod(path, 0o755)

    def payload(self, brief, report, agent="builder"):
        """Claude Code's SubagentStop payload: the builder's own transcript in agent_transcript_path, its last
        message in last_assistant_message, and transcript_path the PARENT session's, here a decoy builder brief
        for another bead that a hook reading the wrong transcript would judge instead."""
        transcript = os.path.join(self.dir.name, "t.jsonl")
        write_transcript(transcript, brief, report)
        parent = os.path.join(self.dir.name, "parent.jsonl")
        write_transcript(parent, "BEAD: ryo-parent\nWORKTREE: " + self.work + "\nPM session.", "DONE: decoy")
        return {"session_id": "s1", "transcript_path": parent, "hook_event_name": "SubagentStop",
                "stop_hook_active": False, "agent_id": "a1", "agent_type": agent,
                "agent_transcript_path": transcript, "last_assistant_message": report}

    def env(self):
        return dict(os.environ, PATH=self.bin + os.pathsep + os.environ["PATH"],
                    CLAUDE_HOOKS_STATE=os.path.join(self.dir.name, "state"))

    def stop(self, brief, report, agent="builder"):
        return run_hook("acceptance_on_stop.py", self.payload(brief, report, agent), self.env())

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
        self.assertEqual(self.stop("Just research this.", "DONE: ok", agent="general-purpose").returncode, 0)
        self.assertEqual(self.stop("Land checkpoint 7.", "DONE: ok", agent="integrator").returncode, 0)
        self.assertEqual(self.stop(self.brief(), "All finished.", agent="Explore").returncode, 0)

    def test_missing_prefix(self):
        self.fake_bd("true")
        result = self.stop(self.brief(), "All finished.")
        self.assertEqual(result.returncode, 2)
        self.assertIn("DONE: or BLOCKED:", result.stderr)

    def test_every_bead_of_the_brief_passes_its_own_acceptance(self):
        path = os.path.join(self.bin, "bd")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write('#!/bin/sh\nif [ "$2" = ryo-t2 ]; then script=false; else script=true; fi\n'
                         'printf \'[{"id": "%s", "notes": "ACCEPTANCE:\\\\n%s\\\\nEND ACCEPTANCE"}]\\n\' "$2" "$script"\n')
        os.chmod(path, 0o755)
        work = "\nWORKTREE: " + self.work + "\nDo the work."
        self.assertEqual(self.stop("BEAD: ryo-t1" + work, "DONE: ok").returncode, 0)
        self.assertEqual(self.stop("BEAD: ryo-t2" + work, "DONE: ok").returncode, 2)
        for brief in ("BEAD: ryo-t1 ryo-t2" + work, "BEAD: ryo-t1\nBEAD: ryo-t2" + work):
            with self.subTest(brief=brief):
                result = self.stop(brief, "DONE: ok")
                self.assertEqual(result.returncode, 2)
                self.assertIn("Acceptance failed for ryo-t2", result.stderr)

    def test_an_error_on_the_builder_stop_holds(self):
        self.fake_bd("true")
        for brief in ("BEAD: ryo/t1\nWORKTREE: " + self.work + "\nDo it.",
                      "BEAD: ryo-t1\nWORKTREE: " + os.path.join(self.dir.name, "no-such-tree") + "\nDo it."):
            with self.subTest(brief=brief):
                self.assertEqual(self.stop(brief, "DONE: ok").returncode, 2)

    def test_a_builder_brief_without_exact_bead_and_worktree_lines_is_held(self):
        self.fake_bd("false")
        for name, brief in malformed_briefs(self.work).items():
            for agent in ("builder", "contract-migration"):
                with self.subTest(brief=name, agent=agent):
                    self.assertEqual(self.stop(brief.format(scope="x"), "DONE: ok", agent=agent).returncode, 2)
                    self.assertEqual(self.stop(brief.format(scope="x"), "BLOCKED: which tree?", agent=agent).returncode, 0)

    def test_a_gate_that_fails_to_load_holds_the_builder(self):
        copy = os.path.join(self.dir.name, "copy")
        os.makedirs(os.path.join(copy, ".claude", "hooks"))
        shutil.copy(os.path.join(HERE, "acceptance_on_stop.py"), os.path.join(copy, ".claude", "hooks"))
        env = self.env()
        self.fake_bd("true")
        with open(os.path.join(HERE, "no_subagent_builders.py"), encoding="utf-8") as handle:
            gate = handle.read()
        # A gate that fails at import still declares ACCEPTED, so only the agents it names are held; a gate that
        # does not parse leaves the stop hook unable to tell, so every stop is held.
        for broken, other in ((gate + "\nimport no_such_module_planted\n", 0), ("def (:\n", 2)):
            with open(os.path.join(copy, ".claude", "hooks", "no_subagent_builders.py"), "w", encoding="utf-8") as handle:
                handle.write(broken)
            for agent, report, code in [("builder", "All finished.", 2), ("builder", "BLOCKED: which crate?", 2),
                                        ("contract-migration", "All finished.", 2), ("integrator", "All finished.", 2),
                                        ("Explore", "All finished.", other), ("scoper", "PROCEED", other)]:
                with self.subTest(broken=broken[-30:], agent=agent, report=report):
                    result = subprocess.run(["python3", "-I", os.path.join(copy, ".claude", "hooks", "acceptance_on_stop.py")],
                                            input=json.dumps(self.payload(self.brief(), report, agent)),
                                            capture_output=True, text=True, env=env, check=False)
                    self.assertEqual(result.returncode, code, result.stderr)

    def test_stop_allowed_after_five_failures(self):
        self.fake_bd("false")
        results = [self.stop(self.brief(), "DONE: x") for _ in range(5)]
        self.assertEqual([r.returncode for r in results], [2, 2, 2, 2, 0])
        self.assertIn("Acceptance failed for ryo-t1", results[0].stderr)


SHA = "a" * 40


class ReviewerStop(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.tree = os.path.join(self.dir.name, "ryo-333")
        os.makedirs(os.path.join(self.tree, "crates/ryo/docs/contracts/evidence/checkpoint"))
        os.makedirs(os.path.join(self.tree, "crates/ryo/docs/contracts/evidence/branches"))

    def review(self, agent, **changes):
        review = {"source": SHA, "verdict": "FAIL", "reviewer": agent,
                  "checks": [{"id": "1", "verdict": "FAIL", "evidence": "x.rs:1"}]}
        review.update(changes)
        return review

    def write(self, rel, review, age=0):
        path = os.path.join(self.tree, "crates/ryo/docs/contracts/evidence", rel)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(review if isinstance(review, str) else json.dumps(review))
        if age:
            stamp = os.path.getmtime(path) - age
            os.utime(path, (stamp, stamp))

    def stop(self, agent, report="Report.", brief=None, stamped=True):
        transcript = os.path.join(self.dir.name, "agent.jsonl")
        clock = __import__("datetime")
        now = clock.datetime.now(clock.timezone.utc) - clock.timedelta(seconds=60)
        first = {"type": "user", "message": {"role": "user", "content": brief if brief is not None else
                 "Review 333. INTEGRATION_TREE=" + self.tree + ": SOURCE=" + SHA + "."}}
        if stamped:
            first["timestamp"] = now.isoformat()
        rows = [first, {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": report}]}}]
        with open(transcript, "w", encoding="utf-8") as handle:
            handle.write("\n".join(json.dumps(r) for r in rows))
        env = dict(os.environ, CLAUDE_HOOKS_STATE=os.path.join(self.dir.name, "state"))
        return run_hook("acceptance_on_stop.py", {"agent_type": agent, "agent_id": "a1", "agent_transcript_path": transcript,
                                                  "transcript_path": "/nonexistent", "stop_hook_active": False}, env)

    def test_each_reviewer_without_its_verdict_file_is_held_every_time(self):
        for agent in ["rust-review", "contract-review", "leak-review", "tm-steward"]:
            with self.subTest(agent=agent):
                codes = [self.stop(agent).returncode for _ in range(6)]
                self.assertEqual(codes, [2] * 6)

    def test_each_reviewer_with_its_valid_verdict_for_the_source_written_this_run_stops(self):
        for agent, rel, brief in [
                ("rust-review", "checkpoint/rust-review.json", None), ("contract-review", "checkpoint/contract-review.json", None),
                ("leak-review", "checkpoint/leak-review.json", None), ("tm-steward", "checkpoint/tm-steward.json", None),
                ("leak-review", "branches/claude-w15-loop.json",
                 "INTEGRATION_TREE=" + self.tree + " SOURCE=" + SHA + " BRANCH=claude/w15-loop.")]:
            with self.subTest(rel=rel):
                self.write(rel, self.review(agent))
                self.assertEqual(self.stop(agent, brief=brief).returncode, 0)

    def test_the_tree_token_drops_trailing_punctuation(self):
        self.write("checkpoint/rust-review.json", self.review("rust-review"))
        for brief in ["INTEGRATION_TREE=" + self.tree + ": SOURCE=" + SHA,
                      "INTEGRATION_TREE=" + self.tree + " SOURCE=" + SHA + ", as before: SOURCE=" + SHA + ".", "(INTEGRATION_TREE=" + self.tree + "), SOURCE=" + SHA + ".",
                      "INTEGRATION_TREE=" + self.tree + "; SOURCE=" + SHA + ";"]:
            with self.subTest(brief=brief):
                self.assertEqual(self.stop("rust-review", brief=brief).returncode, 0)

    def test_a_verdict_that_is_not_this_runs_valid_review_of_the_source_is_held(self):
        branch = "INTEGRATION_TREE=" + self.tree + " SOURCE=" + SHA + " BRANCH=claude/w15-loop"
        for name, rel, review, age, brief, stamped in [
                ("stale", "checkpoint/rust-review.json", self.review("rust-review"), 3600, None, True),
                ("another reviewer", "checkpoint/rust-review.json", self.review("leak-review"), 0, None, True),
                ("no checks", "checkpoint/rust-review.json", self.review("rust-review", checks=[]), 0, None, True),
                ("verdict not decided", "checkpoint/rust-review.json", {"reviewer": "rust-review", "verdict": "not decided"}, 0, None, True),
                ("another source", "checkpoint/rust-review.json", self.review("rust-review", source="b" * 40), 0, None, True),
                ("the quoted earlier source", "checkpoint/rust-review.json", self.review("rust-review", source="b" * 40), 0,
                 "The last pass reviewed SOURCE=" + "b" * 40 + ". Now INTEGRATION_TREE=" + self.tree + " SOURCE=" + SHA + ".", True),
                ("two sources, the verdict for the second", "checkpoint/rust-review.json", self.review("rust-review"), 0,
                 "The last pass reviewed SOURCE=" + "b" * 40 + ". Now INTEGRATION_TREE=" + self.tree + " SOURCE=" + SHA + ".", True),
                ("two sources, the verdict for the first", "checkpoint/rust-review.json", self.review("rust-review"), 0,
                 "INTEGRATION_TREE=" + self.tree + " SOURCE=" + SHA + ". Then SOURCE=" + "b" * 40 + ".", True),
                ("two trees", "checkpoint/rust-review.json", self.review("rust-review"), 0,
                 "INTEGRATION_TREE=/elsewhere SOURCE=" + SHA + " INTEGRATION_TREE=" + self.tree, True),
                ("two branches", "branches/claude-w15-loop.json", self.review("leak-review"), 0,
                 "INTEGRATION_TREE=" + self.tree + " SOURCE=" + SHA + " BRANCH=claude/old BRANCH=claude/w15-loop", True),
                ("not json", "checkpoint/rust-review.json", "{x", 0, None, True),
                ("no timestamp", "checkpoint/rust-review.json", self.review("rust-review"), 0, None, False),
                ("no SOURCE", "checkpoint/rust-review.json", self.review("rust-review"), 0, "INTEGRATION_TREE=" + self.tree, True),
                ("colon form", "checkpoint/rust-review.json", self.review("rust-review"), 0, "INTEGRATION_TREE: " + self.tree + " SOURCE=" + SHA, True),
                ("another branch's file", "branches/claude-other.json", self.review("leak-review"), 0, branch, True)]:
            with self.subTest(name=name):
                for rel_old in ("checkpoint/rust-review.json", "branches/claude-other.json", "branches/claude-w15-loop.json"):
                    path = os.path.join(self.tree, "crates/ryo/docs/contracts/evidence", rel_old)
                    if os.path.exists(path):
                        os.remove(path)
                self.write(rel, review, age)
                agent = "leak-review" if rel.startswith("branches/") else "rust-review"
                self.assertEqual(self.stop(agent, brief=brief, stamped=stamped).returncode, 2)

    def test_the_tm_steward_decides_the_landed_head_as_the_push_gate_reads_it(self):
        git = ["git", "-C", self.tree, "-c", "user.email=t@example.com", "-c", "user.name=t"]
        subprocess.run(git[:3] + ["init", "-q"], check=True)

        def commit(path, text):
            with open(os.path.join(self.tree, path), "w", encoding="utf-8") as handle:
                handle.write(text)
            subprocess.run(git + ["add", "-A"], check=True)
            subprocess.run(git + ["commit", "-q", "-m", path], check=True)
            return subprocess.run(git + ["rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
        landed = commit("code.rs", "fn a() {}")
        evidence = commit("crates/ryo/docs/contracts/evidence/checkpoint/contract-review.md", "x")
        code = commit("code.rs", "fn b() {}")
        subprocess.run(git + ["mv", "code.rs", "crates/ryo/docs/contracts/evidence/checkpoint/code.rs"], check=True)
        subprocess.run(git + ["commit", "-q", "-m", "move"], check=True)
        moved = subprocess.run(git + ["rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
        brief = lambda sha: "INTEGRATION_TREE=" + self.tree + " SOURCE=" + sha
        for name, source, decided, code_ in [("the landed head itself", landed, landed, 0),
                                             ("code renamed into the evidence directory since", moved, code, 2),
                                             ("an ancestor with only evidence changed since", evidence, landed, 0),
                                             ("an ancestor with code changed since", code, landed, 2),
                                             ("a sha that is not an ancestor", landed, code, 2)]:
            with self.subTest(name=name):
                self.write("checkpoint/tm-steward.json", self.review("tm-steward", source=decided))
                self.assertEqual(self.stop("tm-steward", brief=brief(source)).returncode, code_)

    def test_a_guard_that_fails_to_load_holds_the_tm_steward(self):
        copy = os.path.join(self.dir.name, "copy")
        os.makedirs(os.path.join(copy, ".claude", "hooks"))
        shutil.copytree(os.path.join(HERE, "..", "..", "ci", "contract_review"), os.path.join(copy, "ci", "contract_review"))
        shutil.copy(os.path.join(HERE, "acceptance_on_stop.py"), os.path.join(copy, ".claude", "hooks"))
        with open(os.path.join(copy, ".claude", "hooks", "guard_shell.py"), "w", encoding="utf-8") as handle:
            handle.write("import no_such_module_planted\n")
        self.write("checkpoint/tm-steward.json", self.review("tm-steward"))
        self.stop("tm-steward")  # writes the transcript
        transcript = os.path.join(self.dir.name, "agent.jsonl")
        result = subprocess.run(["python3", "-I", os.path.join(copy, ".claude", "hooks", "acceptance_on_stop.py")],
                                input=json.dumps({"agent_type": "tm-steward", "agent_id": "a1",
                                                  "agent_transcript_path": transcript}),
                                capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("ModuleNotFoundError", result.stderr)

    def test_blocked_and_other_agents_stop(self):
        self.assertEqual(self.stop("rust-review", report="BLOCKED: which base sha?").returncode, 0)
        self.assertEqual(self.stop("perf-diagnosis").returncode, 0)


class RealStopPayload(unittest.TestCase):
    """Each stop type driven by the payload Claude Code sends on SubagentStop, with the parent session's transcript
    in transcript_path and a last message only in last_assistant_message, so a hook reading the wrong field fails."""

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.env = dict(os.environ, CLAUDE_HOOKS_STATE=os.path.join(self.dir.name, "state"))
        self.parent = os.path.join(self.dir.name, "parent.jsonl")
        write_transcript(self.parent, "The PM session.", "Launching agents.")

    def stop(self, agent, brief, last, env=None):
        transcript = os.path.join(self.dir.name, "agent.jsonl")
        clock = __import__("datetime")
        rows = [{"type": "user", "timestamp": (clock.datetime.now(clock.timezone.utc) -
                                               clock.timedelta(seconds=60)).isoformat(),
                 "message": {"role": "user", "content": brief}},
                {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "working"}]}}]
        with open(transcript, "w", encoding="utf-8") as handle:
            handle.write("\n".join(json.dumps(r) for r in rows))
        payload = {"session_id": "s1", "transcript_path": self.parent, "hook_event_name": "SubagentStop",
                   "stop_hook_active": False, "agent_id": "a1", "agent_type": agent,
                   "agent_transcript_path": transcript, "last_assistant_message": last}
        return run_hook("acceptance_on_stop.py", payload, env or self.env)

    def test_builder(self):
        work = os.path.join(self.dir.name, "tree")
        os.mkdir(work)
        bin_dir = os.path.join(self.dir.name, "bin")
        os.mkdir(bin_dir)
        with open(os.path.join(bin_dir, "bd"), "w", encoding="utf-8") as handle:
            handle.write("#!/bin/sh\nprintf '[{\"notes\": \"ACCEPTANCE:\\\\ntest -f ok\\\\nEND ACCEPTANCE\"}]\\n'\n")
        os.chmod(os.path.join(bin_dir, "bd"), 0o755)
        env = dict(self.env, PATH=bin_dir + os.pathsep + os.environ["PATH"])
        brief = "BEAD: ryo-t1\nWORKTREE: " + work + "\nDo it."
        self.assertEqual(self.stop("builder", brief, "DONE: R1", env).returncode, 2)
        open(os.path.join(work, "ok"), "w").close()
        self.assertEqual(self.stop("builder", brief, "DONE: R1", env).returncode, 0)
        self.assertEqual(self.stop("builder", brief, "Finished.", env).returncode, 2)

    def test_contract_migration_and_integrator(self):
        work = os.path.join(self.dir.name, "tree")
        os.mkdir(work)
        bin_dir = os.path.join(self.dir.name, "bin")
        os.mkdir(bin_dir)
        with open(os.path.join(bin_dir, "bd"), "w", encoding="utf-8") as handle:
            handle.write("#!/bin/sh\nprintf '[{\"notes\": \"ACCEPTANCE:\\\\ntest -f ok\\\\nEND ACCEPTANCE\"}]\\n'\n")
        os.chmod(os.path.join(bin_dir, "bd"), 0o755)
        env = dict(self.env, PATH=bin_dir + os.pathsep + os.environ["PATH"])
        brief = "BEAD: ryo-t1\nWORKTREE: " + work + "\nDo it."
        for agent in ("contract-migration", "integrator"):
            with self.subTest(agent=agent):
                self.assertEqual(self.stop(agent, brief, "DONE: R1", env).returncode, 2)
                self.assertEqual(self.stop(agent, brief, "BLOCKED: which crate?", env).returncode, 0)
        open(os.path.join(work, "ok"), "w").close()
        for agent in ("contract-migration", "integrator"):
            with self.subTest(agent=agent, accepted=True):
                self.assertEqual(self.stop(agent, brief, "DONE: R1", env).returncode, 0)

    def test_reviewer(self):
        tree = os.path.join(self.dir.name, "ryo-333")
        os.makedirs(os.path.join(tree, "crates/ryo/docs/contracts/evidence/checkpoint"))
        brief = "INTEGRATION_TREE=" + tree + " SOURCE=" + SHA
        self.assertEqual(self.stop("rust-review", brief, "Reviewed.").returncode, 2)
        with open(os.path.join(tree, "crates/ryo/docs/contracts/evidence/checkpoint/rust-review.json"), "w",
                  encoding="utf-8") as handle:
            json.dump({"source": SHA, "verdict": "FAIL", "reviewer": "rust-review",
                       "checks": [{"id": "1", "verdict": "FAIL", "evidence": "x.rs:1"}]}, handle)
        self.assertEqual(self.stop("rust-review", brief, "Reviewed.").returncode, 0)

    def test_scope_review(self):
        scope = os.path.join(self.dir.name, "scope-ryo-x.md")
        with open(scope, "w", encoding="utf-8") as handle:
            handle.write("PROCEED\nBEADS: ryo-x\n")
        self.assertEqual(self.stop("scope-review", "Review SCOPE=" + scope, "Reviewed.").returncode, 2)
        self.assertEqual(self.stop("scope-review", "Review SCOPE=" + scope, "BLOCKED: which goal?").returncode, 0)
        with open(os.path.join(self.dir.name, "scope-review-ryo-x.json"), "w", encoding="utf-8") as handle:
            json.dump(with_digest({"rows": scope_rows(), "verdict": "PASS"}, scope), handle)
        self.assertEqual(self.stop("scope-review", "Review SCOPE=" + scope, "Reviewed.").returncode, 0)


class NoSubagentBuilders(unittest.TestCase):
    SUB = "/home/u/.claude/projects/p/s1/subagents/agent-a1.jsonl"
    MAIN = "/home/u/.claude/projects/p/s1.jsonl"

    def spawn(self, tool, agent_id=None):
        """Claude Code's PreToolUse payload: transcript_path is the session's in a subagent too; agent_id is set only
        when a subagent makes the call."""
        payload = {"tool_name": tool, "tool_input": {}, "transcript_path": self.MAIN, "hook_event_name": "PreToolUse"}
        if agent_id:
            payload.update(agent_id=agent_id, agent_type="builder")
        return run_hook("no_subagent_builders.py", payload)

    def test_refused_from_subagent(self):
        result = self.spawn("Agent", agent_id="a1")
        self.assertEqual(result.returncode, 2)
        self.assertIn("Blocked: a builder never spawns subagents", result.stderr)

    def test_allowed_from_main_session(self):
        self.assertEqual(self.spawn("Agent").returncode, 0)

    def test_allowed_for_other_tool(self):
        self.assertEqual(self.spawn("Bash", agent_id="a1").returncode, 0)

    def test_allowed_on_malformed_input(self):
        result = subprocess.run(["python3", "-I", os.path.join(HERE, "no_subagent_builders.py")],
                                input="not json", capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0)


def rubric_rows():
    """The scope review's rubric rows, read by the gate's own reader (no copy of the list here)."""
    import importlib.util
    spec = importlib.util.spec_from_file_location("gate", os.path.join(HERE, "no_subagent_builders.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.rubric_rows()


def with_digest(review, scope):
    """review with scope_sha256 set to the scope report's sha256, unless the test set it."""
    if isinstance(review, dict) and "scope_sha256" not in review:
        with open(scope, "rb") as handle:
            review = dict(review, scope_sha256=__import__("hashlib").sha256(handle.read()).hexdigest())
    return review


def scope_rows(failing=()):
    return [{"id": row, "result": "FAIL" if row in failing else "PASS", "reason": "x.rs:1 holds"}
            for row in rubric_rows()]


class ScopeGate(unittest.TestCase):
    MAIN = "/home/u/.claude/projects/p/s1.jsonl"

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.tree = repo(self.dir.name, "tree", "claude/p1-scoped")
        self.env = dict(os.environ, CLAUDE_HOOKS_STATE=os.path.join(self.dir.name, "state"))

    def scope(self, first, review, bead="ryo-x", beads="ryo-x"):
        """scope-<bead>.md: first, then `BEADS: <beads>` as line 2 (or first as written when beads is None), and a
        body that names ryo-x, so only line 2 can admit a bead."""
        path = os.path.join(self.dir.name, "scope-" + bead + ".md")
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(first + ("" if beads is None else "\nBEADS: " + beads) +
                         "\n\nThe scope of ryo-x.\n\n## Scope review\nS1 PASS\n")
        with open(os.path.join(self.dir.name, "scope-review-" + bead + ".json"), "w", encoding="utf-8") as handle:
            json.dump(with_digest(review, path), handle)
        return path

    def launch(self, prompt, kind="builder"):
        tool_input = {"prompt": prompt, "description": "work"}
        if kind:
            tool_input["subagent_type"] = kind
        payload = {"tool_name": "Agent", "transcript_path": self.MAIN, "hook_event_name": "PreToolUse",
                   "tool_input": tool_input}
        return run_hook("no_subagent_builders.py", payload, env=self.env)

    def brief(self, extra, bead="ryo-x", tree=None):
        return "BEAD: " + bead + "\nWORKTREE: " + (tree or self.tree) + "\n" + extra + "\nR1 ..."

    def test_refuses_a_builder_without_a_scope(self):
        result = self.launch(self.brief(""))
        self.assertEqual(result.returncode, 2)
        self.assertIn("SCOPE=<path", result.stderr)

    def test_refuses_a_scope_whose_review_failed(self):
        path = self.scope("PROCEED: do it", {"rows": scope_rows({"S2"}), "verdict": "FAIL"})
        self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 2)

    def test_refuses_a_failed_row_under_a_pass_verdict(self):
        path = self.scope("PROCEED: do it", {"rows": scope_rows({"S5"}), "verdict": "PASS"})
        self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 2)

    def test_refuses_a_dropped_scope(self):
        for rows in (scope_rows(), [r for r in scope_rows() if r["id"] in ("S1", "S10")]):
            with self.subTest(rows=len(rows)):
                path = self.scope("DROP: the rewrite deletes this code", {"rows": rows, "verdict": "PASS"})
                self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 2)

    def test_allows_a_passed_proceed_or_reshape(self):
        for verdict in ("PROCEED", "RESHAPE: one crate, not two"):
            with self.subTest(verdict=verdict):
                path = self.scope(verdict, {"rows": scope_rows(), "verdict": "PASS"})
                self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 0)

    def test_allows_a_followup_only_on_a_branch_launched_with_a_passed_scope(self):
        self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p1-scoped")).returncode, 2)
        path = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"})
        self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 0)
        self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p1-scoped")).returncode, 0)
        self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p2-other")).returncode, 2)

    def test_refuses_a_scope_for_another_bead(self):
        passed = {"rows": scope_rows(), "verdict": "PASS"}
        for bead, beads in [("ryo-old", "ryo-old"), ("ryo-x", "ryo-old"), ("ryo-x", "ryo-x.1")]:
            with self.subTest(bead=bead, beads=beads):
                path = self.scope("PROCEED", passed, bead=bead, beads=beads)
                self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 2)

    def test_a_scope_admits_only_the_beads_of_its_second_line(self):
        passed = {"rows": scope_rows(), "verdict": "PASS"}
        for beads, code in [("ryo-x", 0), ("ryo-a ryo-x ryo-b", 0), ("ryo-a ryo-b", 2), ("ryo-c2 ryo-c3", 2)]:
            with self.subTest(beads=beads):
                path = self.scope("PROCEED", passed, beads=beads)
                self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, code)
        for top in ["PROCEED", "PROCEED\n\nBEADS: ryo-x", "PROCEED\n**BEADS:** ryo-x", "PROCEED\n`BEADS:` ryo-x",
                    "PROCEED\nBeads: ryo-x", "PROCEED\n  BEADS: ryo-x", "PROCEED\n- BEADS: ryo-x",
                    "\nPROCEED\nBEADS: ryo-x", "PROCEED\nscope of cp334\nBEADS: ryo-x"]:
            with self.subTest(top=top):
                path = self.scope(top, passed, beads=None)
                self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 2)

    def test_a_brief_with_several_beads_needs_every_one_scoped(self):
        both = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"}, beads="ryo-x ryo-y")
        self.assertEqual(self.launch(self.brief("SCOPE=" + both)).returncode, 0)  # records ryo-x only
        for beads in ("ryo-x ryo-y", "ryo-x\nBEAD: ryo-y"):
            with self.subTest(followup=beads):
                self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p1-scoped", bead=beads)).returncode, 2)
        one = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"}, beads="ryo-x")
        for beads in ("ryo-x ryo-y", "ryo-x\nBEAD: ryo-y"):
            with self.subTest(scope="ryo-x", beads=beads):
                self.assertEqual(self.launch(self.brief("SCOPE=" + one, bead=beads)).returncode, 2)
        both = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"}, beads="ryo-x ryo-y")
        for beads in ("ryo-x ryo-y", "ryo-x\nBEAD: ryo-y"):
            with self.subTest(scope="ryo-x ryo-y", beads=beads):
                self.assertEqual(self.launch(self.brief("SCOPE=" + both, bead=beads)).returncode, 0)
                self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p1-scoped", bead=beads)).returncode, 0)
        self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p1-scoped", bead="ryo-x ryo-z")).returncode, 2)

    def test_a_builder_brief_without_exact_bead_and_worktree_lines_is_refused(self):
        path = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"})
        for name, brief in malformed_briefs(self.tree).items():
            for kind in ("builder", "contract-migration"):
                with self.subTest(brief=name, kind=kind):
                    result = self.launch(brief.format(scope=path), kind=kind)
                    self.assertEqual(result.returncode, 2)
                    self.assertIn("WORKTREE: /", result.stderr)
            for kind in ("general-purpose", "Explore", "Plan", "fork", None):
                with self.subTest(brief=name, kind=kind):
                    result = self.launch(brief.format(scope=path), kind=kind)
                    if name == "Bead:":  # no exact BEAD: line, so not a builder brief
                        self.assertEqual(result.returncode, 0)
                    else:
                        self.assertEqual(result.returncode, 2)
                        self.assertIn("subagent_type builder", result.stderr)
        self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 0)

    def test_a_scope_that_is_not_utf8_is_judged_not_crashed(self):
        path = os.path.join(self.dir.name, "scope-ryo-x.md")
        with open(path, "wb") as handle:
            handle.write(b"PROCEED\nBEADS: ryo-x\n\nThe scope, \xff\xfe not UTF-8.\n")
        with open(os.path.join(self.dir.name, "scope-review-ryo-x.json"), "w", encoding="utf-8") as handle:
            json.dump(with_digest({"rows": scope_rows(), "verdict": "PASS"}, path), handle)
        self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 0)

    def test_a_contract_migration_launch_needs_a_scope_like_a_builder(self):
        self.assertEqual(self.launch(self.brief(""), kind="contract-migration").returncode, 2)
        path = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"})
        self.assertEqual(self.launch(self.brief("SCOPE=" + path), kind="contract-migration").returncode, 0)

    def test_refuses_a_scope_changed_after_its_review(self):
        path = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"})
        with open(path, "a", encoding="utf-8") as handle:
            handle.write("A re-scope after the review.\n")
        self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 2)
        path = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS", "scope_sha256": None})
        self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 2)

    def test_a_followup_needs_its_worktree_on_that_branch_and_the_scoped_bead(self):
        other = repo(self.dir.name, "other", "claude/never-scoped")
        path = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"})
        self.assertEqual(self.launch(self.brief("SCOPE=" + path)).returncode, 0)
        self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p1-scoped", tree=other)).returncode, 2)
        self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p1-scoped", bead="ryo-y")).returncode, 2)
        self.assertEqual(self.launch(self.brief("FOLLOWUP=claude/p1-scoped")).returncode, 0)

    def test_the_rows_come_from_the_rubric_so_a_rubric_change_needs_no_hook_edit(self):
        copy = os.path.join(self.dir.name, "copy")
        os.makedirs(os.path.join(copy, ".claude", "hooks"))
        os.makedirs(os.path.join(copy, ".agents", "agents"))
        shutil.copy(os.path.join(HERE, "no_subagent_builders.py"), os.path.join(copy, ".claude", "hooks"))
        rubric = os.path.join(copy, ".agents", "agents", "scope-review.md")

        def launch(rows, table):
            if table is not None:
                with open(rubric, "w", encoding="utf-8") as handle:
                    handle.write("## Rubric\n| Row | PASS when |\n|---|---|\n" + "".join("| " + r + " Name | x |\n" for r in table))
            path = self.scope("PROCEED", {"rows": [{"id": r, "result": "PASS", "reason": "x"} for r in rows],
                                          "verdict": "PASS"})
            payload = {"tool_name": "Agent", "transcript_path": self.MAIN,
                       "tool_input": {"subagent_type": "builder", "prompt": self.brief("SCOPE=" + path)}}
            return subprocess.run(["python3", "-I", os.path.join(copy, ".claude", "hooks", "no_subagent_builders.py")],
                                  input=json.dumps(payload), capture_output=True, text=True, env=self.env,
                                  check=False).returncode
        ten = ["S%d" % n for n in range(1, 11)]
        self.assertEqual(launch(ten, ten), 0)
        self.assertEqual(launch(ten[:9], ten), 2)
        self.assertEqual(launch(["S1", "S2", "S2b"], ["S1", "S2", "S2b"]), 0)
        os.remove(rubric)
        self.assertEqual(launch(ten, None), 2)

    def test_other_agents_need_no_scope(self):
        self.assertEqual(self.launch("review sha x", kind="leak-review").returncode, 0)

    def test_a_builder_brief_launches_only_as_an_agent_whose_stop_runs_acceptance(self):
        path = self.scope("PROCEED", {"rows": scope_rows(), "verdict": "PASS"})
        for kind in ("general-purpose", "fork", "Explore", None):
            for extra in ("", "SCOPE=" + path):
                with self.subTest(kind=kind, extra=extra):
                    result = self.launch(self.brief(extra), kind=kind)
                    self.assertEqual(result.returncode, 2)
                    self.assertIn("subagent_type builder", result.stderr)
        self.assertEqual(self.launch(self.brief(""), kind="integrator").returncode, 0)
        self.assertEqual(self.launch(self.brief("SCOPE=" + path), kind="builder").returncode, 0)
        self.assertEqual(self.launch("BEAD: ryo-x\nSurvey the bead.", kind="general-purpose").returncode, 2)
        self.assertEqual(self.launch("Survey bead ryo-x.", kind="general-purpose").returncode, 0)


class ScopeReviewStop(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.scope = os.path.join(self.dir.name, "scope-ryo-x.md")
        self.review = os.path.join(self.dir.name, "scope-review-ryo-x.json")
        with open(self.scope, "w", encoding="utf-8") as handle:
            handle.write("PROCEED\n\nBead ryo-x.\n")

    def write(self, review, age=0):
        with open(self.review, "w", encoding="utf-8") as handle:
            handle.write(review if isinstance(review, str) else json.dumps(with_digest(review, self.scope)))
        if age:
            stamp = os.path.getmtime(self.review) - age
            os.utime(self.review, (stamp, stamp))

    def stop(self, report="Scope reviewed."):
        transcript = os.path.join(self.dir.name, "agent.jsonl")
        clock = __import__("datetime")
        now = clock.datetime.now(clock.timezone.utc) - clock.timedelta(seconds=60)
        rows = [{"type": "user", "timestamp": now.isoformat(),
                 "message": {"role": "user", "content": "Review SCOPE=" + self.scope + "."}},
                {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": report}]}}]
        with open(transcript, "w", encoding="utf-8") as handle:
            handle.write("\n".join(json.dumps(r) for r in rows))
        env = dict(os.environ, CLAUDE_HOOKS_STATE=os.path.join(self.dir.name, "state"))
        return run_hook("acceptance_on_stop.py", {"agent_type": "scope-review", "agent_id": "a1",
                                                  "agent_transcript_path": transcript}, env)

    def test_held_until_a_valid_review_is_written_this_run(self):
        self.assertEqual(self.stop().returncode, 2)
        for name, review, age in [("stale", {"rows": scope_rows(), "verdict": "PASS"}, 3600),
                                  ("not json", "{x", 0),
                                  ("a row missing", {"rows": scope_rows()[:2] + scope_rows()[3:], "verdict": "PASS"}, 0),
                                  ("verdict against its rows", {"rows": scope_rows({"S3"}), "verdict": "PASS"}, 0),
                                  ("another report's digest", {"rows": scope_rows(), "verdict": "PASS",
                                                               "scope_sha256": "0" * 64}, 0)]:
            with self.subTest(name=name):
                self.write(review, age)
                self.assertEqual(self.stop().returncode, 2)

    def test_stops_on_a_valid_review_of_either_verdict_or_on_blocked(self):
        for review in ({"rows": scope_rows(), "verdict": "PASS"}, {"rows": scope_rows({"S8"}), "verdict": "FAIL"}):
            with self.subTest(verdict=review["verdict"]):
                self.write(review)
                self.assertEqual(self.stop().returncode, 0)
        os.remove(self.review)
        self.assertEqual(self.stop(report="BLOCKED: which goal?").returncode, 0)

    def test_a_missing_report_or_digest_holds(self):
        self.write({"rows": scope_rows(), "verdict": "PASS", "scope_sha256": None})
        self.assertEqual(self.stop().returncode, 2)
        os.remove(self.scope)
        self.write({"rows": scope_rows(), "verdict": "PASS", "scope_sha256": None})
        self.assertEqual(self.stop().returncode, 2)
        with open(self.review, "w", encoding="utf-8") as handle:
            json.dump({"rows": scope_rows(), "verdict": "PASS"}, handle)
        self.assertEqual(self.stop().returncode, 2)

    def test_a_scope_that_is_not_utf8_stops_on_a_valid_review(self):
        with open(self.scope, "wb") as handle:
            handle.write(b"PROCEED\n\nBead ryo-x, \xff\xfe not UTF-8.\n")
        self.write({"rows": scope_rows(), "verdict": "PASS"})
        self.assertEqual(self.stop().returncode, 0)

    def test_a_defer_or_drop_review_grades_s1_and_s10_only(self):
        short = [r for r in scope_rows({"S10"}) if r["id"] in ("S1", "S10")]
        for verdict in ("DEFER: until the rewrite lands", "DROP: the rewrite deletes this code"):
            with self.subTest(verdict=verdict):
                with open(self.scope, "w", encoding="utf-8") as handle:
                    handle.write(verdict + "\n")
                self.write({"rows": short, "verdict": "FAIL"})
                self.assertEqual(self.stop().returncode, 0)
                self.write({"rows": scope_rows(), "verdict": "PASS"})
                self.assertEqual(self.stop().returncode, 2)
        with open(self.scope, "w", encoding="utf-8") as handle:
            handle.write("PROCEED\n")
        self.write({"rows": short, "verdict": "FAIL"})
        self.assertEqual(self.stop().returncode, 2)


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


MANIFEST = """# the test manifest
tastemaker = "0.1.2"
variables = { internal-packages = ["pkg-a", "pkg-b"] }
workspace = { name = "x", members = ["crates/a", "crates/b"], rules-exclude = ["target/**", "vendor/**"] }
rules = [
  { rule = "idiomatic.arch/no-language-names", include = ["src/**", "tests/**"], exclude = ["src/gen/**", "src/old/**"], params = { max-lines = 400, allowed-packages = ["serde", "toml"] }, description = "No language names." },
  { rule = "idiomatic.arch/second", include = ["src/**"] },
]
specs = [
  { spec = "idiomatic.arch/concept", include = ["."], imports = { allow = ["std", "core"] }, except = ["src/legacy.rs"], description = "The concept." },
]
templates = [
  { template = "idiomatic.rust/adapter", file = "src/adapter.rs", description = "The adapter." },
  { template = "idiomatic.rust/module", file = "src/**/mod.rs", description = "Every module." },
  { template = "idiomatic.rust/changelog", file = "CHANGELOG.md", count = { min = 0 } },
]
"""
RULE = '{ rule = "idiomatic.arch/no-language-names", '
SECOND = '  { rule = "idiomatic.arch/second", include = ["src/**"] },\n'
SPEC = '  { spec = "idiomatic.arch/concept", '
ADAPTER = '  { template = "idiomatic.rust/adapter", file = "src/adapter.rs", description = "The adapter." },\n'
MODULE = '  { template = "idiomatic.rust/module", file = "src/**/mod.rs", description = "Every module." },\n'
SOURCE = ("fn a() {}\n// ignore: idiomatic.arch/one - the reason\n"
          "fn b() {} // ignore: idiomatic.arch/two - the reason\n")
SCRIPT = "#!/bin/sh\n# ignore: idiomatic.arch/three - the reason\necho hi\n"
ROUTE = ("git commit -F <file>", "gh pr create --body-file <file>")
ADMIT_EXCLUDE = """[[manifest]]
path = "crates/x/tastemaker.toml"
change = "added"
fact = "rules[idiomatic.arch/no-language-names].exclude"
value = "src/new/**"
reason = "the owner admits it"
"""


class ClosedForExtension(unittest.TestCase):
    """Tastemaker manifests are closed for extension: an agent's write may tighten them, never extend or loosen them,
    and adds no in-source ignore comment, unless the owner's allowlist admits the change."""

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)
        self.hooks = os.path.join(self.dir.name, "hooks")
        shutil.copytree(HERE, self.hooks, ignore=shutil.ignore_patterns("__pycache__", "test_*"))
        self.allowlist = os.path.join(self.hooks, "closed_for_extension.toml")
        self.tree = repo(self.dir.name, "tree", "claude/closed-x")
        self.manifest = os.path.join(self.tree, "crates", "x", "tastemaker.toml")
        self.source = os.path.join(self.tree, "src", "lib.rs")
        self.script = os.path.join(self.tree, "scripts", "run.sh")
        self.put(self.manifest, MANIFEST)
        self.put(self.source, SOURCE)
        self.put(self.script, SCRIPT)

    def put(self, path, text):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(text)

    def hook(self, name, payload):
        payload = dict(payload, cwd=self.tree)
        return subprocess.run(["python3", "-I", os.path.join(self.hooks, name)], input=json.dumps(payload),
                              capture_output=True, text=True, check=False, cwd=self.tree)

    def write(self, path, content):
        return self.hook("guard_write.py", {"tool_name": "Write", "tool_input": {"file_path": path, "content": content}})

    def edit(self, path, old, new):
        return self.hook("guard_write.py", {"tool_name": "Edit",
                                            "tool_input": {"file_path": path, "old_string": old, "new_string": new}})

    def bash(self, command):
        return self.hook("guard_shell.py", {"tool_name": "Bash", "tool_input": {"command": command}})

    def both(self, old, new, path=None):
        """The Write of the whole changed file and the Edit of the fragment, each run on the unchanged file."""
        path = path or self.manifest
        with open(path, encoding="utf-8") as handle:
            before = handle.read()
        self.assertIn(old, before)
        return {"Write": self.write(path, before.replace(old, new, 1)), "Edit": self.edit(path, old, new)}

    def assert_codes(self, code, changes, path=None, named=()):
        for old, new in changes:
            for tool, done in self.both(old, new, path).items():
                with self.subTest(tool=tool, old=old, new=new):
                    self.assertEqual(done.returncode, code, done.stderr)
                    printed = self.printed_changes(done)
                    for change in named:
                        self.assertIn(change, printed, done.stderr)

    def refused(self, *changes, path=None, named=()):
        """Each change is refused by Write and by Edit, and every change in named is among the printed refusals."""
        self.assert_codes(2, changes, path, named)

    def allowed(self, *changes, path=None):
        self.assert_codes(0, changes, path)

    def assert_commands(self, code, commands, says=None):
        for command in commands:
            with self.subTest(command=command):
                done = self.bash(command)
                self.assertEqual(done.returncode, code, done.stderr)
                if says:
                    self.assertIn(says, done.stderr)

    def printed_blocks(self, done):
        """The [[manifest]] or [[ignore]] blocks a refusal prints for the owner to paste."""
        return re.findall(r"^\[\[(?:manifest|ignore)\]\]\n(?:[a-z]+ = .*\n?)+", done.stderr, re.MULTILINE)

    def printed_changes(self, done):
        """(change, fact, value) of every [[manifest]] block a refusal prints."""
        return [(e["change"], e["fact"], e["value"]) for b in self.printed_blocks(done)
                for e in tomllib.loads(b).get("manifest", [])]

    # refused

    def test_an_added_literal_template_placement_is_refused(self):
        self.refused((ADAPTER, ADAPTER + '  { template = "idiomatic.rust/adapter", file = "src/myname.rs" },\n'),
                     named=[("added", "templates[idiomatic.rust/adapter]", "<entry>"),
                            ("added", "templates[idiomatic.rust/adapter].file", "src/myname.rs")])

    def test_an_added_glob_template_placement_is_refused(self):
        self.refused((ADAPTER, ADAPTER + '  { template = "idiomatic.rust/adapter", file = "src/*.rs" },\n'),
                     named=[("added", "templates[idiomatic.rust/adapter]", "<entry>"),
                            ("added", "templates[idiomatic.rust/adapter].file", "src/*.rs")])
        self.refused((ADAPTER, ADAPTER + '  { template = "idiomatic.rust/adapter", file = "src/myname.r[s]" },\n'),
                     named=[("added", "templates[idiomatic.rust/adapter].file", "src/myname.r[s]")])

    def test_an_added_include_is_refused(self):
        self.refused(('include = ["src/**", "tests/**"]', 'include = ["src/**", "tests/**", "benches/**"]'))

    def test_an_added_exclude_is_refused(self):
        self.refused(('exclude = ["src/gen/**", "src/old/**"]', 'exclude = ["src/gen/**", "src/old/**", "src/new/**"]'))

    def test_an_added_except_or_allow_entry_is_refused(self):
        self.refused(('allow = ["std", "core"]', 'allow = ["std", "core", "alloc"]'),
                     ('except = ["src/legacy.rs"]', 'except = ["src/legacy.rs", "src/other.rs"]'),
                     ('allowed-packages = ["serde", "toml"]', 'allowed-packages = ["serde", "toml", "rand"]'))

    def test_an_added_rules_exclude_glob_is_refused(self):
        self.refused(('rules-exclude = ["target/**", "vendor/**"]', 'rules-exclude = ["target/**", "vendor/**", "build/**"]'))

    def test_an_added_variable_entry_is_refused(self):
        self.refused(('["pkg-a", "pkg-b"]', '["pkg-a", "pkg-b", "pkg-c"]'))

    def test_a_new_rule_entry_is_refused(self):
        self.refused((SECOND, SECOND + '  { rule = "idiomatic.arch/third", include = ["src/**"] },\n'),
                     named=[("added", "rules[idiomatic.arch/third]", "<entry>")])

    def test_a_new_spec_entry_is_refused(self):
        self.refused(("specs = [\n", 'specs = [\n  { spec = "idiomatic.arch/other", include = ["."] },\n'),
                     named=[("added", "specs[idiomatic.arch/other]", "<entry>")])

    def test_a_loosened_params_scalar_is_refused(self):
        self.refused(("max-lines = 400", "max-lines = 800"))

    def test_a_new_manifest_file_is_refused(self):
        self.assertEqual(self.write(os.path.join(self.tree, "crates", "y", "tastemaker.toml"), MANIFEST).returncode, 2)

    def test_a_removed_rule_entry_is_refused(self):
        self.refused((SECOND, ""), named=[("removed", "rules[idiomatic.arch/second]", "<entry>")])

    def test_a_removed_include_is_refused(self):
        self.refused(('include = ["src/**", "tests/**"]', 'include = ["src/**"]'))

    def test_a_removed_variable_element_is_refused(self):
        self.refused(('["pkg-a", "pkg-b"]', '["pkg-a"]'))

    def test_a_removed_or_changed_params_value_is_refused(self):
        self.refused(("max-lines = 400, ", ""), ("max-lines = 400", "max-lines = 300"),
                     ('allowed-packages = ["serde", "toml"]', 'allowed-packages = ["serde"]'))

    def test_a_removed_or_changed_count_is_refused(self):
        self.refused((', count = { min = 0 }', ""), ("count = { min = 0 }", "count = { min = 1 }"))

    def test_a_removed_workspace_member_is_refused(self):
        self.refused(('members = ["crates/a", "crates/b"]', 'members = ["crates/a"]'))

    def test_a_removed_spec_entry_is_refused(self):
        self.refused(("specs = [\n" + SPEC, "specs = [\n  # " + SPEC),
                     named=[("removed", "specs[idiomatic.arch/concept]", "<entry>")])

    def test_a_removed_glob_template_placement_is_refused(self):
        self.refused((MODULE, ""))

    def test_an_unparsable_manifest_write_is_refused(self):
        self.refused(("rules = [\n", "rules = [[[\n"))

    def test_an_edit_whose_old_string_does_not_match_a_manifest_is_refused(self):
        self.assertEqual(self.edit(self.manifest, "no such text", "x = 1").returncode, 2)
        self.assertEqual(self.edit(self.source, "no such text", "// ignore: idiomatic.arch/x - why").returncode, 2)
        self.assertEqual(self.edit(self.source, "no such text", "fn c() {}").returncode, 0)

    def test_an_added_ignore_comment_is_refused(self):
        self.refused(("fn a() {}\n", "fn a() {}\n// ignore: idiomatic.arch/new - the reason\n"),
                     ("fn a() {}", "fn a() {} // ignore: idiomatic.arch/new - the reason"), path=self.source)
        self.refused(("echo hi\n", "echo hi # ignore: idiomatic.arch/new - the reason\n"), path=self.script)
        cargo = os.path.join(self.tree, "Cargo.toml")
        self.put(cargo, '[package]\nname = "x"\n')
        self.refused(('name = "x"\n', 'name = "x"\n# ignore: idiomatic.arch/new - the reason\n'), path=cargo)

    def test_an_added_ignore_comment_inside_a_manifest_is_refused(self):
        self.refused((SECOND, SECOND + "  # ignore: idiomatic.arch/new - the reason\n"),
                     ('tastemaker = "0.1.2"\n', 'tastemaker = "0.1.2"  # ignore: idiomatic.arch/new - the reason\n'))

    def test_a_write_to_the_allowlist_is_refused(self):
        self.assertEqual(self.write(self.allowlist, ADMIT_EXCLUDE).returncode, 2)
        self.put(self.allowlist, "# the owner's\n")
        self.assertEqual(self.edit(self.allowlist, "# the owner's\n", ADMIT_EXCLUDE).returncode, 2)
        self.assertEqual(self.write(os.path.join(self.tree, "closed_for_extension.toml"), "").returncode, 2)
        self.assert_commands(2, ["echo x > " + self.allowlist, "cp a.toml " + self.allowlist,
                                 "cp a.toml " + self.hooks + "/closed_for_extension.toml"])

    def test_a_bash_write_to_a_manifest_is_refused(self):
        self.assert_commands(2, [
            "sed -i s/a/b/ crates/x/tastemaker.toml",
            "perl -pi -e 's/a/b/' crates/x/tastemaker.toml",
            "cat > crates/x/tastemaker.toml <<'EOF'\ntastemaker = \"0.1.2\"\nEOF",
            "echo x | tee crates/x/tastemaker.toml",
            "python3 -c \"open('crates/x/tastemaker.toml', 'w').write('')\"",
            "python3 - <<'EOF'\nopen('crates/x/tastemaker.toml', 'w').write('')\nEOF",
            "git show HEAD:crates/x/tastemaker.toml > crates/x/tastemaker.toml"])
        # the shell writes the hook reads as written paths name the manifest in their refusal
        self.assert_commands(2, [
            "sed -i s/a/b/ crates/x/tastemaker.toml", "perl -pi -e 's/a/b/' crates/x/tastemaker.toml",
            "echo x | tee crates/x/tastemaker.toml", "python3 -c \"open('crates/x/tastemaker.toml', 'w').write('')\"",
            "git show HEAD:crates/x/tastemaker.toml > crates/x/tastemaker.toml"],
            says="crates/x/tastemaker.toml is a Tastemaker manifest")

    def test_digit_bundled_in_place_flags_are_refused(self):
        self.assert_commands(2, [
            "perl -0pi -e 's/a/b/' crates/x/tastemaker.toml",
            "perl -0pi -e 's/$/ # ignore: idiomatic.x/' a.py",
            "sed -Ei s/a/b/ crates/x/tastemaker.toml",
            "perl -0pi -e 's/a/b/' /tmp/claude-scratch/leak-review.json"])

    def test_node_and_deno_eval_writes_are_refused(self):
        self.assert_commands(2, [
            "node -e \"fs.writeFileSync('crates/x/tastemaker.toml', s)\"",
            "node --eval \"require('fs').writeFileSync('crates/x/tastemaker.toml', s)\"",
            "deno eval \"Deno.writeTextFileSync('crates/x/tastemaker.toml', s)\"",
            "node -e \"fs.appendFileSync('a.rs', '// ignore: idiomatic.x')\""])

    def test_an_in_place_editor_driven_by_xargs_or_find_is_refused(self):
        self.assert_commands(2, [
            "git ls-files '*tastemaker.toml' | xargs sed -i s/a/b/",
            "find . -name tastemaker.toml -exec sed -i s/a/b/ {} +",
            "git ls-files '*.rs' | xargs sed -i s/a/b/"])

    def test_git_apply_git_am_and_patch_are_refused(self):
        self.assert_commands(2, ["git apply fix.patch", "git -C t am x.mbox", "patch -p1 < x.diff"])

    def test_a_bash_write_adding_an_ignore_line_is_refused(self):
        self.assert_commands(2, [
            "cat >> Cargo.toml <<EOF\n# ignore: idiomatic.x - why\nEOF",
            "sed -i 's/$/  # ignore: idiomatic.x/' a.py",
            "echo '// ignore: idiomatic.x' >> a.rs"])

    def test_writing_a_patch_file_naming_a_manifest_is_refused(self):
        patch = "--- a/crates/x/tastemaker.toml\n+++ b/crates/x/tastemaker.toml\n@@ -1 +1 @@\n-a\n+b\n"
        self.assertEqual(self.write(os.path.join(self.tree, "fix.patch"), patch).returncode, 2)

    def test_writing_a_script_that_writes_a_manifest_is_refused(self):
        body = "open('crates/x/tastemaker.toml', 'w').write(text)\n"
        self.assertEqual(self.write(os.path.join(self.tree, "fix.py"), body).returncode, 2)
        shell_body = "#!/bin/sh\npython3 -c \"open('crates/x/tastemaker.toml', 'w').write('')\"\n"
        self.assertEqual(self.write(os.path.join(self.tree, "fix"), shell_body).returncode, 2)

    def test_a_shell_script_writing_a_manifest_with_a_shell_writer_is_refused(self):
        for name, body in (("fix.sh", "sed -i s/a/b/ crates/x/tastemaker.toml\n"),
                           ("fix.bash", "echo x | tee crates/x/tastemaker.toml\n"),
                           ("fix.zsh", "cat > crates/x/tastemaker.toml <<'EOF'\nx = 1\nEOF\n"),
                           ("fix", "#!/bin/sh\nsed -i s/a/b/ crates/x/tastemaker.toml\n"),
                           ("fix-env", "#!/usr/bin/env bash\nperl -0pi -e 's/a/b/' crates/x/tastemaker.toml\n")):
            with self.subTest(name=name):
                done = self.write(os.path.join(self.tree, name), body)
                self.assertEqual(done.returncode, 2, done.stderr)

    def test_tm_add_init_and_update_are_refused(self):
        self.assert_commands(2, [
            "tm add idiomatic.arch/x --at src/x.rs", "tm add x --at src/x.rs -n", "tm --root d add x --at src/x.rs",
            "tm --root=d add x --at y", "tm init", "tm --root d init", "tm update", "tm --root d update",
            "flock l tm update"])

    def test_a_commit_message_naming_a_manifest_and_a_writer_word_is_refused_with_the_file_route_named(self):
        for command in ["git commit -m \"drop the cp exclude from tastemaker.toml\"",
                        "git commit -F - <<EOF\nthe # ignore: idiomatic.x -> removed\nEOF",
                        "gh pr create --body \"the tastemaker.toml exclude, cp of it\""]:
            with self.subTest(command=command):
                done = self.bash(command)
                self.assertEqual(done.returncode, 2, done.stderr)
                for route in ROUTE:
                    self.assertIn(route, done.stderr)

    # passed

    def test_each_tightening_removal_is_allowed(self):
        self.allowed(('exclude = ["src/gen/**", "src/old/**"]', 'exclude = ["src/gen/**"]'),
                     ('except = ["src/legacy.rs"]', "except = []"),
                     ('allow = ["std", "core"]', 'allow = ["std"]'),
                     ('rules-exclude = ["target/**", "vendor/**"]', 'rules-exclude = ["target/**"]'),
                     (ADAPTER, ""))
        self.allowed(("// ignore: idiomatic.arch/one - the reason\n", ""),
                     (" // ignore: idiomatic.arch/two - the reason", ""), path=self.source)
        self.allowed(("# ignore: idiomatic.arch/three - the reason\n", ""), path=self.script)

    def test_reorder_reformat_comment_and_description_edits_are_allowed(self):
        self.allowed(('exclude = ["src/gen/**", "src/old/**"]', 'exclude = ["src/old/**", "src/gen/**"]'),
                     ('include = ["src/**", "tests/**"]', 'include = [\n    "tests/**",\n    "src/**",\n  ]'),
                     ("# the test manifest\n", "# the test manifest, with a comment\n# and another\n"),
                     ('description = "No language names."', 'description = "No language names, said better."'),
                     ('{ rule = "idiomatic.arch/second", include = ["src/**"] }',
                      '{ include = ["src/**"], rule = "idiomatic.arch/second" }'),
                     (SECOND, SECOND.replace(" }", ', description = "A new description." }')))

    def test_an_allowlisted_manifest_addition_is_allowed(self):
        self.put(self.allowlist, ADMIT_EXCLUDE)
        self.allowed(('exclude = ["src/gen/**", "src/old/**"]', 'exclude = ["src/gen/**", "src/old/**", "src/new/**"]'))
        self.refused(('exclude = ["src/gen/**", "src/old/**"]', 'exclude = ["src/gen/**", "src/old/**", "src/other/**"]'))

    def test_an_allowlisted_manifest_removal_is_allowed(self):
        done = self.edit(self.manifest, SECOND, "")
        self.assertEqual(done.returncode, 2)
        blocks = self.printed_blocks(done)
        self.assertTrue(blocks, done.stderr)
        admitted = "\n".join(b.replace('reason = ""', 'reason = "the owner drops the rule"') for b in blocks)
        self.assertIn('change = "removed"', admitted)
        self.put(self.allowlist, admitted)
        self.allowed((SECOND, ""))
        self.put(self.allowlist, admitted.replace('change = "removed"', 'change = "added"'))
        self.refused((SECOND, ""))

    def test_an_allowlisted_ignore_line_is_allowed(self):
        line = "// ignore: idiomatic.arch/new - the reason"
        self.put(self.allowlist, '[[ignore]]\npath = "src/lib.rs"\nline = "' + line + '"\nreason = "the owner admits it"\n')
        self.allowed(("fn a() {}\n", "fn a() {}\n" + line + "\n"), path=self.source)
        self.refused(("fn a() {}\n", "fn a() {}\n// ignore: idiomatic.arch/other - the reason\n"), path=self.source)

    def test_an_allowlist_entry_without_reason_or_change_admits_nothing(self):
        added = ('exclude = ["src/gen/**", "src/old/**"]', 'exclude = ["src/gen/**", "src/old/**", "src/new/**"]')
        for entry in (ADMIT_EXCLUDE.replace('reason = "the owner admits it"\n', ""),
                      ADMIT_EXCLUDE.replace('change = "added"\n', "")):
            with self.subTest(entry=entry):
                self.put(self.allowlist, entry)
                self.refused(added)

    def test_a_python_read_of_a_manifest_is_allowed(self):
        self.assert_commands(0, ["python3 -c \"import tomllib; tomllib.load(open('crates/x/tastemaker.toml', 'rb'))\"",
                                 "grep x crates/x/tastemaker.toml 2>/dev/null"])

    def test_a_script_reading_a_manifest_is_allowed(self):
        body = "import tomllib\nprint(tomllib.load(open('crates/x/tastemaker.toml', 'rb')))\n"
        self.assertEqual(self.write(os.path.join(self.tree, "check.py"), body).returncode, 0)
        shell_body = "#!/bin/sh\ngrep -c rule crates/x/tastemaker.toml 2>/dev/null\n"
        self.assertEqual(self.write(os.path.join(self.tree, "check.sh"), shell_body).returncode, 0)

    def test_tm_read_commands_are_allowed(self):
        self.assert_commands(0, ["tm check", "tm --root d check", "tm explain x", "tm list", "tm validate"])

    def test_the_message_file_route_is_allowed(self):
        message = "/tmp/claude-scratch/msg.txt"
        self.assertEqual(self.write(message, "drop the cp exclude from tastemaker.toml\n").returncode, 0)
        self.assert_commands(0, ["git commit -F " + message, "gh pr create --body-file " + message])

    # fails closed

    def test_the_write_hook_fails_closed_when_guard_shell_fails_to_import(self):
        with open(os.path.join(self.hooks, "guard_shell.py"), "a", encoding="utf-8") as handle:
            handle.write("\nraise RuntimeError('injected')\n")
        self.assertEqual(self.write(self.manifest, MANIFEST).returncode, 2)
        self.assertEqual(self.write(self.allowlist, "").returncode, 2)
        self.assertEqual(self.write(os.path.join(self.tree, "src", "new.rs"), "// ignore: idiomatic.arch/x - why\n").returncode, 2)
        self.assertEqual(self.edit(self.source, "fn a() {}", "fn a() {} // ignore: idiomatic.arch/x - why").returncode, 2)
        self.assertEqual(self.write(os.path.join(self.tree, "notes.md"), "hi\n").returncode, 0)

    def test_the_shell_hook_fails_closed_on_its_own_error_for_commands_naming_a_closed_file_or_mark(self):
        path = os.path.join(self.hooks, "guard_shell.py")
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
        old = "def segments(text):"
        self.assertIn(old, text)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(text.replace(old, "def segments(text):\n    raise RuntimeError('injected')\n\n\ndef _segments(text):", 1))
        for command in ("sed -i s/a/b/ crates/x/tastemaker.toml", "echo '# ignore: idiomatic.x' >> a.py",
                        "echo x > closed_for_extension.toml"):
            with self.subTest(command=command):
                done = self.bash(command)
                self.assertEqual(done.returncode, 2, done.stderr)
                self.assertIn("guard failed closed", done.stderr)
        self.assertEqual(self.bash("ls").returncode, 1)

    def test_an_unparsable_allowlist_admits_nothing(self):
        self.put(self.allowlist, "[[manifest]\n" + ADMIT_EXCLUDE)
        self.refused(('exclude = ["src/gen/**", "src/old/**"]', 'exclude = ["src/gen/**", "src/old/**", "src/new/**"]'))


if __name__ == "__main__":
    unittest.main()
