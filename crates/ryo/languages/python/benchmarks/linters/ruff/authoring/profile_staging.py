"""Profile staging: linter-specific authoring and proof utilities."""
from __future__ import annotations
import collections
import contextlib
import hashlib
import json
import io
import shutil
import subprocess
import tarfile
import tempfile
from pathlib import Path
import tomllib
from authoring import case_records
from authoring import profile_config
from authoring import profile_proofs
from authoring import profile_sources
from authoring import profile_worker
from authoring import reference_discrepancies

def stage_directory(name: str, stack: contextlib.ExitStack, output_directory: Path | None) -> Path:
    """Use a durable repro directory when requested, otherwise a temporary one."""
    if output_directory is not None:
        destination = output_directory / 'profiles' / name / 'repro'
        destination.mkdir(parents=True, exist_ok=False)
        return destination
    return Path(stack.enter_context(tempfile.TemporaryDirectory(prefix=f'ryo-ruff-{name}-')))

def stage_binding(manifest: dict[str, object], selected: list[str], stack: contextlib.ExitStack, output_directory: Path | None=None) -> list[tuple[str, Path]]:
    staged = []
    profiles = manifest['profiles']
    for profile_name in selected:
        recorded = profiles[profile_name]
        rule = recorded.get('rule_id', profile_name)
        profile = profile_sources.prepared_profile_path(profile_name, recorded)
        profile_data = tomllib.loads(profile.read_text(encoding='utf-8'))
        explicit_native = recorded.get('selection_contract') == 'explicit-native-standalone'
        if not explicit_native and profile_data.get('defaults') != [rule]:
            raise ValueError(f'profile must select only {rule}')
        if 'selection_contract' in recorded and (not explicit_native):
            raise ValueError('unreviewed standalone selection contract')
        if not profile_sources.profile_tool_matches(profile_data, manifest):
            raise ValueError(f'tool identity mismatch for {rule}')
        candidate_path = profile_sources.prepared_candidate_path(rule, recorded)
        cases_path = profile_sources.prepared_case_path(rule, recorded)
        cases = tomllib.loads(cases_path.read_text(encoding='utf-8'))['cases']
        names = [case['name'] for case in cases]
        if recorded.get('selected_rules') != [rule] or names != recorded['case_names'] or len(cases) != recorded['case_count']:
            raise ValueError(f'case inventory mismatch for {rule}')
        if 'oracle_file' in recorded:
            oracle = profile_config.REPOSITORY / recorded['oracle_file']
            if not oracle.is_file() or profile_config.sha256(oracle) != recorded['oracle_sha256']:
                raise ValueError(f'oracle evidence mismatch for {rule}')
        if 'reference_audit_file' in recorded:
            audit = profile_config.REPOSITORY / recorded['reference_audit_file']
            if not audit.is_file() or profile_config.sha256(audit) != recorded['reference_audit_sha256']:
                raise ValueError(f'reference case audit mismatch for {rule}')
        destination = stage_directory(profile_name, stack, output_directory)
        rule_dir = destination / 'rules' / rule
        rule_dir.mkdir(parents=True)
        registry = profile_sources.profile_modules(profile_data)
        profile_sources.copy_modules(registry, destination)
        target_version = recorded.get('target_version', 'py310')
        profile_sources.write_staged_profile(profile, destination / 'benchmark.toml', registry, profile_sources.parser_configuration(target_version), profile_proofs.selected_rule_applicability(manifest, [rule, *profile_data.get('supporting-rules', [])]), selected_rules=[rule] if explicit_native else None)
        profile_proofs.check_staged_applicability(destination / 'benchmark.toml', [rule], manifest)
        shutil.copyfile(candidate_path, rule_dir / 'query.ryoql')
        shutil.copyfile(cases_path, rule_dir / 'cases.toml')
        profile_sources.copy_supporting_queries(profile, destination)
        reference_discrepancies.stage_discrepancy_artifacts(profile, destination, cases, rule, source_root=profile_config.discrepancy_source_root(rule, profile))
        profile_proofs.check_canonical_standard_suppression(destination / 'benchmark.toml', {rule: candidate_path})
        staged.append((profile_name, destination))
    return staged

def stage_combined(profile_name: str, descriptor: dict[str, object], manifest: dict[str, object], stack: contextlib.ExitStack, output_directory: Path | None=None) -> tuple[str, Path]:
    profile_path = profile_config.REPOSITORY / descriptor['profile_file']
    profile_data = tomllib.loads(profile_path.read_text(encoding='utf-8'))
    selected_rules = descriptor['selected_rules']
    if profile_data.get('defaults') != selected_rules or len(selected_rules) != len(set(selected_rules)):
        raise ValueError(f'{profile_name} selection does not match its prepared descriptor')
    if not profile_sources.profile_tool_matches(profile_data, manifest):
        raise ValueError(f'tool identity mismatch for {profile_name}')
    if profile_config.sha256(profile_path) != descriptor['benchmark_sha256']:
        raise ValueError(f'{profile_name} benchmark hash mismatch')
    overlays_path = profile_config.REPOSITORY / descriptor['expectations_file']
    if profile_config.sha256(overlays_path) != descriptor['expectations_sha256']:
        raise ValueError(f'{profile_name} expectation overlay hash mismatch')
    overlay = json.loads(overlays_path.read_text(encoding='utf-8'))
    oracle_path = profile_config.REPOSITORY / descriptor['oracle_file']
    oracle = json.loads(oracle_path.read_text(encoding='utf-8'))
    full_report = descriptor.get('expectations_mode') == 'complete-full-report'
    if overlay.get('profile') != profile_name or overlay.get('mode', 'shape-overlay') != descriptor.get('expectations_mode', 'shape-overlay') or overlay.get('reference_sha256') not in manifest['reference_sha256'] or (overlay.get('selection_count') != len(selected_rules)) or (overlay.get('selected_rules_sha256') != hashlib.sha256(json.dumps(sorted(selected_rules), separators=(',', ':')).encode('utf-8')).hexdigest()) or (overlay.get('oracle_sha256') != descriptor['oracle_sha256']) or (overlay.get('oracle_file') != descriptor.get('oracle_file')):
        raise ValueError(f'{profile_name} expectation overlay identity mismatch')
    overlays = {}
    added_findings = 0
    expected_findings = 0
    for item in overlay['cases']:
        key = (item['rule'], item['case'])
        if key in overlays or item['rule'] not in selected_rules:
            raise ValueError(f'duplicate or unselected {profile_name} overlay case: {key}')
        cases_path = combined_case_path(item['rule'], descriptor)
        canonical_case_label = (profile_config.RUFF_DIR / 'rules' / item['rule'] / 'cases.toml').relative_to(profile_config.REPOSITORY).as_posix()
        if item['cases_file'] != canonical_case_label:
            raise ValueError(f'{profile_name} overlay names a noncanonical case identity: {key}')
        if profile_config.sha256(cases_path) != item['canonical_cases_sha256']:
            raise ValueError(f"{profile_name} canonical case hash mismatch: {item['cases_file']}")
        source_cases = tomllib.loads(cases_path.read_text(encoding='utf-8'))['cases']
        matching = [case for case in source_cases if case['name'] == item['case']]
        if len(matching) != 1:
            raise ValueError(f'{profile_name} overlay case is absent or duplicated: {key}')
        case = matching[0]
        if not full_report and case.get('comparison', 'rule') != 'full':
            raise ValueError(f'{profile_name} overlay requires a full-report case: {key}')
        if case_records.source_hash(case['files']) != item['case_source_sha256']:
            raise ValueError(f'{profile_name} case source hash mismatch: {key}')
        findings = item['expected']
        for finding in findings:
            if not isinstance(finding.get('rule'), str) or not isinstance(finding.get('file'), str) or (not isinstance(finding.get('start'), int)) or (not isinstance(finding.get('end'), int)) or (finding['start'] < 0) or (finding['end'] < finding['start']) or (finding['file'] not in case['files']) or (finding['rule'] not in selected_rules and finding['rule'] != 'invalid-syntax'):
                raise ValueError(f'invalid {profile_name} expected finding: {key}')
        if case.get('comparison', 'rule') == 'full' and any(('rule' not in entry for entry in case['expected'])):
            raise ValueError(f'{profile_name} canonical full-report finding lacks an explicit rule: {key}')
        original = case_records.diagnostic_multiset([{**entry, 'rule': entry.get('rule', item['rule'])} for entry in case['expected']])
        replacement = case_records.diagnostic_multiset(findings)
        if original - replacement:
            raise ValueError(f'{profile_name} overlay drops canonical findings: {key}')
        if not full_report:
            added_findings += sum((replacement - original).values())
        expected_findings += len(findings)
        overlays[key] = (item, case)
    if full_report:
        if len(overlays) != descriptor['case_count'] or len(overlays) != overlay['case_count']:
            raise ValueError(f'{profile_name} complete expected inventory count mismatch')
    elif len(overlays) != overlay['changed_case_count'] or added_findings != overlay['additional_diagnostic_count']:
        raise ValueError(f'{profile_name} overlay diagnostic totals mismatch')
    if expected_findings != overlay['expected_diagnostic_count']:
        raise ValueError(f'{profile_name} overlay diagnostic totals mismatch')
    oracle_cases: dict[tuple[str, str], dict[str, object]] = {}
    for oracle_case in oracle['cases']:
        key = (oracle_case['rule'], oracle_case.get('case', oracle_case.get('name')))
        if key in oracle_cases:
            raise ValueError(f'{profile_name} oracle has duplicate case identity: {key}')
        oracle_cases[key] = oracle_case
    if full_report and len(oracle_cases) != descriptor['case_count']:
        raise ValueError(f'{profile_name} oracle case count mismatch')
    for key, (item, _case) in overlays.items():
        oracle_case = oracle_cases.get(key)
        if oracle_case is None or oracle_case['case_source_sha256'] != item['case_source_sha256'] or case_records.diagnostic_multiset(item['expected']) != case_records.diagnostic_multiset(oracle_case['expected']):
            raise ValueError(f'{profile_name} overlay diagnostics differ from the sealed oracle: {key}')
    inventory = []
    staged_cases: dict[str, list[dict[str, object]]] = {}
    for rule in selected_rules:
        query = combined_query_path(rule, descriptor)
        cases_path = combined_case_path(rule, descriptor)
        if not query.is_file() or not cases_path.is_file():
            raise ValueError(f'{profile_name} source is missing for {rule}')
        cases = tomllib.loads(cases_path.read_text(encoding='utf-8'))['cases']
        staged_cases[rule] = cases
        for case in cases:
            comparison = 'full' if full_report else case.get('comparison', 'rule')
            inventory.append({'rule': rule, 'name': case['name'], 'comparison': comparison})
    counts = collections.Counter({'full': len(inventory)}) if full_report else collections.Counter((item.get('comparison', 'rule') for cases in staged_cases.values() for item in cases))
    if len(inventory) != descriptor['case_count'] or counts != descriptor['comparison_counts']:
        raise ValueError(f'{profile_name} comparison inventory mismatch')
    if full_report and set(overlays) != {(item['rule'], item['name']) for item in inventory}:
        raise ValueError(f'{profile_name} complete expected inventory does not match selected cases')
    inventory_hash = hashlib.sha256(json.dumps(inventory, ensure_ascii=False, separators=(',', ':')).encode('utf-8')).hexdigest()
    if inventory_hash != descriptor['inventory_sha256']:
        raise ValueError(f'{profile_name} inventory hash mismatch')
    destination = stage_directory(profile_name, stack, output_directory)
    registry = profile_sources.combined_modules(profile_data, selected_rules, manifest)
    profile_sources.copy_modules(registry, destination)
    target_version = descriptor.get('target_version', 'py310')
    profile_sources.write_staged_profile(profile_path, destination / 'benchmark.toml', registry, profile_sources.parser_configuration(target_version), profile_proofs.selected_rule_applicability(manifest, selected_rules))
    profile_proofs.check_staged_applicability(destination / 'benchmark.toml', selected_rules, manifest)
    for rule in selected_rules:
        rule_dir = destination / 'rules' / rule
        rule_dir.mkdir(parents=True)
        query_path = combined_query_path(rule, descriptor)
        shutil.copyfile(query_path, rule_dir / 'query.ryoql')
        cases = staged_cases[rule]
        for case in cases:
            overlay_entry = overlays.get((rule, case['name']))
            if overlay_entry is not None:
                case['expected'] = overlay_entry[0]['expected']
            elif full_report:
                raise ValueError(f"{profile_name} is missing complete expected case: {rule}/{case['name']}")
            if full_report:
                case['comparison'] = 'full'
        profile_sources.write_cases(rule_dir / 'cases.toml', cases)
    profile_proofs.check_canonical_standard_suppression(destination / 'benchmark.toml', {rule: combined_query_path(rule, descriptor) for rule in selected_rules})
    return (profile_name, destination)

def combined_input_path(value: str, descriptor: dict[str, object]) -> Path:
    """Resolve a combined input only when its exact path is sealed and contained."""
    if value not in descriptor['input_files']:
        raise ValueError(f'combined input is not sealed: {value}')
    relative = Path(value)
    if relative.is_absolute() or any((part in ('.', '..') for part in relative.parts)):
        raise ValueError(f'unsafe combined input path: {value}')
    resolved = (profile_config.REPOSITORY / relative).resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(profile_config.RUFF_DIR.resolve()):
        raise ValueError(f'combined input is outside Ruff profile: {value}')
    cursor = profile_config.RUFF_DIR
    for part in relative.relative_to(profile_config.RUFF_DIR.relative_to(profile_config.REPOSITORY)).parts:
        cursor = cursor / part
        if cursor.is_symlink():
            raise ValueError(f'combined input traverses a symlink: {value}')
    return resolved

def combined_query_filename(rule: str, descriptor: dict[str, object]) -> str:
    """Return the sealed source basename for canonical combined queries."""
    if rule in descriptor.get('query_files', {}):
        return combined_input_path(descriptor['query_files'][rule], descriptor).name
    return combined_filename(rule, descriptor, 'candidate.ryoql', 'query.ryoql')

def combined_query_path(rule: str, descriptor: dict[str, object]) -> Path:
    paths = descriptor.get('query_files', {})
    if rule in paths:
        return combined_input_path(paths[rule], descriptor)
    source_dir = profile_config.RUFF_DIR / 'rules' / rule
    return source_dir / combined_filename(rule, descriptor, 'candidate.ryoql', 'query.ryoql')

def combined_case_path(rule: str, descriptor: dict[str, object]) -> Path:
    paths = descriptor.get('case_files', {})
    if rule in paths:
        return combined_input_path(paths[rule], descriptor)
    source_dir = profile_config.RUFF_DIR / 'rules' / rule
    return source_dir / combined_filename(rule, descriptor, 'cases.toml')

def combined_filename(rule: str, descriptor: dict[str, object], *names: str) -> str:
    """Select one per-rule artifact filename from a combined cohort's sealed inputs."""
    inputs = set(descriptor['input_files'])
    source_dir = f'crates/ryo/languages/python/benchmarks/linters/ruff/rules/{rule}'
    candidates = [name for name in names if f'{source_dir}/{name}' in inputs]
    if len(candidates) != 1:
        raise ValueError(f'combined profile must seal exactly one {names} source for {rule}')
    return candidates[0]

MEASURED_INPUTS = ('rules', 'fragments', 'benchmark.toml')

def staged_digest(profile_root: Path) -> str:
    """The digest a measurement receipt records: the sha256 of the lines `<sha256>  <path>` of every staged file, in path order."""
    return hashlib.sha256(''.join(f'{digest}  {path}\n' for path, digest in sorted(profile_worker.staged_hashes(profile_root).items())).encode()).hexdigest()

def stage_measurement_request(pinned_request: Path, source: str, output_directory: Path, cache: Path) -> dict[str, object]:
    """Stage rules, fragments and benchmark.toml from the measured source commit and build the run-ryo request over them.

    The corpus side (root, files, context manifest, project inventory) is the pinned request's; every rule-side field is
    read from the staged benchmark.toml, never from the pinned request or an earlier staged copy."""
    commit = subprocess.run(['git', '-C', str(profile_config.REPOSITORY), 'rev-parse', '--verify', f'{source}^{{commit}}'], check=True, capture_output=True, text=True).stdout.strip()
    prefix = profile_config.RUFF_DIR.relative_to(profile_config.REPOSITORY).as_posix()
    archive = subprocess.run(['git', '-C', str(profile_config.REPOSITORY), 'archive', '--format=tar', commit, *(f'{prefix}/{name}' for name in MEASURED_INPUTS)], check=True, capture_output=True).stdout
    profile_root = output_directory / 'profile'
    profile_root.mkdir(parents=True, exist_ok=False)
    with tarfile.open(fileobj=io.BytesIO(archive)) as members:
        for member in members.getmembers():
            if not member.isfile():
                continue
            destination = profile_root / Path(member.name).relative_to(prefix)
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(members.extractfile(member).read())
    profile = tomllib.loads((profile_root / 'benchmark.toml').read_text(encoding='utf-8'))
    request = json.loads(pinned_request.read_text(encoding='utf-8'))
    request.update({
        'cache': str(cache),
        'rules': str(profile_root / 'rules'),
        'selected_rules': profile['defaults'],
        'rule_applicability': profile['rule-applicability'],
        'query_modules': profile['query-modules'],
        'rule_observations': profile['rule-observations'],
        'rule_context': {'tables': profile['rule-context']['tables'], 'configuration': profile['rule-context']['defaults']},
        'parser_configuration': profile['parser-configuration'],
        'project_context_mode': profile['project-context-mode'],
        'project_search_roots': profile['project-search-roots'],
        'performance': {'diagnostics': request['performance']['diagnostics']},
    })
    (output_directory / 'request.json').write_text(json.dumps(request), encoding='utf-8')
    receipt = {'source_commit': commit, 'staged': list(MEASURED_INPUTS), 'staged_files': len(profile_worker.staged_hashes(profile_root)),
               'staged_digest': staged_digest(profile_root), 'pinned_request': str(pinned_request), 'pinned_request_sha256': profile_config.sha256(pinned_request),
               'request_sha256': profile_config.sha256(output_directory / 'request.json')}
    profile_config.atomic_json_write(output_directory / 'staged.json', receipt)
    return receipt
