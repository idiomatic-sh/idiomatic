"""Run registered sparse-traversal guards with the caller's already sealed bench worker."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

GUARDS = (("pie794-rooted", 8), ("sparse-descendant-consumers", 10), ("plr1708-rooted", 8))
FIXTURES = Path("crates/ryo/languages/python/tests/fixtures/ryoql-fragments")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_guards(repository, worker, output, invoke=subprocess.run):
    """Never build, invoke Ruff, reuse a stale result, or accept an incomplete probe."""
    output.mkdir(exist_ok=False)
    receipts = []
    for name, expected_cases in GUARDS:
        script = repository / FIXTURES / name / "generic.py"
        destination = output / name
        command = [sys.executable, str(script), "--candidate-root", str(repository),
                   "--worker", str(worker), "--output", str(destination)]
        record = {"guard": name, "command": command, "script_sha256": digest(script),
                  "worker_sha256": digest(worker)}
        result = invoke(command, text=True, capture_output=True, timeout=120)
        record.update(status=result.returncode, stdout=result.stdout, stderr=result.stderr)
        receipts.append(record)
        (output / "executions.json").write_text(json.dumps(receipts, indent=2) + "\n")
        if result.returncode:
            raise ValueError(f"sparse traversal guard failed: {name}")
        proof_path = destination / "proof.json"
        proof = json.loads(proof_path.read_text())
        if (proof.get("cases") != expected_cases
                or proof.get("all_processes_zero") is not True
                or proof.get("unknown_or_unread") != 0
                or proof.get("counted_multiset_differences") != 0):
            raise ValueError(f"incomplete sparse traversal proof: {name}")
        record["proof_sha256"] = digest(proof_path)
    (output / "executions.json").write_text(json.dumps(receipts, indent=2) + "\n")
    return receipts


class RegistrationTests(unittest.TestCase):
    """Exercise the hook without compiling or starting an actual worker."""

    def exercise(self, *, status=0, proof_change=None, omit_proof=False):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            worker = root / "sealed-bench-worker"
            worker.write_bytes(b"worker identity")
            for name, _ in GUARDS:
                script = root / FIXTURES / name / "generic.py"
                script.parent.mkdir(parents=True)
                script.write_text("guard source")
            commands = []

            def fake(command, **options):
                commands.append(command)
                self.assertEqual(options["timeout"], 120)
                self.assertEqual(command[command.index("--worker") + 1], str(worker))
                self.assertNotIn("ruff", command)
                if not omit_proof:
                    destination = Path(command[-1])
                    destination.mkdir()
                    count = dict(GUARDS)[destination.name]
                    proof = dict(cases=count, all_processes_zero=True,
                                 unknown_or_unread=0, counted_multiset_differences=0)
                    proof.update(proof_change or {})
                    (destination / "proof.json").write_text(json.dumps(proof))
                return subprocess.CompletedProcess(command, status, "", "")

            receipts = run_guards(root, worker, root / "new-proof", fake)
            self.assertEqual(len(commands), len(GUARDS))
            self.assertEqual(len(receipts), len(GUARDS))
            with self.assertRaises(FileExistsError):
                run_guards(root, worker, root / "new-proof", fake)

    def test_calls_each_registered_guard_with_existing_worker(self):
        self.exercise()

    def test_nonzero_refused(self):
        with self.assertRaises(ValueError):
            self.exercise(status=1)

    def test_missing_proof_refused(self):
        with self.assertRaises(FileNotFoundError):
            self.exercise(omit_proof=True)

    def test_incomplete_or_wrong_proof_refused(self):
        for change in ({"cases": 0}, {"unknown_or_unread": 1},
                       {"all_processes_zero": False}, {"counted_multiset_differences": 1}):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.exercise(proof_change=change)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--candidate-root", type=Path)
    parser.add_argument("--worker", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    if arguments.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(RegistrationTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    if not all((arguments.candidate_root, arguments.worker, arguments.output)):
        parser.error("candidate-root, worker and fresh output required")
    run_guards(arguments.candidate_root.resolve(), arguments.worker.resolve(),
               arguments.output.resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
