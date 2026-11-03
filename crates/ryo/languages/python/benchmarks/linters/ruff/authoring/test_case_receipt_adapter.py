"""Delegation cannot invent shared process acceptance from receipt flags."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from authoring.case_receipt_adapter import verified_process_ids


class ReceiptAdapterTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        self.executable = root / 'validator'
        self.executable.write_bytes(b'pinned shared validator fixture')
        self.receipt = root / 'one.jsonl'
        self.records = [{'rule': 'A001', 'name': 'one', 'side': 'reference'}]
        self.receipt.write_text(json.dumps(self.records[0]) + '\n')
        self.inventory = [{'rule': 'A001', 'name': 'one', 'reference_discrepancy': {'process': {'kind': 'process-failure'}}}]
        self.environment = {'RYO_BENCHMARK_RECEIPT_VALIDATOR': str(self.executable),
                            'RYO_BENCHMARK_RECEIPT_VALIDATOR_SHA256': hashlib.sha256(self.executable.read_bytes()).hexdigest()}
        self.audit = {'schema': 'benchmark-case-receipt-audit/v1', 'case_count': 1,
                      'validated_case_ids': [{'rule': 'A001', 'name': 'one'}],
                      'raw_exact': 0, 'documented_discrepancies': 1, 'contract_only': 0, 'errors': []}

    def run_audit(self, audit=None, code=0):
        result = subprocess.CompletedProcess([], code, json.dumps(self.audit if audit is None else audit).encode(), b'')
        with patch.dict(os.environ, self.environment, clear=True), patch('authoring.case_receipt_adapter.subprocess.run', return_value=result) as run:
            answer = verified_process_ids(self.inventory, self.records, self.receipt)
            self.assertEqual(run.call_args.args[0][1], 'validate-case-receipts')
            self.assertEqual(run.call_args.args[0][-1], str(self.receipt.resolve()))
            return answer

    def test_pinned_shared_audit_is_mandatory(self):
        self.assertEqual(self.run_audit(), frozenset({'A001/one'}))
        with patch.dict(os.environ, {}, clear=True), self.assertRaises(ValueError):
            verified_process_ids(self.inventory, self.records, self.receipt)
        self.executable.write_bytes(b'changed')
        with self.assertRaises(ValueError): self.run_audit()

    def test_forged_and_inconsistent_audits_refuse(self):
        for update, code in [({'validated_case_ids': [{'rule': 'other', 'name': 'one'}]}, 0),
                             ({'documented_discrepancies': 2}, 0), ({'errors': ['failure']}, 0), ({}, 1)]:
            with self.subTest(update=update, code=code), self.assertRaises(ValueError):
                self.run_audit({**self.audit, **update}, code)
        incomplete = {**self.audit, 'validated_case_ids': [], 'documented_discrepancies': 0, 'errors': ['missing candidate']}
        self.assertEqual(self.run_audit(incomplete, 1), frozenset())

    def test_in_memory_mutation_refuses_before_delegation(self):
        self.records[0]['side'] = 'candidate'
        with patch.dict(os.environ, self.environment, clear=True), patch('authoring.case_receipt_adapter.subprocess.run') as run, self.assertRaises(ValueError):
            verified_process_ids(self.inventory, self.records, self.receipt)
        run.assert_not_called()

    def test_diagnostic_only_receipts_need_no_process_validator(self):
        with patch.dict(os.environ, {}, clear=True):
            self.assertEqual(verified_process_ids([{'reference_discrepancy': None}], [], None), frozenset())
