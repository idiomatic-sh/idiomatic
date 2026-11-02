"""Rule registration: linter-specific authoring and proof utilities."""
from __future__ import annotations
import contextlib
import json
import re
import shutil
import subprocess
from pathlib import Path
import tomllib
from authoring import case_records
from authoring import profile_config
from authoring import profile_proofs
from authoring import profile_sources
from authoring import profile_worker
from authoring import reference_discrepancies

def full_inventory_ids(profile: dict[str, object], phases: dict[str, object]) -> list[str]:
    """The pinned defaults are the inventory; an admission subset cannot narrow it."""
    rules = profile.get('defaults')
    if not isinstance(rules, list) or len(rules) != 413 or any((not isinstance(rule, str) or not rule for rule in rules)) or (len(set(rules)) != 413) or (set(rules) != set(phases['shipped_default_rules'])):
        raise ValueError('full inventory requires exactly 413 unique pinned default IDs')
    return rules

def full_inventory_settings(rule: str, manifest: dict[str, object], native: dict[str, object]) -> tuple[Path, str | None, str]:
    """Preserve explicit singleton settings, otherwise an established cohort's scope."""
    for entry in native.get('profiles', []):
        if entry['rule_id'] == rule:
            return (profile_sources.prepared_profile_path(entry['profile_name'], entry), None, 'native singleton settings')
    prepared = [(name, entry) for name, entry in manifest['profiles'].items() if entry.get('rule_id', name) == rule]
    explicit = [(name, entry) for name, entry in prepared if 'profile_file' in entry]
    if len(explicit) > 1:
        raise ValueError('ambiguous explicit singleton settings')
    if explicit:
        name, entry = explicit[0]
        return (profile_sources.prepared_profile_path(name, entry), entry.get('target_version'), 'prepared per-rule settings')
    dedicated = profile_config.RUFF_DIR / 'profiles' / rule / 'benchmark.toml'
    if dedicated.is_file():
        target = prepared[0][1].get('target_version') if len(prepared) == 1 else None
        return (dedicated, target, 'dedicated singleton profile (existing parser default contract)')
    cohorts = [(name, entry) for name, entry in manifest.get('combined_profiles', {}).items() if rule in entry.get('selected_rules', [])]
    active = [(name, entry) for name, entry in cohorts if entry.get('validation_status') == 'active-current']
    choices = active or cohorts
    signatures = set()
    resolved = []
    for name, entry in choices:
        path = profile_config.REPOSITORY / entry['profile_file']
        data = tomllib.loads(path.read_text(encoding='utf-8'))
        target = entry.get('target_version', manifest.get('target_version'))
        if target is None and 'parser-configuration' not in data:
            raise ValueError('cohort has no established parser target')
        signature = (data['parser-configuration'] if 'parser-configuration' in data else profile_sources.parser_configuration(target), data.get('project-context-mode', 'isolated'))
        signatures.add(signature)
        resolved.append((path, target, f'established cohort {name}'))
    if len(signatures) != 1 or not resolved:
        raise ValueError('missing or ambiguous required singleton/cohort settings registration')
    return resolved[0]

def stage_full_inventory_rule(rule: str, output: Path, base: dict[str, object], phases: dict[str, object], manifest: dict[str, object], native: dict[str, object]) -> tuple[Path, dict[str, object]]:
    directory = profile_config.RUFF_DIR / 'rules' / rule
    query = directory / 'query.ryoql'
    cases_path = directory / 'cases.toml'
    if not cases_path.is_file():
        raise ValueError('implemented query has no canonical cases.toml')
    container = tomllib.loads(cases_path.read_text(encoding='utf-8'))
    cases = container.get('cases')
    if set(container) != {'cases'} or not isinstance(cases, list) or (not cases):
        raise ValueError('canonical case registration is empty or has unsupported settings')
    names = [case['name'] for case in cases]
    if len(names) != len(set(names)):
        raise ValueError('canonical cases contain duplicate names')
    source_profile, target, scope = full_inventory_settings(rule, manifest, native)
    data = tomllib.loads(source_profile.read_text(encoding='utf-8'))
    if any((data.get(key) != base.get(key) for key in ('tool', 'version', 'sha256'))):
        raise ValueError('per-rule profile does not retain the pinned tool identity')
    if rule not in data.get('defaults', []):
        raise ValueError('settings registration does not select this rule')
    configuration = data.get('parser-configuration', profile_sources.parser_configuration(target) if target else 'python-target=3.10')
    if target and 'parser-configuration' in data and (profile_sources.parser_configuration(target) != configuration):
        raise ValueError('conflicting explicit parser target registrations')
    registry = {**base.get('query-modules', {}), **data.get('query-modules', {})}
    closure = {}
    pending = [query]
    while pending:
        source = pending.pop()
        for logical in re.findall('^\\s*import\\s+\\w+\\s+from\\s+"([^"]+)"\\s*;', source.read_text(encoding='utf-8'), re.MULTILINE):
            if logical in closure:
                continue
            if logical not in registry:
                raise ValueError(f'undeclared canonical helper: {logical}')
            relative = registry[logical]
            profile_sources.module_path(relative)
            closure[logical] = relative
            pending.append(profile_config.RUFF_DIR / relative)
    destination = output / 'profiles' / rule / 'repro'
    rule_root = destination / 'rules' / rule
    rule_root.mkdir(parents=True)
    stage_registry = {**data.get('query-modules', {}), **closure}
    for relative in stage_registry.values():
        copied = destination / relative
        copied.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(profile_config.RUFF_DIR / relative, copied)
    projected = {key: value for key, value in data.items() if key not in {'deferred', 'pending', 'query-modules', 'rule-applicability'}}
    projected['defaults'] = [rule]
    projected['parser-configuration'] = configuration
    if stage_registry:
        projected['query-modules'] = stage_registry
    projected['rule-applicability'] = {rule: phases['rules'][rule]['applicability']}
    profile_sources.canonical_run_ryo_settings(projected, [rule], phases)
    profile = destination / 'benchmark.toml'
    profile.write_text(''.join((f'{key} = {profile_sources.toml_value(value)}\n' for key, value in projected.items())), encoding='utf-8')
    profile_sources.assert_staged_profile_fixed_point(destination)
    shutil.copyfile(query, rule_root / 'query.ryoql')
    shutil.copyfile(cases_path, rule_root / 'cases.toml')
    reference_discrepancies.stage_discrepancy_artifacts(source_profile, destination, cases, rule)
    inventory = case_records.profile_case_inventory(destination)
    profile_proofs.check_canonical_standard_suppression(profile, {rule: query})
    return (destination, {'query_file': query.relative_to(profile_config.RUFF_DIR).as_posix(), 'query_sha256': profile_config.sha256(query), 'cases_file': cases_path.relative_to(profile_config.RUFF_DIR).as_posix(), 'cases_sha256': profile_config.sha256(cases_path), 'case_count': len(inventory), 'settings_source': source_profile.relative_to(profile_config.REPOSITORY).as_posix(), 'settings_scope': scope, 'parser_configuration': configuration, 'settings_source_sha256': profile_config.sha256(source_profile), 'profile_sha256': profile_config.sha256(profile), 'module_sha256': {logical: profile_config.sha256(profile_config.RUFF_DIR / relative) for logical, relative in stage_registry.items()}, 'effective_settings': profile_sources.canonical_run_ryo_settings(projected, [rule], phases)})

def verify_full_inventory(arguments) -> int:
    """Bind this linter's convention/settings; the shared Rust runner owns execution."""
    output = arguments.output_dir.resolve()
    profile_worker.create_new_output_directory(output)
    base = tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8'))
    phases = json.loads(profile_config.RULE_PHASES_FILE.read_text(encoding='utf-8'))
    rules = full_inventory_ids(base, phases)
    manifest = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    native = profile_proofs.native_descriptor_json(profile_config.NATIVE_DESCRIPTOR_FILE.read_text(encoding='utf-8'))
    reference = arguments.ruff.resolve()
    selected = set(arguments.rules)
    if len(selected) != len(arguments.rules) or selected - set(rules):
        raise ValueError('unknown or duplicate inventory rule')
    identity = {'inventory_sha256': profile_config.sha256(profile_config.RUFF_DIR / 'benchmark.toml'), 'phase_registry_sha256': profile_config.sha256(profile_config.RULE_PHASES_FILE)}
    registration_error = None
    worker = None
    try:
        profile_proofs.verify_rule_phase_registry(manifest, phases)
        native_ids = [entry['rule_id'] for entry in native.get('profiles', [])]
        if len(native_ids) != len(set(native_ids)) or set(native_ids) - set(rules):
            raise ValueError('native settings registry has duplicate or unknown pinned IDs')
        identity['pinned_reference'] = profile_worker.reference_tool_identity(reference, manifest)
        if arguments.worker is not None:
            worker = arguments.worker.resolve()
        elif arguments.reuse_worker_from is not None:
            worker = profile_worker.reuse_frozen_worker(arguments.reuse_worker_from.resolve())[0]
        else:
            with contextlib.ExitStack() as stack:
                worker = profile_worker.build_worker_once(output, stack)[0]
        identity['worker_sha256'] = profile_config.sha256(worker)
    except Exception as error:
        registration_error = f'tool registration: {type(error).__name__}: {error}'
    registered = []
    for rule in rules:
        entry = {'rule_id': rule, 'command': None, 'reason': None, 'cases': [], 'instance': None}
        directory = profile_config.RUFF_DIR / 'rules' / rule
        if selected and rule not in selected:
            entry['reason'] = 'not selected for this invocation'
        elif not (directory / 'query.ryoql').is_file():
            entry['reason'] = 'missing implementation'
        elif registration_error is not None:
            entry['reason'] = registration_error
        else:
            try:
                stage, entry['instance'] = stage_full_inventory_rule(rule, output, base, phases, manifest, native)
                entry['cases'] = case_records.profile_case_inventory(stage)
                entry['command'] = {'executable': str(worker), 'arguments': ['verify-cases', str(reference), str(stage)], 'directory': str(profile_config.REPOSITORY / 'crates' / 'ryo'), 'environment': {}}
            except Exception as error:
                entry['reason'] = f'rule registration: {type(error).__name__}: {error}'
        registered.append(entry)
    registration = output / 'registration.json'
    profile_config.atomic_json_write(registration, {'provider': 'ruff', 'defaults': rules, 'rules': registered, 'identity': identity})
    runner = arguments.inventory_runner or shutil.which('ryo-benchmark-collector') or profile_config.REPOSITORY / 'crates' / 'ryo' / 'target' / 'release' / 'ryo-benchmark-collector'
    return subprocess.run([str(runner), 'verify-inventory', str(registration), str(output), str(arguments.rule_timeout)], check=False).returncode
