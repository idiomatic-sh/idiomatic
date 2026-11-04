"""Validation errors produce failing durable shard receipts; unfinished shards resume."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from authoring import case_receipt_adapter, case_records, profile_shards


class ProcessShardTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.profile = self.root / 'profile'
        self.profile.mkdir()
        (self.profile / 'benchmark.toml').write_text('parser-configuration="python3.10"\n')
        self.case = {'rule': 'A001', 'name': 'case', 'reference_discrepancy': {'process': {'kind': 'process-failure'}}}
        self.calls = 0

    def run_shards(self, resume=False, environment=None, payload=...):
        def runner(command, *, env, **kwargs):
            self.calls += 1
            Path(next(arg.split('=', 1)[1] for arg in command if arg.startswith('--case-receipts='))).write_text(json.dumps({'rule':'A001','name':'case','side':'reference'} if payload is ... else payload) + '\n')
            return subprocess.CompletedProcess(command, 0, b'1 cases passed against both contestants\n', b'')
        with patch.dict(os.environ, environment or {}, clear=True), \
             patch('authoring.profile_shards.case_records.profile_case_inventory', return_value=[self.case]), \
             patch('authoring.profile_shards.profile_config.sha256', return_value='pin'), \
             patch('authoring.profile_shards.profile_worker.staged_hashes', return_value={}), \
             patch('authoring.profile_shards.profile_worker.command_for', return_value=['fixture']):
            return profile_shards.verify_profile_sharded('toy', self.profile, self.root/'worker', self.root/'reference', self.root/'output', {}, 1, resume=resume, worker_limit=1, runner=runner)

    def receipt(self):
        return self.root/'output'/'profiles'/'toy'/'shards'/'000.receipt.json'

    def test_missing_validator_records_failed_shard_instead_of_escaping_future(self):
        self.assertEqual(self.run_shards(), 1)
        receipt = json.loads(self.receipt().read_text())
        self.assertEqual(receipt['status'], 1)
        self.assertFalse(receipt['case_result_records_valid'])
        self.assertIn('pinned shared validator', receipt['error'])
        self.assertEqual(receipt['last_process_status'], 0)
        self.assertTrue(self.receipt().with_name('000.stdout.bin').is_file())

    def test_missing_receipt_is_unfinished_and_resume_still_runs_the_shard(self):
        self.assertEqual(self.run_shards(), 1)
        self.receipt().unlink()
        self.receipt().with_name('000.cases.jsonl').unlink()
        self.assertEqual(self.run_shards(resume=True), 1)
        self.assertEqual(self.calls, 2)
        self.assertIn('pinned shared validator', json.loads(self.receipt().read_text())['error'])

    def test_wrong_validator_in_prior_history_is_recorded_not_silently_reused(self):
        validator = self.root/'validator'
        validator.write_bytes(b'actual validator bytes')
        environment = {'RYO_BENCHMARK_RECEIPT_VALIDATOR': str(validator), 'RYO_BENCHMARK_RECEIPT_VALIDATOR_SHA256': 'a'*64}
        self.assertEqual(self.run_shards(environment=environment), 1)
        self.assertEqual(self.run_shards(resume=True, environment=environment), 1)
        receipt = json.loads(self.receipt().read_text())
        self.assertIn('executable identity differs', receipt['error'])
        self.assertIn('prior case receipt validation failed', receipt['error'])
        self.assertIn('case receipt validation failed', receipt['error'])
        self.assertEqual(self.calls, 2)
        self.assertTrue(list(self.receipt().parent.glob('000.cases.jsonl.history-*')))

    def test_no_receipt_never_authorizes_any_process_case(self):
        with patch.dict(os.environ, {}, clear=True):
            self.assertEqual(case_receipt_adapter.verified_case_ids([self.case], [], None), frozenset())
            self.assertEqual(case_receipt_adapter.verified_case_ids([self.case], [], self.root/'absent.jsonl'), frozenset())

    def test_nonobject_and_wrong_identity_types_have_explicit_failed_shard_receipts(self):
        for payload in [None, [], {'rule':'A001','name':'case','side':[]}]:
            with self.subTest(payload=payload):
                self.assertEqual(self.run_shards(resume=self.receipt().exists(), payload=payload), 1)
                receipt = json.loads(self.receipt().read_text())
                self.assertEqual(receipt['status'], 1)
                self.assertFalse(receipt['case_result_records_valid'])
                self.assertIn('object with string rule/name/side', receipt['error'])

    def test_malformed_resume_history_is_preserved_even_when_no_rows_can_be_kept(self):
        self.assertEqual(self.run_shards(), 1)
        records = self.receipt().with_name('000.cases.jsonl')
        for text in ['null\n', '[]\n', '{"rule":"A001","name":"case","side":[]}\n']:
            with self.subTest(text=text):
                records.write_text(text)
                self.assertEqual(self.run_shards(resume=True), 1)
                history = list(records.parent.glob('000.cases.jsonl.history-*'))
                self.assertIn(text.encode(), [path.read_bytes() for path in history])
                self.assertIn('prior native case receipt could not be read', json.loads(self.receipt().read_text())['error'])

    def test_generic_record_boundary_refuses_invalid_rows_without_process_cases(self):
        for row in [None, [], {'rule':[], 'name':'case', 'side':'reference'}, {'rule':'A001', 'name':{}, 'side':'reference'}, {'rule':'A001', 'name':'case', 'side':[]}]:
            with self.subTest(row=row), self.assertRaises(ValueError):
                case_records.complete_case_ids([], [row])

    def test_missing_required_metadata_never_resumes_byte_only_rows_as_complete(self):
        self.case = {'rule': 'A001', 'name': 'case', 'reference_discrepancy': None, 'file_observation': 'posix-executable'}
        self.assertEqual(self.run_shards(), 1)
        self.assertEqual(self.run_shards(resume=True), 1)
        self.assertEqual(self.calls, 2)
        receipt = json.loads(self.receipt().read_text())
        self.assertEqual(receipt['status'], 1)
        self.assertFalse(receipt['case_result_records_valid'])
        self.assertIn('pinned shared validator', receipt['error'])
        self.assertTrue(list(self.receipt().parent.glob('000.cases.jsonl.history-*')))
