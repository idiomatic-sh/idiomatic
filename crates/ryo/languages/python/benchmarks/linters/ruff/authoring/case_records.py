"""Case records: linter-specific authoring and proof utilities."""
from __future__ import annotations
import collections
import hashlib
import json
import os
import re
from pathlib import Path
import tomllib
from authoring import reference_discrepancies

def source_hash(files: dict[str, str]) -> str:
    encoded = json.dumps(files, sort_keys=True, separators=(',', ':'), ensure_ascii=False)
    return hashlib.sha256(encoded.encode('utf-8')).hexdigest()

def diagnostic_multiset(findings: list[dict[str, object]]) -> collections.Counter[tuple[object, ...]]:
    """Compare exact reference findings while retaining duplicate diagnostics."""
    return collections.Counter(((finding['rule'], finding['file'], finding['start'], finding['end']) for finding in findings))

def expected_context_identity(case: dict[str, object], mode: str) -> dict[str, object] | None:
    """Bind declared context bytes using the benchmark's framed SHA256 manifest contract."""
    if mode not in {'isolated', 'listed-disk'}:
        raise ValueError(f'unsupported project context mode: {mode}')
    context = case.get('context')
    if mode == 'isolated':
        if context is not None:
            raise ValueError('isolated profile cannot admit a context-authored case')
        return None
    if not isinstance(context, dict) or set(context) - {'logical-root', 'auxiliary-files', 'missing'}:
        raise ValueError('listed-disk profile requires an explicit context for every case')

    def relative(value: object, *, root: bool=False) -> Path:
        if not isinstance(value, str) or not value or Path(value).is_absolute():
            raise ValueError('unsafe context path')
        path = Path(value)
        if any((part == '..' for part in path.parts)) or (not root and (not path.parts)):
            raise ValueError('unsafe context path')
        return path
    logical = context.get('logical-root')
    root_path = relative(logical, root=True)
    sources = {str(root_path / relative(name)): hashlib.sha256(text.encode('utf-8')).hexdigest() for name, text in case['files'].items()}
    auxiliary = {str(relative(name)): hashlib.sha256(text.encode('utf-8')).hexdigest() for name, text in context.get('auxiliary-files', {}).items()}
    missing = sorted((str(relative(name)) for name in context.get('missing', [])))
    if set(sources) & set(auxiliary) or set(missing) & (set(sources) | set(auxiliary)) or len(missing) != len(set(missing)):
        raise ValueError('context source, auxiliary and missing roles overlap')
    digest = hashlib.sha256()

    def frame(value: bytes) -> None:
        digest.update(len(value).to_bytes(8, 'little'))
        digest.update(value)
    frame(b'listed-disk-context-v1')
    frame(os.fsencode(logical))
    for role, files in ((b'source', sources), (b'auxiliary', auxiliary)):
        frame(role)
        for name, value in sorted(files.items()):
            frame(os.fsencode(name))
            frame(value.encode('ascii'))
    for name in missing:
        frame(b'missing')
        frame(os.fsencode(name))
    return {'mode': mode, 'logical_root': logical, 'manifest_hash': digest.hexdigest(), 'present_paths': sorted(set(sources) | set(auxiliary)), 'missing_paths': missing}

def context_receipts_valid(case: dict[str, object], sides: dict[str, dict[str, object]]) -> bool:
    """Check opaque native receipts' identities and roles; Rust validates content fingerprints."""
    mode = case.get('project_context_mode', 'isolated')
    if mode == 'isolated':
        return all((row.get('context_manifest') is None and row.get('project_context') is None for row in sides.values()))
    expected = case.get('context_identity')
    if mode != 'listed-disk' or not isinstance(expected, dict):
        return False
    reference = sides['reference'].get('context_manifest')
    candidate = sides['candidate'].get('context_manifest')
    if not isinstance(reference, dict) or reference != candidate or set(reference) != {'mode', 'logical_root', 'actual_root', 'manifest_hash'}:
        return False
    if any((reference.get(key) != expected[key] for key in ('mode', 'logical_root', 'manifest_hash'))):
        return False
    actual = reference.get('actual_root')
    if not isinstance(actual, str) or not Path(actual).is_absolute():
        return False
    root_parts = Path(expected['logical_root']).parts
    if root_parts and Path(actual).parts[-len(root_parts):] != root_parts:
        return False
    receipt = sides['candidate'].get('project_context')
    if sides['reference'].get('project_context') is not None or not isinstance(receipt, dict) or set(receipt) != set(reference) | {'dependencies'}:
        return False
    if any((receipt.get(key) != value for key, value in reference.items())):
        return False
    dependencies = receipt.get('dependencies')
    if not isinstance(dependencies, dict):
        return False
    for name, fingerprint in dependencies.items():
        if not isinstance(name, str) or Path(name).is_absolute():
            return False
        resolved = list(root_parts)
        for part in Path(name).parts:
            if part == '..':
                if not resolved:
                    return False
                resolved.pop()
            elif part != '.':
                resolved.append(part)
        declared = str(Path(*resolved))
        if fingerprint is None:
            if declared not in expected['missing_paths']:
                return False
        elif declared not in expected['present_paths'] or not isinstance(fingerprint, list) or len(fingerprint) != 16 or any((type(byte) is not int or not 0 <= byte <= 255 for byte in fingerprint)):
            return False
    return True

def profile_case_inventory(profile_root: Path) -> list[dict[str, object]]:
    """Derive canonical case identities after the staged profile is sealed."""
    inventory = []
    profile_path = profile_root / 'benchmark.toml'
    profile = tomllib.loads(profile_path.read_text(encoding='utf-8')) if profile_path.is_file() else {}
    mode = profile.get('project-context-mode', 'isolated')
    for rule_root in sorted((profile_root / 'rules').iterdir(), key=lambda path: path.name):
        cases_path = rule_root / 'cases.toml'
        if not cases_path.is_file():
            continue
        cases = tomllib.loads(cases_path.read_text(encoding='utf-8'))['cases']
        for case in cases:
            identity = {'rule': rule_root.name, 'name': case['name']}
            encoded = json.dumps(case, sort_keys=True, separators=(',', ':'), ensure_ascii=False)
            source_digest = hashlib.sha256()
            for file, source in sorted(case['files'].items()):
                source_digest.update(file.encode('utf-8'))
                source_digest.update(b'\x00')
                source_digest.update(source.encode('utf-8'))
            expected = sorted(({'rule': finding.get('rule', rule_root.name), 'file': finding['file'], 'start': finding['start'], 'end': finding['end']} for finding in case.get('expected', [])), key=lambda row: (row['rule'], row['file'], row['start'], row['end']))
            inventory.append({**identity, 'project_context_mode': mode, 'context_identity': expected_context_identity(case, mode), 'case_sha256': hashlib.sha256(encoded.encode()).hexdigest(), 'source_sha256': source_digest.hexdigest(), 'files': sorted(case['files']), 'comparison': case.get('comparison', 'rule'), 'expected': expected, 'reference_discrepancy': reference_discrepancies.validate_reference_discrepancy(profile_root, case, rule_root.name)})
    inventory.sort(key=lambda case: (case['rule'], case['name']))
    ids = [(case['rule'], case['name']) for case in inventory]
    if not ids or len(ids) != len(set(ids)):
        raise ValueError('staged case inventory is empty or has duplicate (rule, name) identities')
    return inventory

def discrepancy_record_pair_valid(case: dict[str, object], sides: dict[str, dict[str, object]]) -> bool:
    """Native attestations bind the actually verified request, not inferred historical settings."""
    certificate = case.get('reference_discrepancy')
    if certificate is None:
        return True
    identity = certificate['identity']
    for side, row in sides.items():
        proof = row.get('discrepancy_validation')
        if not isinstance(proof, dict) or proof != sides['reference'].get('discrepancy_validation'):
            return False
        checks = {'settings_sha256': 'settings-sha256', 'profile_sha256': 'profile-sha256', 'source_sha256': 'source-sha256', 'reference_output_sha256': 'reference-output-sha256', 'intended_output_sha256': 'intended-output-sha256'}
        if any((proof.get(key) != identity[value] for key, value in checks.items())):
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

def record_contract_satisfied(case: dict[str, object], sides: dict[str, dict[str, object]]) -> bool:
    """Apply the declared contract without promoting unavailable reference accounting to zero."""
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

def complete_case_ids(selected_cases: list[dict[str, object]], records: list[dict[str, object]]) -> set[str]:
    """Return cases with one valid, durable full report from each contestant."""
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
            and row.get('files') == expected_case['files']
            and row.get('expected') == expected_case['expected']
            and isinstance(row.get('case_passed'), bool)
            for row in sides.values()
        )
        if (identities_match and context_receipts_valid(expected_case, sides)
            and discrepancy_record_pair_valid(expected_case, sides)
            and record_contract_satisfied(expected_case, sides)):
            completed.add(case_id)
    return completed

def case_expectation_summary(inventory: list[dict[str, object]], records: list[dict[str, object]]) -> dict[str, int]:
    """Raw parity and independently intended behavior are separate, never interchangeable totals."""
    completed = complete_case_ids(inventory, records)
    grouped: dict[str, dict[str, dict[str, object]]] = {}
    for row in records:
        case_id = f"{row.get('rule')}/{row.get('name')}"
        if case_id in completed:
            grouped.setdefault(case_id, {})[row['side']] = row
    counts = {'complete_case_pairs': len(completed), 'incomplete_cases': len(inventory) - len(completed), 'raw_parity_matches': 0, 'raw_parity_mismatches': 0, 'documented_reference_discrepancies': 0, 'unexplained_mismatches': 0, 'intended_behavior_passes': 0, 'intended_behavior_failures': 0, 'diagnostic_parity_matches': 0, 'diagnostic_parity_mismatches': 0, 'execution_metadata_exact': 0, 'execution_metadata_different': 0, 'execution_metadata_incomparable': 0, 'typed_candidate_execution_complete': 0, 'legacy_or_unknown_candidate_execution': 0}
    for case in inventory:
        sides = grouped.get(f"{case['rule']}/{case['name']}")
        if sides is None:
            continue
        reference, candidate = (sides['reference'], sides['candidate'])
        diagnostic_match = diagnostic_multiset(reference['diagnostics']) == diagnostic_multiset(candidate['diagnostics'])
        counts['diagnostic_parity_matches' if diagnostic_match else 'diagnostic_parity_mismatches'] += 1
        counts[f'execution_metadata_{record_execution_comparison(reference, candidate)}'] += 1
        counts['typed_candidate_execution_complete' if record_execution_complete(candidate) else 'legacy_or_unknown_candidate_execution'] += 1
        raw_match = diagnostic_multiset(reference['diagnostics']) == diagnostic_multiset(candidate['diagnostics']) and reference.get('skipped', []) == candidate.get('skipped', [])
        intended_pass = candidate.get('case_passed') is True
        counts['raw_parity_matches' if raw_match else 'raw_parity_mismatches'] += 1
        counts['intended_behavior_passes' if intended_pass else 'intended_behavior_failures'] += 1
        documented = not raw_match and case.get('reference_discrepancy') is not None and (reference.get('case_passed') is True) and intended_pass
        if documented:
            counts['documented_reference_discrepancies'] += 1
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
        valid_ids = complete_case_ids(inventory, records)
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

def ruff_location_to_byte(source: str, location: dict[str, object]) -> int:
    """Convert Ruff's one-based Unicode row/column to the wire's UTF-8 byte offset."""
    row = location.get('row')
    column = location.get('column')
    if not isinstance(row, int) or not isinstance(column, int) or row < 1 or (column < 1):
        raise ValueError('Ruff returned an invalid source location')
    separators = list(re.finditer('\\r\\n|\\r|\\n', source))
    starts = [0, *(match.end() for match in separators)]
    if row > len(starts):
        if row == len(starts) + 1 and column == 1:
            return len(source.encode('utf-8'))
        raise ValueError('Ruff returned a row beyond the staged source')
    start = starts[row - 1]
    end = separators[row - 1].start() if row <= len(separators) else len(source)
    line = source[start:end]
    prefix = line[:column - 1]
    if len(prefix) != column - 1:
        raise ValueError('Ruff returned a column beyond the staged source')
    return len(source[:start].encode('utf-8')) + len(prefix.encode('utf-8'))

def check_run_ryo_settings(request: dict[str, object], expected: dict[str, object], *, expected_cache_path: Path | None=None) -> None:
    """Refuse wrong phase, target, context, selection or unreviewed request flags."""
    if set(request) != {'root', 'rules', 'query_modules', 'rule_applicability', 'parser_configuration', 'files', 'cache'}:
        raise ValueError('direct worker request has unreviewed flags or context')
    if expected['project_context_mode'] != 'isolated' or expected['flags'] != {} or request['parser_configuration'] != expected['parser_configuration'] or (request['rule_applicability'] != expected['rule_applicability']) or (sorted(request['rule_applicability']) != expected['defaults']) or (request['cache'] != (str(expected_cache_path) if expected_cache_path is not None else None)):
        raise ValueError('direct worker request differs from canonical execution settings')

def complete_run_ryo_report(status: int, stdout: str, stderr: str, files: list[str], rules: list[str], *, request: dict[str, object] | None=None, settings: dict[str, object] | None=None, expected_cache_path: Path | None=None) -> dict[str, object]:
    """Validate complete reports with default-disabled or explicitly bound cache state."""
    if (request is None) != (settings is None):
        raise ValueError('direct report settings must be supplied with their request')
    if expected_cache_path is not None and request is None:
        raise ValueError('cache state requires the actual request and canonical settings')
    if request is not None:
        check_run_ryo_settings(request, settings, expected_cache_path=expected_cache_path)
    if status != 0 or stderr:
        raise ValueError('direct worker did not complete successfully')
    report = json.loads(stdout)
    if not isinstance(report, dict) or set(report) != {'execution_metadata', 'diagnostics', 'skipped', 'coverage', 'files'}:
        raise ValueError('fresh direct worker report has an invalid schema')
    if report['execution_metadata'] != {'availability': 'complete'}:
        raise ValueError('fresh direct worker execution metadata is not explicitly complete')
    if report['files'] != sorted(set(files)):
        raise ValueError('direct worker file inventory is incomplete')
    coverage = report['coverage']
    if not isinstance(coverage, dict) or set(coverage) != {'selected', 'evaluated', 'skipped', 'unknown_or_unread'}:
        raise ValueError('direct worker coverage is incomplete')
    if any((type(value) is not int or value < 0 for value in coverage.values())):
        raise ValueError('direct worker coverage contains invalid counts')
    selected = len(set(files)) * len(set(rules))
    if coverage['selected'] != selected or coverage['unknown_or_unread'] != 0 or coverage['evaluated'] + coverage['skipped'] != selected:
        raise ValueError('direct worker did not account for every rule/file pair')
    if not isinstance(report['skipped'], list) or len(report['skipped']) != coverage['skipped']:
        raise ValueError('direct worker skip receipts are incomplete')
    if not isinstance(report['diagnostics'], list):
        raise ValueError('direct worker diagnostics are missing')
    for diagnostic in report['diagnostics']:
        if not isinstance(diagnostic, dict) or set(diagnostic) != {'rule', 'file', 'start', 'end'} or diagnostic['file'] not in files or (diagnostic['rule'] not in [*rules, 'invalid-syntax']) or (type(diagnostic['start']) is not int) or (type(diagnostic['end']) is not int) or (not 0 <= diagnostic['start'] <= diagnostic['end']):
            raise ValueError('direct worker diagnostic has an invalid schema')
    return {key: report[key] for key in ('diagnostics', 'skipped', 'coverage', 'files')}
