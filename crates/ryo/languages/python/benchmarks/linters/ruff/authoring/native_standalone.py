"""The native standalone registry manifest."""
from __future__ import annotations
import re
from pathlib import Path
import tomllib
from authoring import profile_config
from authoring import profile_sources
from authoring.profile_proofs import check_base_rule_applicability, verify_rule_phase_registry

def native_descriptor(manifest: dict[str, object]) -> dict[str, object]:
    """Seal the native descriptor from the live tree: schema v2, one entry per native member,
    each bound to its own profile, every identity hashed now."""
    raise NotImplementedError

def native_standalone_manifest(manifest: dict[str, object], descriptor: dict[str, object]) -> dict[str, object]:
    """Validate a descriptor against the live tree; `descriptor` is required.

    Candidate variants in the ordinary prepared profiles are never substituted,
    even when they share a diagnostic rule ID.
    """
    fields = {'schema', 'execution_contract', 'historical_certification', 'intent', 'source_profile_file', 'source_profile_sha256', 'source_settings', 'module_registry', 'module_sha256', 'phase_registry_sha256', 'query_count', 'case_count', 'profiles'}
    if set(descriptor) != fields or descriptor['schema'] != 'native-standalone-cases/v2':
        raise ValueError('unsupported native descriptor schema')
    if descriptor['execution_contract'] != 'singleton-per-declared-rule' or descriptor['historical_certification'] is not False:
        raise ValueError('native selection is a new explicit contract, never historical certification')
    if not isinstance(descriptor['intent'], str) or not descriptor['intent'].strip():
        raise ValueError('native execution intent must be documented')
    if any((type(descriptor[key]) is not int or descriptor[key] <= 0 for key in ('query_count', 'case_count'))):
        raise ValueError('native inventory counts must be positive integers')
    if not isinstance(descriptor['module_registry'], dict) or not isinstance(descriptor['module_sha256'], dict):
        raise ValueError('native module declarations must be objects')
    source = profile_sources.prepared_profile_path('native', {'profile_file': descriptor['source_profile_file']})
    if profile_config.sha256(source) != descriptor['source_profile_sha256']:
        raise ValueError('native source profile identity mismatch')
    profile = tomllib.loads(source.read_text(encoding='utf-8'))
    if not profile_sources.profile_tool_matches(profile, manifest):
        raise ValueError('native source tool identity mismatch')
    phases = verify_rule_phase_registry(manifest)
    if descriptor['phase_registry_sha256'] != profile_config.sha256(profile_config.RULE_PHASES_FILE):
        raise ValueError('native pinned phase registry identity mismatch')
    check_base_rule_applicability(profile, phases)
    source_settings = profile_sources.canonical_run_ryo_settings(profile, profile['defaults'], phases)
    if source_settings != descriptor['source_settings']:
        raise ValueError('native source execution settings mismatch')
    registry = profile_sources.profile_modules(profile)
    if registry != descriptor['module_registry'] or set(registry) != set(descriptor['module_sha256']):
        raise ValueError('native declared module registry mismatch')
    for logical, relative in registry.items():
        module = profile_config.REPOSITORY / profile_sources.module_path(relative)
        if profile_config.sha256(module) != descriptor['module_sha256'][logical]:
            raise ValueError(f'native module identity mismatch: {logical}')

    def check_imports(source_path: Path, declared: dict[str, str]=registry) -> None:
        imports = re.findall('^\\s*import\\s+\\w+\\s+from\\s+"([^"]+)"\\s*;', source_path.read_text(encoding='utf-8'), re.MULTILINE)
        if set(imports) - set(declared):
            raise ValueError(f'native query has an undeclared module: {source_path}')
    for relative in registry.values():
        check_imports(profile_config.RUFF_DIR / relative)
    native_queries = {path.relative_to(profile_config.RUFF_DIR).as_posix(): path for path in (profile_config.RUFF_DIR / 'rules').glob('*/query.ryoql')}
    if not isinstance(descriptor['profiles'], list) or not descriptor['profiles']:
        raise ValueError('native profile declarations must be a nonempty list')
    if {recorded.get('rule_id') if isinstance(recorded, dict) else None for recorded in descriptor['profiles']} != set(profile_sources.native_members()):
        raise ValueError('native declared membership differs from the native evidence')
    profiles = {}
    declared_queries = set()
    declared_rules = set()
    case_count = 0
    entry_fields = {'profile_name', 'rule_id', 'selected_rules', 'profile_file', 'candidate_file', 'query_sha256', 'cases_file', 'cases_sha256', 'case_count', 'case_names', 'effective_settings', 'profile_sha256', 'source_settings', 'module_registry', 'module_sha256'}
    for recorded in descriptor['profiles']:
        if not isinstance(recorded, dict):
            raise ValueError('unsupported native profile declaration schema')
        if set(recorded) != entry_fields:
            raise ValueError('unsupported native profile declaration schema')
        if type(recorded['case_count']) is not int or recorded['case_count'] <= 0:
            raise ValueError('native case counts must be positive integers')
        rule = recorded['rule_id']
        name = recorded['profile_name']
        if not isinstance(rule, str) or rule not in phases['shipped_default_rules']:
            raise ValueError('native rule is not in the pinned default registry')
        if name != f'native-{rule}' or rule in declared_rules or name in profiles:
            raise ValueError('duplicate or noncanonical native profile identity')
        if recorded['selected_rules'] != [rule]:
            raise ValueError('native singleton selection must be explicitly declared')
        if recorded['profile_file'] != f'profiles/{rule}/benchmark.toml':
            raise ValueError("native entry must bind its rule's own profile")
        if any((not isinstance(recorded[key], dict) for key in ('source_settings', 'module_registry', 'module_sha256'))):
            raise ValueError('native per-rule settings/module declarations must be objects')
        entry_path = profile_sources.prepared_profile_path(name, recorded)
        if profile_config.sha256(entry_path) != recorded['profile_sha256']:
            raise ValueError('native per-rule source profile identity mismatch')
        entry_profile = tomllib.loads(entry_path.read_text(encoding='utf-8'))
        if not profile_sources.profile_tool_matches(entry_profile, manifest):
            raise ValueError('native per-rule tool identity mismatch')
        entry_registry = profile_sources.profile_modules(entry_profile)
        if entry_registry != recorded['module_registry'] or set(entry_registry) != set(recorded['module_sha256']):
            raise ValueError('native per-rule module registry mismatch')
        for logical, relative in entry_registry.items():
            if profile_config.sha256(profile_config.REPOSITORY / profile_sources.module_path(relative)) != recorded['module_sha256'][logical]:
                raise ValueError('native per-rule module identity mismatch')
            check_imports(profile_config.RUFF_DIR / relative, entry_registry)
        entry_settings = profile_sources.canonical_run_ryo_settings(entry_profile, entry_profile['defaults'], phases)
        if entry_settings != recorded['source_settings']:
            raise ValueError('native per-rule source settings mismatch')
        if recorded['candidate_file'] != f'rules/{rule}/query.ryoql' or recorded['cases_file'] != f'rules/{rule}/cases.toml':
            raise ValueError('native query/case path must preserve its exact authored variant')
        query = profile_sources.prepared_candidate_path(rule, recorded)
        cases_path = profile_sources.prepared_case_path(rule, recorded)
        if not query.is_file() or profile_config.sha256(query) != recorded['query_sha256']:
            raise ValueError(f'native query identity mismatch: {rule}')
        if not cases_path.is_file() or profile_config.sha256(cases_path) != recorded['cases_sha256']:
            raise ValueError(f'native case identity mismatch: {rule}')
        check_imports(query, entry_registry)
        container = tomllib.loads(cases_path.read_text(encoding='utf-8'))
        if set(container) != {'cases'}:
            raise ValueError('native case container has unreviewed settings')
        cases = container['cases']
        names = [case['name'] for case in cases]
        if not cases or len(names) != len(set(names)) or names != recorded['case_names'] or (len(cases) != recorded['case_count']):
            raise ValueError(f'native case inventory mismatch: {rule}')
        for case in cases:
            if set(case) - {'name', 'comparison', 'files', 'expected', 'context', 'reference-discrepancy', 'selected-rules'}:
                raise ValueError('native case has unreviewed execution settings')
            selection = case.get('selected-rules', [rule])
            if not isinstance(selection, list) or not selection or selection[0] != rule or len(selection) != len(set(selection)) or set(selection[1:]) - set(entry_settings['rule_applicability']):
                raise ValueError('native case selects an undeclared supporting rule')
            if (entry_settings['project_context_mode'] == 'isolated') != (case.get('context') is None):
                raise ValueError('native case context differs from its declared profile mode')
        effective = profile_sources.canonical_run_ryo_settings(entry_profile, [rule], phases)
        if effective != recorded['effective_settings']:
            raise ValueError(f'native selected execution settings mismatch: {rule}')
        target = {configuration: version for version, configuration in profile_config.PARSER_CONFIGURATIONS.items()}.get(effective['parser_configuration'])
        if target is None:
            raise ValueError('unsupported native parser configuration')
        profiles[name] = {**recorded, 'target_version': target, 'selection_contract': 'explicit-native-standalone'}
        declared_queries.add(recorded['candidate_file'])
        declared_rules.add(rule)
        case_count += len(cases)
    if not declared_queries.issubset(native_queries) or descriptor['query_count'] != len(declared_queries) or descriptor['query_count'] != len(profiles):
        raise ValueError('native declared query inventory is incomplete or inconsistent')
    if descriptor['case_count'] != case_count:
        raise ValueError('native total case inventory mismatch')
    return {**manifest, 'profiles': profiles, 'combined_profiles': {}, 'actual_proof_receipts': [], 'native_identity': {'execution_contract': descriptor['execution_contract'], 'historical_certification': False, 'source_profile_sha256': descriptor['source_profile_sha256'], 'source_settings': source_settings, 'query_count': len(profiles), 'case_count': case_count}}
