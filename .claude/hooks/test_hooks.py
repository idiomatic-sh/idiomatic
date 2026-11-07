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
            LOCK + "cargo test --target-dir /workspaces/idiomatic-benchmark-guards/target",
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

    def push(self, command="git push origin claude/ryo-consolidation"):
        return run_hook("guard_shell.py", {"tool_input": {"command": command}, "cwd": self.tree})

    def test_refused_without_the_review(self):
        result = self.push()
        self.assertEqual(result.returncode, 2)
        self.assertIn("HEAD carries no " + self.REVIEW, result.stderr)

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
        self.assertIn("not an ancestor of HEAD", result.stderr)

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
        for command in ["git push origin claude/ryo-consolidation", "git push origin HEAD:claude/ryo-consolidation",
                        "git push -q origin +HEAD:refs/heads/claude/ryo-consolidation",
                        "cd . && git -C . push origin claude/ryo-consolidation"]:
            with self.subTest(command=command):
                self.assertEqual(self.push(command).returncode, 0)

    def test_every_refspec_form_is_guarded(self):
        for command in ["git push origin HEAD:claude/ryo-consolidation", "git push -u origin claude/ryo-consolidation",
                        "git push origin +x:refs/heads/claude/ryo-consolidation", "x && git push origin claude/ryo-consolidation",
                        "git -C " + self.tree + " push origin claude/ryo-consolidation"]:
            with self.subTest(command=command):
                self.assertEqual(run_hook("guard_shell.py", {"tool_input": {"command": command}}).returncode
                                 if command.startswith("git -C") else self.push(command).returncode, 2)

    def test_other_branches_are_unaffected(self):
        for command in ["git push origin claude/checkpoint-9", "git push -u origin claude/w4-rust-reviewer",
                        "git push", "git push origin claude/ryo-consolidation-notes", "git log claude/ryo-consolidation",
                        "git commit -m 'git push origin claude/ryo-consolidation'"]:
            with self.subTest(command=command):
                self.assertEqual(self.push(command).returncode, 0)


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
