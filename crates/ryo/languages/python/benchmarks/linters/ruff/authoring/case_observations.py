"""The recorded executions of cases and the observations compared across contestants."""
from __future__ import annotations
import collections
import json
from pathlib import Path
from authoring import case_receipt_adapter
from authoring.case_records import context_receipts_valid, diagnostic_multiset, profile_case_inventory

def discrepancy_record_pair_valid(case: dict[str, object], sides: dict[str, dict[str, object]], *, _shared_verified: frozenset[str]=frozenset()) -> bool:
    """Native attestations bind the actually verified request, not inferred historical settings."""
    certificate = case.get('reference_discrepancy')
    if certificate is None:
        return True
    if certificate.get('process') is not None:
        return f"{case['rule']}/{case['name']}" in _shared_verified
    identity = certificate['identity']
    basis = certificate.get('basis', 'language-defect')
    if identity.get('file-metadata-sha256') != case.get('file_metadata_sha256'):
        return False
    for side, row in sides.items():
        proof = row.get('discrepancy_validation')
        if not isinstance(proof, dict) or proof != sides['reference'].get('discrepancy_validation'):
            return False
        if proof.get('discrepancy_basis', 'language-defect') != basis or row.get('documented_reference_discrepancy') is not (basis == 'language-defect') or row.get('documented_tool_contract_difference', False) is not (basis == 'tool-contract-difference'):
            return False
        checks = {'settings_sha256': 'settings-sha256', 'profile_sha256': 'profile-sha256', 'source_sha256': 'source-sha256', 'reference_output_sha256': 'reference-output-sha256', 'intended_output_sha256': 'intended-output-sha256'}
        if proof.get('file_metadata_sha256') != case.get('file_metadata_sha256') or any((proof.get(key) != identity[value] for key, value in checks.items())):
            return False
        if proof.get('actual_executable_sha256') not in certificate['approved_executable_sha256']:
            return False
        expected = certificate['reference_expected'] if side == 'reference' else certificate['intended_expected']
        if row.get('skipped') != [] or not isinstance(row.get('diagnostics'), list):
            return False
        actual_matches = diagnostic_multiset(row['diagnostics']) == diagnostic_multiset(expected)
        if row.get('case_passed') is not actual_matches:
            return False
        coverage = row.get('coverage')
        if coverage is None and side == 'candidate':
            return False
        if coverage is not None and (coverage.get('unknown_or_unread_rule_file_pairs') != 0 or coverage.get('skipped_rule_file_pairs') != 0 or coverage.get('selected_rule_file_pairs', 0) <= 0 or (coverage.get('selected_rule_file_pairs') != coverage.get('evaluated_rule_file_pairs'))):
            return False
    return True

def record_execution_complete(row: dict[str, object]) -> bool:
    """Validate an explicitly declared producer capability; legacy absence never becomes complete."""
    if row.get('execution_metadata') != {'availability': 'complete'}:
        return False
    coverage = row.get('coverage')
    fields = {'selected_rule_file_pairs', 'evaluated_rule_file_pairs', 'skipped_rule_file_pairs', 'unknown_or_unread_rule_file_pairs'}
    if not isinstance(coverage, dict) or set(coverage) != fields:
        return False
    if any((type(value) is not int or value < 0 for value in coverage.values())):
        return False
    skips = row.get('skipped')
    files = row.get('files')
    if not isinstance(skips, list) or not isinstance(files, list) or (not files) or any((not isinstance(file, str) or not file for file in files)) or (len(set(files)) != len(files)):
        return False
    pairs = set()
    for skip in skips:
        if not isinstance(skip, dict) or set(skip) != {'rule', 'path', 'reason'} or (not isinstance(skip['rule'], str)) or (not skip['rule'].strip()) or (skip['path'] not in files) or (skip['reason'] != 'syntax-rejected'):
            return False
        pair = (skip['rule'], skip['path'])
        if pair in pairs:
            return False
        pairs.add(pair)
    return coverage['selected_rule_file_pairs'] >= len(files) and coverage['unknown_or_unread_rule_file_pairs'] == 0 and (coverage['skipped_rule_file_pairs'] == len(skips)) and (coverage['selected_rule_file_pairs'] == coverage['evaluated_rule_file_pairs'] + coverage['skipped_rule_file_pairs'])

def record_execution_comparison(reference: dict[str, object], candidate: dict[str, object]) -> str:
    """Keep unavailable/legacy evidence incomparable, without accepting invalid new candidate evidence."""
    if candidate.get('observation_schema') == 'benchmark-case/v2' and (not record_execution_complete(candidate)):
        return 'different'
    if not record_execution_complete(candidate):
        return 'incomparable'
    capability = reference.get('execution_metadata')
    if not isinstance(capability, dict) or capability.get('availability') != 'complete':
        return 'incomparable'
    if not record_execution_complete(reference):
        return 'different'
    skips = lambda row: sorted(((skip['rule'], skip['path'], skip['reason']) for skip in row['skipped']))
    return 'exact' if reference['coverage'] == candidate['coverage'] and skips(reference) == skips(candidate) else 'different'

def record_contract_satisfied(case: dict[str, object], sides: dict[str, dict[str, object]], *, _shared_verified: frozenset[str]=frozenset()) -> bool:
    """Apply the declared contract without promoting unavailable reference accounting to zero."""
    if isinstance(case.get('reference_discrepancy'), dict) and case['reference_discrepancy'].get('process') is not None:
        return f"{case['rule']}/{case['name']}" in _shared_verified
    contract = case.get('expectation_contract', 'diagnostics')
    if contract not in {'diagnostics', 'complete-report'} or set(sides) != {'reference', 'candidate'}:
        return False
    for row in sides.values():
        if row.get('observation_schema') != 'benchmark-case/v2' or row.get('expectation_contract') != contract:
            return False
        capability = row.get('execution_metadata')
        if not isinstance(capability, dict):
            return False
        if capability.get('availability') == 'unavailable':
            if set(capability) != {'availability', 'reason'} or not isinstance(capability['reason'], str) or not capability['reason'].strip() or row.get('coverage') is not None or row.get('skipped') != []:
                return False
        elif capability != {'availability': 'complete'} or not record_execution_complete(row):
            return False
    if not record_execution_complete(sides['candidate']):
        return False
    execution = record_execution_comparison(sides['reference'], sides['candidate'])
    return execution == 'exact' or (contract == 'diagnostics' and execution == 'incomparable')

def case_record_objects(records: list[object]) -> list[dict[str, object]]:
    """Validate JSON row structure before grouping; diagnostic policy remains with its verifier."""
    for index, row in enumerate(records, 1):
        if not isinstance(row, dict) or any(not isinstance(row.get(key), str) for key in ('rule', 'name', 'side')):
            raise ValueError(f'native case receipt row {index} must be an object with string rule/name/side identities')
    return records

def complete_case_ids(selected_cases: list[dict[str, object]], records: list[dict[str, object]], *, receipt_file: Path | None=None) -> set[str]:
    """Return cases with one valid, durable full report from each contestant."""
    records = case_record_objects(records)
    shared_verified = case_receipt_adapter.verified_case_ids(selected_cases, records, receipt_file)
    expected_cases = {f"{case['rule']}/{case['name']}": case for case in selected_cases}
    grouped: dict[str, dict[str, dict[str, object]]] = {}
    duplicates = set()
    for row in records:
        case_id = f"{row.get('rule')}/{row.get('name')}"
        side = row.get('side')
        if case_id not in expected_cases or side not in {'reference', 'candidate'}:
            continue
        bucket = grouped.setdefault(case_id, {})
        if side in bucket:
            duplicates.add(case_id)
            continue
        bucket[side] = row
    completed = set()
    for case_id, sides in grouped.items():
        expected_case = expected_cases[case_id]
        if case_id in duplicates or set(sides) != {'reference', 'candidate'}:
            continue
        identities_match = all(
            row.get('complete') is True and row.get('error') is None
            and row.get('source_sha256') == expected_case['source_sha256']
            and row.get('project_inputs_sha256') == expected_case.get('project_inputs_sha256')
            and row.get('file_metadata_sha256') == expected_case.get('file_metadata_sha256')
            and row.get('selected_rules') == expected_case.get('selected_rules')
            and row.get('rule_observations') == expected_case.get('rule_observations')
            and row.get('rule_context') == expected_case.get('rule_context')
            and row.get('files') == expected_case['files']
            and row.get('expected') == expected_case['expected']
            and isinstance(row.get('case_passed'), bool)
            for row in sides.values()
        )
        physical_verified = expected_case.get('file_metadata') is None and expected_case.get('file_observation') is None and expected_case.get('file_metadata_sha256') is None or case_id in shared_verified
        if (identities_match and physical_verified and context_receipts_valid(expected_case, sides)
            and discrepancy_record_pair_valid(expected_case, sides, _shared_verified=shared_verified)
            and record_contract_satisfied(expected_case, sides, _shared_verified=shared_verified)):
            completed.add(case_id)
    return completed

def case_expectation_summary(inventory: list[dict[str, object]], records: list[dict[str, object]], *, receipt_file: Path | None=None) -> dict[str, int]:
    """Raw parity and independently intended behavior are separate, never interchangeable totals."""
    completed = complete_case_ids(inventory, records, receipt_file=receipt_file)
    grouped: dict[str, dict[str, dict[str, object]]] = {}
    for row in records:
        case_id = f"{row.get('rule')}/{row.get('name')}"
        if case_id in completed:
            grouped.setdefault(case_id, {})[row['side']] = row
    counts = {'complete_case_pairs': len(completed), 'incomplete_cases': len(inventory) - len(completed), 'raw_parity_matches': 0, 'raw_parity_mismatches': 0, 'documented_reference_discrepancies': 0, 'documented_tool_contract_differences': 0, 'unexplained_mismatches': 0, 'intended_behavior_passes': 0, 'intended_behavior_failures': 0, 'diagnostic_parity_matches': 0, 'diagnostic_parity_mismatches': 0, 'execution_metadata_exact': 0, 'execution_metadata_different': 0, 'execution_metadata_incomparable': 0, 'typed_candidate_execution_complete': 0, 'legacy_or_unknown_candidate_execution': 0}
    for case in inventory:
        sides = grouped.get(f"{case['rule']}/{case['name']}")
        if sides is None:
            continue
        reference, candidate = (sides['reference'], sides['candidate'])
        process = isinstance(case.get('reference_discrepancy'), dict) and case['reference_discrepancy'].get('process') is not None
        if process:
            counts['diagnostic_parity_incomparable'] = counts.get('diagnostic_parity_incomparable', 0) + 1
        diagnostic_match = not process and diagnostic_multiset(reference['diagnostics']) == diagnostic_multiset(candidate['diagnostics'])
        if not process:
            counts['diagnostic_parity_matches' if diagnostic_match else 'diagnostic_parity_mismatches'] += 1
        counts[f'execution_metadata_{record_execution_comparison(reference, candidate)}'] += 1
        counts['typed_candidate_execution_complete' if record_execution_complete(candidate) else 'legacy_or_unknown_candidate_execution'] += 1
        raw_match = not process and diagnostic_multiset(reference['diagnostics']) == diagnostic_multiset(candidate['diagnostics']) and reference.get('skipped', []) == candidate.get('skipped', [])
        intended_pass = candidate.get('case_passed') is True
        counts['raw_parity_matches' if raw_match else 'raw_parity_mismatches'] += 1
        counts['intended_behavior_passes' if intended_pass else 'intended_behavior_failures'] += 1
        documented = not raw_match and case.get('reference_discrepancy') is not None and (reference.get('case_passed') is True) and intended_pass
        if documented:
            label = 'documented_tool_contract_differences' if case['reference_discrepancy'].get('basis') == 'tool-contract-difference' else 'documented_reference_discrepancies'
            counts[label] += 1
            if process:
                counts['reference_process_discrepancies'] = counts.get('reference_process_discrepancies', 0) + 1
        elif not raw_match:
            if diagnostic_match and record_contract_satisfied(case, sides):
                counts['contract_only_passes'] = counts.get('contract_only_passes', 0) + 1
            else:
                counts['unexplained_mismatches'] += 1
    return counts

def case_observation_map(paths: list[Path]) -> dict[tuple[str, str, str], dict[str, object]]:
    """Load complete per-case contestant records and reject duplicate observations."""
    observations = {}
    for path in paths:
        for line in path.read_text(encoding='utf-8').splitlines():
            row = json.loads(line)
            key = (row['rule'], row['name'], row['side'])
            if key in observations:
                raise ValueError(f'duplicate case observation: {key}')
            if row.get('complete') is not True:
                raise ValueError(f'incomplete case observation: {key}')
            observations[key] = row
    return observations

def case_observations_match(monolithic: Path, shards: list[Path]) -> bool:
    """Require exact full-report observations for each side and case across modes."""
    try:
        return case_observation_map([monolithic]) == case_observation_map(shards)
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        return False

def candidate_result_valid(profile_root: Path, stdout: bytes, case_results_path: Path) -> tuple[bool, str | None]:
    """Require the real worker summary and exactly two complete contestant records per case."""
    try:
        inventory = profile_case_inventory(profile_root)
        records = [json.loads(line) for line in case_results_path.read_text(encoding='utf-8').splitlines()]
        valid_ids = complete_case_ids(inventory, records, receipt_file=case_results_path)
        expected_ids = {f"{case['rule']}/{case['name']}" for case in inventory}
        side_counts = collections.Counter(((row.get('rule'), row.get('name'), row.get('side')) for row in records))
        output = stdout.decode('utf-8')
    except (OSError, UnicodeDecodeError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        return (False, f'candidate proof records could not be read: {error}')
    expected_summary = f'{len(inventory)} cases passed against both contestants\n'
    if output != expected_summary:
        return (False, f'worker summary mismatch: expected {expected_summary!r}, got {output!r}')
    if valid_ids != expected_ids or len(records) != 2 * len(inventory):
        return (False, f'worker case receipts incomplete: expected {len(expected_ids)} exact case IDs, observed {len(valid_ids)}')
    if any((side_counts[case['rule'], case['name'], side] != 1 for case in inventory for side in ('reference', 'candidate'))):
        return (False, 'worker case receipts contain missing or duplicate contestant records')
    if any((row.get('case_passed') is not True for row in records)):
        return (False, 'worker case receipts include a failed contestant comparison')
    return (True, None)
