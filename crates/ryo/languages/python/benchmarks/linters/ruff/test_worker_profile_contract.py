"""Guard ordinary profiling against an accidentally traced worker command."""

import unittest
from pathlib import Path
import os
import sys
import tempfile

from profile_ryo import profile_manifest, run_profile_command
from worker_profile_contract import validate_worker_profile


class WorkerProfileContractTest(unittest.TestCase):
    def test_disabled_manifest_requires_ordinary_worker_even_with_flags_off(self):
        manifest = {"worker_tracing": "disabled", "measurement": False}
        validate_worker_profile(manifest, ["worker", "run-ryo", "--request-file", "request.json"])
        with self.assertRaisesRegex(ValueError, "worker tracing is jsonl"):
            validate_worker_profile(
                manifest,
                ["worker", "diagnostic-ryo", "--request-file", "request.json"],
            )

    def test_traced_profile_must_be_declared_intrusive(self):
        argv = ["worker", "diagnostic-ryo", "--request-file", "request.json"]
        validate_worker_profile({"worker_tracing": "jsonl", "measurement": False}, argv)
        with self.assertRaisesRegex(ValueError, "cannot provide a benchmark measurement"):
            validate_worker_profile({"worker_tracing": "jsonl", "measurement": True}, argv)

    def test_ordinary_worker_under_perf_is_still_intrusive(self):
        with tempfile.TemporaryDirectory() as temporary:
            worker = Path(temporary) / "worker"
            request = Path(temporary) / "request.json"
            worker.write_bytes(b"sealed worker")
            request.write_bytes(b"{}")
            command = [str(worker), "run-ryo", "--request-file", str(request)]
            manifest = profile_manifest(worker, request, "disabled", 49, None, command)
            validate_worker_profile(manifest, command)
            self.assertFalse(manifest["measurement"])
            self.assertFalse(manifest["timing_acceptance"])
            self.assertEqual(manifest["sampling_instrumentation"], "external-perf-intrusive")
            self.assertIn("request-referenced corpus is not hashed", manifest["input_binding"])

    def test_timeout_reaps_profile_process_group(self):
        with tempfile.TemporaryDirectory() as temporary:
            stdout_path = Path(temporary) / "stdout.bin"
            stderr_path = Path(temporary) / "stderr.bin"
            with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
                exit_code, status, error = run_profile_command(
                    [sys.executable, "-c", "import time; time.sleep(30)"],
                    dict(os.environ), stdout, stderr, 0.05,
                )
            self.assertEqual(status, "timeout")
            self.assertIsNotNone(exit_code)
            self.assertIn("exceeded", error)


if __name__ == "__main__":
    unittest.main()
