"""Serialize case registrations and delegate physical-observation receipt policy to the shared Rust verifier."""
from __future__ import annotations
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def file_metadata_digest(values: dict[str, bool] | None) -> str | None:
    """Encode the shared versioned path-length/UTF-8/boolean framing; no comparison policy."""
    if values is None:
        return None
    if not isinstance(values, dict) or any(not isinstance(key, str) or type(value) is not bool for key, value in values.items()):
        raise ValueError('file metadata must map declared paths to booleans')
    digest = hashlib.sha256(b'ryo-case-file-metadata-v1\x00')
    for path, executable in sorted(values.items()):
        encoded = path.encode('utf-8')
        digest.update(len(encoded).to_bytes(8, 'little'))
        digest.update(encoded)
        digest.update(bytes([int(executable)]))
    return digest.hexdigest()


def receipt_registration(inventory: list[dict[str, object]]) -> dict[str, object]:
    """Declare receipt-only authority; this registration cannot execute a worker."""
    rules = sorted({case['rule'] for case in inventory})
    return {'authority': {'kind': 'receipt-only'}, 'provider': 'case-receipts', 'defaults': rules, 'identity': {}, 'rules': [
        {'rule_id': rule, 'definition': None, 'modules': {}, 'artifacts': {}, 'command': None, 'reason': None, 'instance': None,
         'cases': [case for case in inventory if case['rule'] == rule]} for rule in rules
    ]}


def verified_case_ids(inventory: list[dict[str, object]], records: list[dict[str, object]], receipt: Path | None) -> frozenset[str]:
    """Decode a pinned shared audit; this adapter never matches process bytes or exit policy."""
    if not any(case.get('file_metadata') is not None or case.get('file_observation') is not None or case.get('file_metadata_sha256') is not None or (isinstance(case.get('reference_discrepancy'), dict) and case['reference_discrepancy'].get('process') is not None) for case in inventory):
        return frozenset()
    if receipt is None or not receipt.is_file():
        return frozenset()
    configured = os.environ.get('RYO_BENCHMARK_RECEIPT_VALIDATOR')
    pin = os.environ.get('RYO_BENCHMARK_RECEIPT_VALIDATOR_SHA256')
    if not configured or not pin:
        raise ValueError('physical-observation receipts require an explicit receipt file and pinned shared validator')
    original = receipt.read_bytes()
    loaded = [json.loads(line) for line in original.splitlines()]
    canonical = lambda rows: sorted(json.dumps(row, ensure_ascii=False, sort_keys=True, separators=(',', ':')) for row in rows)
    if canonical(loaded) != canonical(records):
        raise ValueError('in-memory observations differ from the explicit native receipt file')
    executable = Path(configured).resolve(strict=True)
    if hashlib.sha256(executable.read_bytes()).hexdigest() != pin:
        raise ValueError('shared receipt validator executable identity differs')
    registration = receipt_registration(inventory)
    with tempfile.TemporaryDirectory(prefix='case-receipt-registration-') as temporary:
        path = Path(temporary) / 'registration.json'
        path.write_text(json.dumps(registration, ensure_ascii=False, sort_keys=True) + '\n')
        try:
            result = subprocess.run([str(executable), 'validate-case-receipts', str(path), str(receipt.resolve())],
                                    input=b'', capture_output=True, timeout=30, check=False)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise ValueError('shared receipt validator could not complete') from error
    if receipt.read_bytes() != original:
        raise ValueError('native receipt changed during shared validation')
    if result.returncode not in (0, 1):
        raise ValueError('shared receipt validator operational failure')
    try:
        audit = json.loads(result.stdout)
        if set(audit) - {'file_metadata_verified_cases', 'tool_contract_difference_case_ids'} != {'schema', 'case_count', 'validated_case_ids', 'raw_exact', 'documented_discrepancies', 'contract_only', 'errors'} or audit['schema'] != 'benchmark-case-receipt-audit/v1' or type(audit['case_count']) is not int or audit['case_count'] != len(inventory):
            raise ValueError('shared receipt audit scope differs')
        expected = {(case['rule'], case['name']) for case in inventory}
        if not isinstance(audit['validated_case_ids'], list) or any(set(row) != {'rule', 'name'} for row in audit['validated_case_ids']):
            raise ValueError('shared receipt identity encoding differs')
        pairs = [(row['rule'], row['name']) for row in audit['validated_case_ids']]
        if len(set(pairs)) != len(pairs) or not set(pairs) <= expected:
            raise ValueError('shared receipt audit contains unknown or duplicate identities')
        policy_cases = {(case['rule'], case['name']) for case in inventory if isinstance(case.get('reference_discrepancy'), dict) and case['reference_discrepancy'].get('basis') == 'tool-contract-difference'}
        policy_rows = audit.get('tool_contract_difference_case_ids', [])
        if not isinstance(policy_rows, list) or any(not isinstance(row, dict) or set(row) != {'rule', 'name'} for row in policy_rows) or {(row['rule'], row['name']) for row in policy_rows} != set(pairs).intersection(policy_cases) or len(policy_rows) != len(set((row['rule'], row['name']) for row in policy_rows)):
            raise ValueError('shared validator did not identify exact tool-contract differences')
        metadata_cases = {(case['rule'], case['name']): case for case in inventory if case.get('file_metadata') is not None or case.get('file_observation') is not None or case.get('file_metadata_sha256') is not None}
        witnesses = audit.get('file_metadata_verified_cases', [])
        if not isinstance(witnesses, list) or any(not isinstance(row, dict) or set(row) != {'rule', 'name', 'sha256'} or not all(isinstance(row[key], str) for key in row) or len(row['sha256']) != 64 or any(character not in '0123456789abcdef' for character in row['sha256']) for row in witnesses):
            raise ValueError('shared metadata witness encoding differs')
        witness_pairs = [(row['rule'], row['name']) for row in witnesses]
        if len(set(witness_pairs)) != len(witness_pairs) or set(witness_pairs) != set(pairs).intersection(metadata_cases):
            raise ValueError('shared validator did not verify required file metadata')
        if any(row['sha256'] != metadata_cases[(row['rule'], row['name'])].get('file_metadata_sha256') for row in witnesses):
            raise ValueError('shared metadata witness identity differs')
        counts = [audit[key] for key in ('raw_exact', 'documented_discrepancies', 'contract_only')]
        if any(type(value) is not int or value < 0 for value in counts) or sum(counts) != len(pairs):
            raise ValueError('shared receipt audit count encoding differs')
        if not isinstance(audit['errors'], list) or any(not isinstance(error, str) for error in audit['errors']):
            raise ValueError('shared receipt audit errors are malformed')
        complete = len(pairs) == len(inventory) and not audit['errors']
        if (result.returncode == 0) != complete:
            raise ValueError('shared receipt audit exit contradicts its result')
    except (KeyError, TypeError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError('shared receipt audit could not be decoded') from error
    return frozenset(f'{rule}/{name}' for rule, name in pairs)
