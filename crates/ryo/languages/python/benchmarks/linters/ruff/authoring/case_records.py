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
from authoring import case_receipt_adapter

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

def project_inputs(case: dict[str, object], profile: dict[str, object]) -> dict[str, object] | None:
    """Canonical explicit project authority, shared with the Rust case receipt digest."""
    roots = case.get('project-search-roots', profile.get('project-search-roots'))
    inventory = case.get('given-project-inventory', {'completeness': 'partial', 'entries': []})
    if roots is not None and (not isinstance(roots, list) or any(not isinstance(root, str) or (root and (Path(root).is_absolute() or any(part in {'.', '..'} for part in Path(root).parts))) for root in roots)):
        raise ValueError('project search roots must be ordered safe relative paths')
    if not isinstance(inventory, dict) or set(inventory) != {'completeness', 'entries'} or inventory['completeness'] not in {'partial', 'complete-within-declared-scope'} or not isinstance(inventory['entries'], list):
        raise ValueError('Given project inventory has an invalid shape')
    for entry in inventory['entries']:
        if not isinstance(entry, dict) or set(entry) != {'path', 'state'} or not isinstance(entry['path'], str) or not entry['path'] or Path(entry['path']).is_absolute() or any(part in {'.', '..'} for part in Path(entry['path']).parts) or entry['state'] not in {'file', 'directory', 'other', 'missing', 'unknown'}:
            raise ValueError('Given project entry has an invalid path or kind')
    if roots is None and inventory['completeness'] == 'complete-within-declared-scope':
        raise ValueError('complete Given project inventory requires declared search roots')
    if roots is None and inventory == {'completeness': 'partial', 'entries': []}:
        return None
    return {'project_search_roots': roots, 'given_project_inventory': inventory}

def project_inputs_digest(inputs: dict[str, object] | None) -> str | None:
    if inputs is None:
        return None
    return hashlib.sha256(json.dumps(inputs, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode('utf-8')).hexdigest()

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
            context_declaration = profile.get('rule-context')
            context_override = case.get('rule-context')
            if context_override is not None and context_declaration is None:
                raise ValueError('rule context override requires an explicit profile declaration')
            if context_declaration is not None:
                if not isinstance(context_declaration, dict) or set(context_declaration) != {'tables', 'defaults'}:
                    raise ValueError('invalid rule context declaration')
                prepared_rule_context = {'tables': context_declaration['tables'], 'configuration': context_override if context_override is not None else context_declaration['defaults']}
            else:
                prepared_rule_context = None
            declarations = case.get('rule-observations')
            selected = case.get('selected-rules')
            if prepared_rule_context is not None and selected is None:
                selected = profile['defaults']
            metadata = case.get('file-metadata')
            metadata_sha = case_receipt_adapter.file_metadata_digest(metadata)
            extra = {}
            if prepared_rule_context is not None:
                extra['rule_context'] = prepared_rule_context
            if metadata is not None:
                extra.update(file_metadata=metadata, file_metadata_sha256=metadata_sha)
            if profile.get('file-observation') is not None:
                extra['file_observation'] = profile['file-observation']
            if declarations is not None:
                extra['rule_observations'] = declarations
            if selected is not None:
                extra['selected_rules'] = selected
            inputs = project_inputs(case, profile)
            if inputs is not None:
                extra.update(inputs)
                extra['project_inputs_sha256'] = project_inputs_digest(inputs)
            inventory.append({**extra, **identity, 'project_context_mode': mode, 'context_identity': expected_context_identity(case, mode), 'case_sha256': hashlib.sha256(encoded.encode()).hexdigest(), 'source_sha256': source_digest.hexdigest(), 'files': sorted(case.get('requested-files', case['files'])), 'comparison': case.get('comparison', 'rule'), 'expected': expected, 'reference_discrepancy': reference_discrepancies.validate_reference_discrepancy(profile_root, case, rule_root.name)})
    inventory.sort(key=lambda case: (case['rule'], case['name']))
    ids = [(case['rule'], case['name']) for case in inventory]
    if not ids or len(ids) != len(set(ids)):
        raise ValueError('staged case inventory is empty or has duplicate (rule, name) identities')
    return inventory


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
    required = {'root', 'rules', 'query_modules', 'rule_applicability', 'parser_configuration', 'files', 'cache'}
    if set(request) != required | ({'project_search_roots'} if 'project_search_roots' in expected else set()):
        raise ValueError('direct worker request has unreviewed flags or context')
    if request.get('project_search_roots') != expected.get('project_search_roots'):
        raise ValueError('direct worker project search authority differs from the profile')
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
