"""Serialize case registrations and delegate process-receipt policy to the shared Rust verifier."""
from __future__ import annotations
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def verified_process_ids(inventory: list[dict[str, object]], records: list[dict[str, object]], receipt: Path | None) -> frozenset[str]:
    """Decode a pinned shared audit; this adapter never matches process bytes or exit policy."""
    if not any(case.get('reference_discrepancy', {}).get('process') is not None for case in inventory if isinstance(case.get('reference_discrepancy'), dict)):
        return frozenset()
    if receipt is None or not receipt.is_file():
        return frozenset()
    configured = os.environ.get('RYO_BENCHMARK_RECEIPT_VALIDATOR')
    pin = os.environ.get('RYO_BENCHMARK_RECEIPT_VALIDATOR_SHA256')
    if not configured or not pin:
        raise ValueError('process receipts require an explicit receipt file and pinned shared validator')
    original = receipt.read_bytes()
    loaded = [json.loads(line) for line in original.splitlines()]
    canonical = lambda rows: sorted(json.dumps(row, ensure_ascii=False, sort_keys=True, separators=(',', ':')) for row in rows)
    if canonical(loaded) != canonical(records):
        raise ValueError('in-memory observations differ from the explicit native receipt file')
    executable = Path(configured).resolve(strict=True)
    if hashlib.sha256(executable.read_bytes()).hexdigest() != pin:
        raise ValueError('shared receipt validator executable identity differs')
    rules = sorted({case['rule'] for case in inventory})
    registration = {'provider': 'case-receipts', 'defaults': rules, 'identity': {}, 'rules': [
        {'rule_id': rule, 'command': None, 'reason': None, 'instance': None,
         'cases': [case for case in inventory if case['rule'] == rule]} for rule in rules
    ]}
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
        if set(audit) != {'schema', 'case_count', 'validated_case_ids', 'raw_exact', 'documented_discrepancies', 'contract_only', 'errors'} or audit['schema'] != 'benchmark-case-receipt-audit/v1' or type(audit['case_count']) is not int or audit['case_count'] != len(inventory):
            raise ValueError('shared receipt audit scope differs')
        expected = {(case['rule'], case['name']) for case in inventory}
        if not isinstance(audit['validated_case_ids'], list) or any(set(row) != {'rule', 'name'} for row in audit['validated_case_ids']):
            raise ValueError('shared receipt identity encoding differs')
        pairs = [(row['rule'], row['name']) for row in audit['validated_case_ids']]
        if len(set(pairs)) != len(pairs) or not set(pairs) <= expected:
            raise ValueError('shared receipt audit contains unknown or duplicate identities')
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
