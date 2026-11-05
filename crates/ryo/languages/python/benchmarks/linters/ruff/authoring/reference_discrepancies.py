"""Reference discrepancies: linter-specific authoring and proof utilities."""
from __future__ import annotations
import hashlib
import json
import re
import shutil
from pathlib import Path
import tomllib
from authoring import case_records
from authoring import case_receipt_adapter
from authoring import profile_config

def discrepancy_json_hash(value: object) -> str:
    """Canonical certificate encoding shared with the native benchmark harness."""
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()

def discrepancy_output(files: list[str], findings: list[dict[str, object]]) -> dict[str, object]:
    """Bind exact files and the complete diagnostic multiset, preserving duplicates."""
    return {'files': sorted(files), 'diagnostics': sorted(findings, key=lambda row: (row['rule'], row['file'], row['start'], row['end'])), 'skipped': []}

def selected_case_files(case: dict[str, object]) -> list[str]:
    """Return the files the contestant actually runs, excluding staged context sources."""
    supplied = case['files']
    selected = case.get('requested-files', list(supplied))
    if not isinstance(supplied, dict) or not isinstance(selected, list) or any(not isinstance(file, str) or file not in supplied for file in selected) or len(selected) != len(set(selected)):
        raise ValueError('invalid selected discrepancy files')
    return sorted(selected)

def validate_reference_discrepancy(profile_root: Path, case: dict[str, object], parent_rule: str) -> dict[str, object] | None:
    """Admit two concrete expectations only with pinned independent language evidence.

    This validates evidence identity, not the truth of a specification interpretation.
    Runtime verification must additionally bind the actual invocation and approved tool.
    """
    annotation = case.get('reference-discrepancy')
    tool_contract = case.get('tool-contract-difference')
    if annotation is not None and tool_contract is not None:
        raise ValueError('a case cannot declare two discrepancy authorities')
    basis = 'tool-contract-difference' if tool_contract is not None else 'language-defect'
    annotation = tool_contract if tool_contract is not None else annotation
    if annotation is None:
        return None
    if basis == 'language-defect' and isinstance(annotation, dict) and 'process' in annotation:
        return serialize_process_discrepancy(profile_root, case, annotation)
    if not isinstance(annotation, dict) or set(annotation) != {'expected', 'identity', 'evidence', 'reason'}:
        raise ValueError('invalid reference-discrepancy schema')
    if case.get('comparison', 'rule') != 'full' or not str(annotation['reason']).strip():
        raise ValueError('reference discrepancy requires full comparison and a reason')
    identity = annotation['identity']
    identity_fields = {'tool', 'version', 'observed-executable-sha256', 'profile-sha256', 'settings-sha256', 'source-sha256', 'reference-output-sha256', 'intended-output-sha256'}
    if not isinstance(identity, dict) or not identity_fields.issubset(identity) or set(identity) - identity_fields - {'file-metadata-sha256'}:
        raise ValueError('incomplete reference discrepancy identity')
    if identity.get('file-metadata-sha256') != case_receipt_adapter.file_metadata_digest(case.get('file-metadata')):
        raise ValueError('stale discrepancy file metadata identity')
    for field in (set(identity) - {'tool', 'version'}):
        if not isinstance(identity[field], str) or re.fullmatch('[a-f0-9]{64}', identity[field]) is None:
            raise ValueError('reference discrepancy requires lowercase SHA256 identities')
    profile_path = profile_root / 'benchmark.toml'
    profile = tomllib.loads(profile_path.read_text(encoding='utf-8'))
    if identity['tool'] != profile['tool'] or identity['version'] != profile['version'] or identity['observed-executable-sha256'] not in profile['sha256'] or (identity['profile-sha256'] != profile_config.sha256(profile_path)):
        raise ValueError('stale discrepancy tool/profile identity')
    source = hashlib.sha256()
    for file, text in sorted(case['files'].items()):
        source.update(file.encode())
        source.update(b'\x00')
        source.update(text.encode())
    if source.hexdigest() != identity['source-sha256']:
        raise ValueError('stale discrepancy source identity')

    selected_files = selected_case_files(case)

    def expected_rows(rows: object) -> list[dict[str, object]]:
        if not isinstance(rows, list):
            raise ValueError('discrepancy expectation must be an array')
        normalized = []
        for row in rows:
            if not isinstance(row, dict) or set(row) != {'rule', 'file', 'start', 'end'}:
                raise ValueError('full discrepancy expectations require explicit diagnostic identities')
            if row['file'] not in selected_files or not isinstance(row['rule'], str) or (not row['rule']):
                raise ValueError('discrepancy diagnostic outside selected source')
            text = case['files'][row['file']].encode()
            if type(row['start']) is not int or type(row['end']) is not int or (not 0 <= row['start'] <= row['end'] <= len(text)):
                raise ValueError('invalid discrepancy diagnostic range')
            text[:row['start']].decode()
            text[:row['end']].decode()
            normalized.append(dict(row))
        return normalized
    intended = discrepancy_output(selected_files, expected_rows(case['expected']))
    reference = discrepancy_output(selected_files, expected_rows(annotation['expected']))
    if intended == reference:
        raise ValueError('identical expectations are not a reference discrepancy')
    if discrepancy_json_hash(intended) != identity['intended-output-sha256'] or discrepancy_json_hash(reference) != identity['reference-output-sha256']:
        raise ValueError('stale reference/intended expectation identity')
    evidence = annotation['evidence']
    if not isinstance(evidence, list) or not evidence:
        raise ValueError('reference discrepancy requires independent evidence')
    seen_kinds = []
    for item in evidence:
        language_fields = {'path', 'sha256', 'kind', 'language-version', 'target-configuration', 'version-rationale'}
        contract_fields = {'path', 'sha256', 'kind', 'source-revision', 'source-path', 'rationale'}
        prior_fields = {'path', 'sha256', 'kind', 'worker-sha256', 'control-source-sha256', 'control-query-sha256', 'control-profile-sha256', 'witness-case', 'rationale'}
        if not isinstance(item, dict):
            raise ValueError('incomplete discrepancy evidence')
        kind = item.get('kind')
        seen_kinds.append(kind)
        if basis == 'language-defect':
            if set(item) != language_fields or kind not in {'language-spec', 'independent-language-run'} or not str(item['language-version']).strip() or not str(item['version-rationale']).strip() or item['target-configuration'] != profile.get('parser-configuration'):
                raise ValueError('evidence does not support configured language contract')
        elif kind == 'public-tool-contract':
            if set(item) != contract_fields or not re.fullmatch('[a-f0-9]{40}', item['source-revision']) or not str(item['source-path']).strip() or not str(item['rationale']).strip():
                raise ValueError('incomplete public tool contract evidence')
        elif kind == 'prior-worker-run':
            if set(item) != prior_fields or any(not re.fullmatch('[a-f0-9]{64}', item[field]) for field in ('worker-sha256','control-source-sha256','control-query-sha256','control-profile-sha256')) or not str(item['witness-case']).strip() or not str(item['rationale']).strip():
                raise ValueError('incomplete prior worker evidence')
        else:
            raise ValueError('unsupported tool contract evidence kind')
        path = Path(item['path'])
        if path.is_absolute() or not path.parts or any((part in {'.', '..'} for part in path.parts)):
            raise ValueError('unsafe discrepancy evidence path')
        resolved = (profile_root / path).resolve(strict=True)
        if not resolved.is_relative_to(profile_root.resolve()) or not resolved.is_file() or profile_config.sha256(resolved) != item['sha256']:
            raise ValueError('stale/external evidence or unsupported language contract')
    if basis == 'tool-contract-difference' and sorted(seen_kinds) != ['prior-worker-run', 'public-tool-contract']:
        raise ValueError('tool contract difference needs public contract and prior worker evidence')
    return {'basis': basis, 'identity': identity, 'approved_executable_sha256': profile['sha256'], 'reference_expected': reference['diagnostics'], 'intended_expected': intended['diagnostics'], 'reason': annotation['reason']}

def classify_reference_discrepancy(profile_root: Path, case: dict[str, object], parent_rule: str, reference: dict[str, object], candidate: dict[str, object], *, actual_executable_sha256: str, actual_settings_sha256: str) -> dict[str, object]:
    """Retain raw parity mismatch while separately asserting the intended language behavior."""
    certificate = validate_reference_discrepancy(profile_root, case, parent_rule)
    if certificate is None:
        raise ValueError('classification requires an explicit discrepancy certificate')
    if certificate.get('process') is not None:
        raise ValueError('process discrepancy classification requires the shared receipt-file validator')
    profile = tomllib.loads((profile_root / 'benchmark.toml').read_text())
    if actual_executable_sha256 not in profile['sha256'] or actual_settings_sha256 != certificate['identity']['settings-sha256']:
        raise ValueError('unapproved executable or stale actual invocation settings')
    files = selected_case_files(case)
    for side, report in (('reference', reference), ('candidate', candidate)):
        if report.get('complete') is not True or report.get('error') is not None or report.get('files') != files or (report.get('skipped') != []) or (not isinstance(report.get('diagnostics'), list)):
            raise ValueError('discrepancy cannot accept an incomplete or failed report')
        coverage = report.get('coverage')
        if coverage is None and side == 'candidate':
            raise ValueError('candidate discrepancy classification needs accounting')
        if coverage is not None and (coverage.get('unknown_or_unread_rule_file_pairs') != 0 or coverage.get('skipped_rule_file_pairs') != 0 or coverage.get('selected_rule_file_pairs', 0) <= 0 or (coverage.get('selected_rule_file_pairs') != coverage.get('evaluated_rule_file_pairs'))):
            raise ValueError('unknown/inconsistent coverage is not a reference discrepancy')
    ref_exact = case_records.diagnostic_multiset(reference['diagnostics']) == case_records.diagnostic_multiset(certificate['reference_expected'])
    intended_exact = case_records.diagnostic_multiset(candidate['diagnostics']) == case_records.diagnostic_multiset(certificate['intended_expected'])
    raw_exact = case_records.diagnostic_multiset(reference['diagnostics']) == case_records.diagnostic_multiset(candidate['diagnostics'])
    if not ref_exact or not intended_exact:
        raise ValueError('reference or candidate violated its separate exact expectation')
    return {'raw_parity_match': raw_exact,
            'documented_reference_discrepancy': certificate['basis'] == 'language-defect',
            'documented_tool_contract_difference': certificate['basis'] == 'tool-contract-difference',
            'intended_behavior_pass': intended_exact, 'reference_expectation_pass': ref_exact,
            'reason': certificate['reason']}

def declared_discrepancy_artifacts(profile: Path, cases: list[dict[str, object]], *, source_root: Path | None = None) -> dict[str, tuple[Path, str]]:
    """Resolve only explicitly declared, pinned artifacts within the admitted root."""
    source_root = (source_root or profile.parent).resolve(strict=True)
    artifacts: dict[str, tuple[Path, str]] = {}
    for case in cases:
        if case.get('reference-discrepancy') is not None and case.get('tool-contract-difference') is not None:
            raise ValueError('a case cannot declare two discrepancy authorities')
        annotation = case.get('reference-discrepancy') or case.get('tool-contract-difference')
        if annotation is None:
            continue
        if not isinstance(annotation, dict) or not isinstance(annotation.get('evidence'), list):
            raise ValueError('invalid discrepancy evidence declaration')
        declared = list(annotation['evidence'])
        process = annotation.get('process')
        if process is not None:
            if not isinstance(process, dict) or not isinstance(process.get('observations'), list):
                raise ValueError('invalid serialized process observation list')
            for observation in process['observations']:
                declared.extend(observation[key] for key in ('invocation', 'stdout', 'stderr'))
        for item in declared:
            if not isinstance(item, dict):
                raise ValueError('invalid discrepancy evidence artifact')
            relative, expected = (item.get('path'), item.get('sha256'))
            if not isinstance(relative, str) or not relative or '\\' in relative or Path(relative).is_absolute() or any((part in {'', '.', '..'} for part in relative.split('/'))) or (re.match('^[A-Za-z]:', relative) is not None) or (not isinstance(expected, str)) or (re.fullmatch('[a-f0-9]{64}', expected) is None):
                raise ValueError('unsafe discrepancy evidence artifact')
            try:
                source = (source_root / relative).resolve(strict=True)
            except OSError as error:
                raise ValueError('missing discrepancy evidence artifact') from error
            if not source.is_relative_to(source_root) or not source.is_file():
                raise ValueError('external or non-file discrepancy evidence artifact')
            if profile_config.sha256(source) != expected:
                raise ValueError('stale discrepancy evidence artifact')
            prior = artifacts.get(relative)
            if prior is not None and prior != (source, expected):
                raise ValueError('conflicting discrepancy evidence declarations')
            artifacts[relative] = (source, expected)
    return artifacts

def stage_discrepancy_artifacts(source_profile: Path, destination: Path, cases: list[dict[str, object]], rule: str, *, source_root: Path | None = None) -> None:
    """Copy the declared artifacts without overwriting any canonical stage input."""
    artifacts = declared_discrepancy_artifacts(source_profile, cases, source_root=source_root)
    root = destination.resolve(strict=True)
    for relative in artifacts:
        target = destination / relative
        if target.exists() or target.is_symlink() or (not target.resolve(strict=False).is_relative_to(root)):
            raise ValueError('discrepancy evidence collides with staged input')
    for relative, (source, expected) in artifacts.items():
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        if profile_config.sha256(target) != expected:
            raise ValueError('discrepancy evidence changed while staging')
    for case in cases:
        validate_reference_discrepancy(destination, case, rule)


def serialize_process_discrepancy(profile_root: Path, case: dict[str, object], annotation: dict[str, object]) -> dict[str, object]:
    """Serialize/stage an explicit native contract; matching/admission lives only in shared Rust."""
    if set(annotation) != {'process', 'identity', 'evidence', 'reason'} or case.get('comparison') != 'full':
        raise ValueError('process discrepancy requires an explicit full-comparison wire contract')
    if not isinstance(annotation['process'], dict) or annotation['process'].get('kind') != 'process-failure':
        raise ValueError('unknown process expectation wire variant')
    profile = tomllib.loads((profile_root / 'benchmark.toml').read_text())
    intended = []
    for row in case['expected']:
        if set(row) != {'rule', 'file', 'start', 'end'}:
            raise ValueError('full process discrepancy requires explicit intended diagnostic identities')
        intended.append(dict(row))
    intended.sort(key=lambda row: (row['rule'], row['file'], row['start'], row['end']))
    # JSON serialization is also the outbound field-encoding check; do not implement native byte policy here.
    json.dumps(annotation, ensure_ascii=False)
    return {'identity': annotation['identity'], 'approved_executable_sha256': profile['sha256'],
            'reference_expected': None, 'intended_expected': intended, 'reason': annotation['reason'],
            'process': annotation['process']}
