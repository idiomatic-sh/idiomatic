"""Profile proofs: linter-specific authoring and proof utilities."""
from __future__ import annotations
import json
import re
from pathlib import Path
import tomllib
from authoring import profile_config
from authoring import profile_sources
from authoring import reference_discrepancies

def manifest_paths(manifest: dict[str, object], helper_path: str) -> set[str]:
    profiles = manifest['profiles']
    base_profile = profile_config.RUFF_DIR / 'benchmark.toml'
    paths = {helper_path, base_profile.relative_to(profile_config.REPOSITORY).as_posix()}
    paths.add(profile_config.NATIVE_DESCRIPTOR_FILE.relative_to(profile_config.REPOSITORY).as_posix())
    paths.add(profile_config.HISTORICAL_NATIVE_DESCRIPTOR_FILE.relative_to(profile_config.REPOSITORY).as_posix())
    paths.add(profile_config.HISTORICAL_NATIVE_PROFILE_FILE.relative_to(profile_config.REPOSITORY).as_posix())
    native = native_descriptor_json(profile_config.NATIVE_DESCRIPTOR_FILE.read_text(encoding='utf-8'))
    for profile in native['profiles']:
        paths.add(profile_sources.prepared_candidate_path(profile['rule_id'], profile).relative_to(profile_config.REPOSITORY).as_posix())
        paths.add(profile_sources.prepared_case_path(profile['rule_id'], profile).relative_to(profile_config.REPOSITORY).as_posix())
        native_source_profile = profile_sources.prepared_profile_path(profile['profile_name'], profile)
        paths.add(native_source_profile.relative_to(profile_config.REPOSITORY).as_posix())
        native_source_data = tomllib.loads(native_source_profile.read_text(encoding='utf-8'))
        paths.update((profile_sources.module_path(relative) for relative in profile_sources.profile_modules(native_source_data).values()))
        native_cases = tomllib.loads(profile_sources.prepared_case_path(profile['rule_id'], profile).read_text(encoding='utf-8'))['cases']
        paths.update((source.relative_to(profile_config.REPOSITORY).as_posix() for source, _digest in reference_discrepancies.declared_discrepancy_artifacts(native_source_profile, native_cases, source_root=profile_config.discrepancy_source_root(profile['rule_id'], native_source_profile)).values()))
    paths.add(profile_config.RULE_PHASES_FILE.relative_to(profile_config.REPOSITORY).as_posix())
    paths.add((profile_config.RUFF_DIR / 'results/standard-suppression-policy.json').relative_to(profile_config.REPOSITORY).as_posix())
    base_data = tomllib.loads(base_profile.read_text(encoding='utf-8'))
    paths.update((profile_sources.module_path(path) for path in profile_sources.profile_modules(base_data).values()))
    for profile_name, recorded in profiles.items():
        rule = recorded.get('rule_id', profile_name)
        profile = profile_sources.prepared_profile_path(profile_name, recorded)
        case_file = profile_sources.prepared_case_path(rule, recorded)
        candidate_file = profile_sources.prepared_candidate_path(rule, recorded)
        paths.update({profile.relative_to(profile_config.REPOSITORY).as_posix(), candidate_file.relative_to(profile_config.REPOSITORY).as_posix(), case_file.relative_to(profile_config.REPOSITORY).as_posix()})
        cases = tomllib.loads(case_file.read_text(encoding='utf-8'))['cases']
        paths.update((source.relative_to(profile_config.REPOSITORY).as_posix() for source, _digest in reference_discrepancies.declared_discrepancy_artifacts(profile, cases, source_root=profile_config.discrepancy_source_root(rule, profile)).values()))
        profile_data = tomllib.loads(profile.read_text(encoding='utf-8'))
        paths.update((profile_sources.module_path(path) for path in profile_sources.profile_modules(profile_data).values()))
        if 'oracle_file' in recorded:
            paths.add(recorded['oracle_file'])
        if 'reference_audit_file' in recorded:
            paths.add(recorded['reference_audit_file'])
    for descriptor in manifest.get('combined_profiles', {}).values():
        paths.update(descriptor['input_files'])
        combined_profile = profile_config.REPOSITORY / descriptor['profile_file']
        combined_data = tomllib.loads(combined_profile.read_text(encoding='utf-8'))
        paths.update((profile_sources.module_path(path) for path in profile_sources.profile_modules(combined_data).values()))
    for receipt_path in manifest.get('actual_proof_receipts', []):
        paths.add(receipt_path)
        receipt = json.loads((profile_config.REPOSITORY / receipt_path).read_text(encoding='utf-8'))
        paths.update(receipt.get('execution_artifacts', {}))
    for fixture in [profile_config.REPOSITORY / 'crates/ryo/crates/benchmark/tests/fixtures/reference-discrepancy-python-f821', profile_config.RUFF_DIR.parents[2] / 'tests/fixtures/ryoql-fragments/sparse-traversal-guards', profile_config.RUFF_DIR.parents[2] / 'tests/fixtures/ryoql-fragments/pie794-rooted', profile_config.RUFF_DIR.parents[2] / 'tests/fixtures/ryoql-fragments/sparse-descendant-consumers', profile_config.RUFF_DIR.parents[2] / 'tests/fixtures/ryoql-fragments/plr1708-rooted']:
        paths.update((path.relative_to(profile_config.REPOSITORY).as_posix() for path in fixture.rglob('*') if path.is_file()))
    paths.add('crates/ryo/crates/benchmark/tests/fixtures/reference-discrepancy-canonical.json')
    paths.update((path.relative_to(profile_config.REPOSITORY).as_posix() for path in (profile_config.RUFF_DIR / 'authoring').glob('*.py')))
    return paths

def verify_actual_proof_assets(manifest: dict[str, object], repository: Path=profile_config.REPOSITORY, *, historical_only: bool=False) -> None:
    """Admit v2 execution identities; an explicit historical audit cannot admit v1 READY."""
    for relative in manifest.get('actual_proof_receipts', []):
        receipt = json.loads((repository / relative).read_text(encoding='utf-8'))
        schema = receipt.get('schema')
        if schema not in {'actual-proof-assets/v1', 'actual-proof-assets/v2'}:
            raise ValueError('unsupported actual-proof asset receipt')
        for artifact, expected_hash in receipt.get('execution_artifacts', {}).items():
            path = repository / artifact
            if not path.is_file() or profile_config.sha256(path) != expected_hash:
                raise ValueError(f'actual-proof execution artifact mismatch: {artifact}')
        phases = None
        if schema == 'actual-proof-assets/v2':
            phase_asset = receipt.get('phase_registry', {})
            phase_path = profile_config.RULE_PHASES_FILE.relative_to(profile_config.REPOSITORY).as_posix()
            if phase_asset.get('canonical_path') != phase_path or phase_asset.get('proof_sha256') != profile_config.sha256(repository / phase_path):
                raise ValueError('READY actual-proof phase registry identity mismatch')
            phases = json.loads((repository / phase_path).read_text(encoding='utf-8'))
        for profile in receipt['profiles']:
            recorded = manifest.get('profiles', {}).get(profile['rule'])
            if not isinstance(recorded, dict):
                raise ValueError('READY actual-proof profile is not selected by the manifest')
            expected_profile = profile_sources.prepared_profile_path(profile['rule'], recorded).relative_to(profile_config.REPOSITORY).as_posix()
            expected_query = profile_sources.prepared_candidate_path(recorded.get('rule_id', profile['rule']), recorded).relative_to(profile_config.REPOSITORY).as_posix()
            if profile['profile_file'] != expected_profile or profile['query']['canonical_path'] != expected_query:
                raise ValueError('READY actual-proof canonical selection path mismatch')
            canonical = tomllib.loads((repository / profile['profile_file']).read_text(encoding='utf-8'))
            settings = {'defaults': canonical['defaults'], 'parser_configuration': canonical.get('parser-configuration', 'python-target=3.10'), 'project_context_mode': canonical.get('project-context-mode', 'isolated')}
            if schema == 'actual-proof-assets/v2':
                settings = profile_sources.canonical_run_ryo_settings(canonical, canonical['defaults'], phases)
            if settings != profile['settings'] or canonical.get('query-modules', {}) != profile['module_registry']:
                raise ValueError(f"READY actual-proof profile identity mismatch: {profile['rule']}")
            query = profile['query']
            modules = profile['modules']
            if set(modules) != set(profile['module_registry']):
                raise ValueError('READY actual-proof dependency closure is incomplete')
            for logical, asset in modules.items():
                expected_path = profile_config.RUFF_DIR.relative_to(profile_config.REPOSITORY) / profile['module_registry'][logical]
                if asset['canonical_path'] != expected_path.as_posix():
                    raise ValueError('READY actual-proof dependency path mismatch')
            for asset in [query, *modules.values()]:
                path = repository / asset['canonical_path']
                if not path.is_file() or profile_config.sha256(path) != asset['proof_sha256']:
                    raise ValueError(f"READY actual-proof asset mismatch: {asset['canonical_path']}")
        if schema == 'actual-proof-assets/v1' and (not historical_only):
            raise ValueError('historical v1 proof lacks explicit canonical phase/flags; current admission requires v2')

def verify_files(manifest: dict[str, object], helper_path: str) -> None:
    files = manifest['files']
    if manifest.get('target_version') != 'py310':
        raise ValueError('prepared Ruff target version must be py310')
    phase_registry = verify_rule_phase_registry(manifest)
    base_profile = tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8'))
    for disposition in ('deferred', 'pending'):
        if any((not rules for rules in base_profile.get(disposition, {}).values())):
            raise ValueError('promoted base profile contains an empty capability group')
    check_base_rule_applicability(tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8')), phase_registry)
    allowed_targets = set(profile_config.PARSER_CONFIGURATIONS)
    if any((profile.get('target_version', 'py310') not in allowed_targets for profile in manifest['profiles'].values())):
        raise ValueError('prepared single-rule profile has an unsupported target version')
    if any((profile.get('target_version', 'py310') not in allowed_targets for profile in manifest.get('combined_profiles', {}).values())):
        raise ValueError('prepared combined profile has an unsupported target version')
    if set(files) != manifest_paths(manifest, helper_path):
        raise ValueError('prepared input paths do not match the candidate acceptance cohorts')
    defaults = set(tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8'))['defaults'])
    profile_rules = {profile.get('rule_id', profile_name) for profile_name, profile in manifest['profiles'].items()}
    if profile_rules - defaults:
        raise ValueError('prepared profile contains a non-default rule')
    for descriptor in manifest.get('combined_profiles', {}).values():
        selected = descriptor['selected_rules']
        if len(selected) != len(set(selected)) or set(selected) - defaults:
            raise ValueError('combined profile contains a non-default rule')
        if descriptor.get('validation_status') not in {'active-current', 'archived-historical'}:
            raise ValueError('combined profile must declare active-current or archived-historical status')
    if files.get(helper_path) != profile_config.sha256(Path(profile_config.ENTRYPOINT)):
        raise ValueError('profile helper hash does not match prepared.json')
    for relative, expected in files.items():
        path = profile_config.REPOSITORY / relative
        actual = profile_config.sha256(path) if path.is_file() else None
        if actual != expected:
            raise ValueError(f'prepared input hash mismatch: {relative} expected={expected} actual={actual} manifest={profile_config.MANIFEST}')
    verify_actual_proof_assets(manifest)

def verify_rule_phase_registry(manifest: dict[str, object], registry: dict[str, object] | None=None) -> dict[str, object]:
    """Require a pinned, positive-dispatch phase for every current prepared rule."""
    if manifest.get('reference_version') != profile_config.PINNED_RUFF_VERSION:
        raise ValueError('rule phase registry is only valid for pinned Ruff 0.16.5')
    if registry is None:
        registry = json.loads(profile_config.RULE_PHASES_FILE.read_text(encoding='utf-8'))
    if registry.get('schema_version') != 1 or registry.get('ruff_version') != profile_config.PINNED_RUFF_VERSION or registry.get('ruff_source_commit') != profile_config.PINNED_RUFF_SOURCE_COMMIT or (not isinstance(registry.get('code_to_variant_registry'), dict)) or (not isinstance(registry.get('phase_registry'), dict)) or (manifest.get('rule_phases_file') != profile_config.RULE_PHASES_FILE.relative_to(profile_config.REPOSITORY).as_posix()):
        raise ValueError('pinned Ruff phase registry has an unsupported identity')
    expected_rules = {profile.get('rule_id', profile_name) for profile_name, profile in manifest['profiles'].items()}
    expected_rules.update((rule for descriptor in manifest.get('combined_profiles', {}).values() if descriptor.get('validation_status') == 'active-current' for rule in descriptor['selected_rules']))
    rules = registry.get('rules')
    shipped_defaults = registry.get('shipped_default_rules')
    supplemental = registry.get('registered_nondefault_rules', {})
    if not isinstance(supplemental, dict) or (isinstance(rules, dict) and set(rules) & set(supplemental)):
        raise ValueError('nondefault phase registrations must be distinct from shipped defaults')
    if not isinstance(shipped_defaults, list) or len(shipped_defaults) != len(set(shipped_defaults)) or (not isinstance(rules, dict)) or (set(rules) != set(shipped_defaults)) or (not expected_rules <= (set(rules) | set(supplemental))):
        missing = expected_rules - (set(rules or {}) | set(supplemental))
        extra = set(rules or {}) - set(shipped_defaults or [])
        raise ValueError(f'pinned Ruff phase coverage mismatch: missing={sorted(missing)} extra={sorted(extra)}')
    for rule, record in {**rules, **supplemental}.items():
        if not isinstance(record, dict):
            raise ValueError(f'invalid phase classification for {rule}')
        phases = record.get('phases')
        if not isinstance(phases, list) or not phases or any((not isinstance(phase, str) for phase in phases)) or (len(phases) != len(set(phases))) or set(phases) - profile_config.RUFF_PHASES:
            raise ValueError(f'unknown or empty Ruff phase classification for {rule}')
        evidence = record.get('evidence')
        if not isinstance(evidence, list) or len(evidence) != 2 or any((not isinstance(item, dict) or item.get('kind') not in {'code-to-variant', 'lint-source'} or (not isinstance(item.get('line'), int)) or (item['line'] <= 0) or (not isinstance(item.get('sha256'), str)) or (len(item['sha256']) != 64) or any((char not in '0123456789abcdef' for char in item['sha256'])) or (item.get('path') != (registry.get('code_to_variant_registry', {}).get('path') if item.get('kind') == 'code-to-variant' else registry.get('phase_registry', {}).get('path'))) for item in evidence)) or (not isinstance(record.get('registry_variant'), str)) or (not isinstance(record.get('registry_line'), int)) or (record['registry_line'] <= 0):
            raise ValueError(f'phase classification for {rule} lacks pinned dispatch evidence')
        evidence_by_kind = {item['kind']: item for item in evidence}
        code_registry = registry['code_to_variant_registry']
        phase_registry = registry['phase_registry']
        if set(evidence_by_kind) != {'code-to-variant', 'lint-source'} or evidence_by_kind['code-to-variant']['sha256'] != code_registry.get('sha256') or evidence_by_kind['code-to-variant']['line'] != record['registry_line'] or (evidence_by_kind['lint-source']['sha256'] != phase_registry.get('sha256')) or (not isinstance(phase_registry.get('ast_fallback_line'), int)):
            raise ValueError(f'phase evidence does not match the pinned registry for {rule}')
        phase_evidence = evidence_by_kind['lint-source']
        resolution = phase_evidence.get('resolution')
        lint_sources = registry.get('lint_sources', {})
        if phases == ['ast']:
            if resolution != 'Ast wildcard fallback' or phase_evidence['line'] != phase_registry['ast_fallback_line']:
                raise ValueError(f'AST classification lacks the positive pinned fallback for {rule}')
        elif len(phases) != 1 or lint_sources.get(resolution) != phases[0] or phase_evidence['line'] == phase_registry['ast_fallback_line']:
            if not record.get('mixed_policy'):
                raise ValueError(f'non-AST or mixed phase lacks explicit registry evidence for {rule}')
        applicability = record.get('applicability')
        if set(phases) <= {'ast', 'imports'}:
            expected_applicability = 'accepted-syntax'
        elif set(phases) <= profile_config.RUFF_PHASES - {'ast', 'imports'}:
            expected_applicability = 'always'
        else:
            if applicability not in {'always', 'accepted-syntax'} or not record.get('mixed_policy'):
                raise ValueError(f'mixed Ruff phase policy is not explicitly reviewed for {rule}')
            expected_applicability = applicability
        if applicability != expected_applicability:
            raise ValueError(f'Ruff phase/applicability mismatch for {rule}: {phases} -> {applicability}')
    return registry

def verify_active_combined_cohort(manifest: dict[str, object]) -> None:
    """Require the default acceptance run to include one current full-report cohort."""
    active = [(name, descriptor) for name, descriptor in manifest.get('combined_profiles', {}).items() if descriptor.get('validation_status') == 'active-current']
    if len(active) != 1:
        raise ValueError(f'default acceptance requires exactly one active combined cohort; found {len(active)}')
    name, descriptor = active[0]
    if descriptor.get('expectations_mode') != 'complete-full-report':
        raise ValueError(f'active combined cohort {name} is not a complete full-report profile')

def check_base_rule_applicability(profile: dict[str, object], registry: dict[str, object]) -> None:
    """Keep all shipped defaults and their exact tagged phases in the native profile."""
    defaults = profile.get('defaults')
    expected_ids = registry['shipped_default_rules']
    if not isinstance(defaults, list) or len(defaults) != 413 or len(defaults) != len(set(defaults)) or (set(defaults) != set(expected_ids)) or (profile.get('rule-applicability') != {rule: registry['rules'][rule]['applicability'] for rule in expected_ids}):
        raise ValueError('base profile changed shipped defaults or their tagged phases')

def selected_rule_applicability(manifest: dict[str, object], selected_rules: list[str]) -> dict[str, str]:
    """Project the pinned phase registry onto this exact selected rule inventory."""
    registry = verify_rule_phase_registry(manifest)
    return {rule: registry['rules'][rule]['applicability'] for rule in selected_rules}

def check_staged_applicability(profile: Path, selected_rules: list[str], manifest: dict[str, object]) -> None:
    """Refuse a staged profile whose typed policy differs from its sealed selection."""
    data = tomllib.loads(profile.read_text(encoding='utf-8'))
    actual = data.get('rule-applicability')
    expected = selected_rule_applicability(manifest, [*selected_rules, *data.get('supporting-rules', [])])
    if actual != expected:
        raise ValueError('staged rule applicability differs from the pinned phase registry')

def check_canonical_standard_suppression(profile: Path, query_sources: dict[str, Path]) -> None:
    """Apply migration policy to canonical sources; retain historical snapshots."""
    policy = json.loads((profile_config.RUFF_DIR / 'results/standard-suppression-policy.json').read_text(encoding='utf-8'))
    selected = [rule for rule, source in query_sources.items() if rule in policy['canonical_queries'] and source == profile_config.RUFF_DIR / policy['canonical_queries'][rule]]
    check_standard_suppression(profile, selected)

def check_standard_suppression(profile: Path, selected_rules: list[str], policies: dict[str, str] | None=None) -> None:
    """Guard the explicitly migrated source forms, not arbitrary query semantics."""
    if policies is None:
        policy = json.loads((profile_config.RUFF_DIR / 'results/standard-suppression-policy.json').read_text(encoding='utf-8'))
        if policy.get('schema') != 'standard-suppression-policy/v1':
            raise ValueError('unsupported standard suppression policy')
        policies = policy['rules']
    scoped = set(selected_rules) & policies.keys()
    if not scoped:
        return
    registry = profile_sources.profile_modules(tomllib.loads(profile.read_text(encoding='utf-8')))
    variants = {'standard-module': 'suppress', 'unmapped-module': 'suppress-unmapped', 'additional-lines-module': 'suppress-with-additional-lines', 'explicit-edges-module': 'suppression-edges'}
    for rule in sorted(scoped):
        if policies[rule] not in variants:
            raise ValueError(f'unreviewed suppression policy for {rule}')
        module = variants[policies[rule]]
        logical = 'python/ruff/' + module
        if registry.get(logical) != 'fragments/' + module + '.ryoql':
            raise ValueError(f'{rule} requires its declared suppression module source')
        pending_modules = [logical]
        checked_modules = set()
        while pending_modules:
            dependency = pending_modules.pop()
            if dependency in checked_modules:
                continue
            checked_modules.add(dependency)
            if dependency not in registry:
                raise ValueError(f'{rule} has an undeclared suppression dependency')
            staged = profile.parent / registry[dependency]
            if not staged.is_file():
                raise ValueError(f'{rule} suppression dependency was not staged')
            pending_modules.extend(re.findall('^import\\s+\\w+\\s+from\\s+"([^"]+)"\\s*;', staged.read_text(encoding='utf-8'), re.MULTILINE))
        source = (profile.parent / 'rules' / rule / 'query.ryoql').read_text(encoding='utf-8')
        imports = re.findall('^import\\s+(\\w+)\\s+from\\s+"' + re.escape(logical) + '"\\s*;', source, re.MULTILINE)
        if len(imports) != 1:
            raise ValueError(f'{rule} requires one declared suppression-family import')
        if re.search('^let\\s+(?:mapping_edges|supp_range_comments)\\s*=', source, re.MULTILINE):
            raise ValueError(f'{rule} contains a copied suppression implementation')
        bindings = dict(re.findall('^let\\s+(\\w+)\\s*=\\s*(.*);\\s*$', source, re.MULTILINE))
        def depends_on_family(roots: list[str]) -> bool:
            pending = list(roots)
            visited = set()
            while pending:
                name = pending.pop()
                if name in visited or name not in bindings:
                    continue
                visited.add(name)
                expression = re.sub('"(?:\\\\.|[^"\\\\])*"', '""', bindings[name])
                if re.search('\\b' + re.escape(imports[0]) + '\\s*\\(', expression):
                    return True
                pending.extend(set(re.findall('\\b\\w+\\b', expression)) & bindings.keys())
            return False
        emitted = re.findall(r'^emit\s+(\w+)\s+subject\b', source, re.MULTILINE)
        if not depends_on_family(emitted):
            raise ValueError(f'{rule} does not emit a result depending on declared suppression')
        if policies[rule] == 'explicit-edges-module':
            observed = re.findall(r'^observe\s+(\w+)\s+as\s+\w+\s*;', source, re.MULTILINE)
            if not observed or not all(depends_on_family([root]) for root in observed):
                raise ValueError(f'{rule} does not publish named outputs depending on declared suppression matches')

def native_descriptor_json(text: str) -> dict[str, object]:
    """An explicit execution contract must not lose duplicate JSON declarations."""

    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f'duplicate native descriptor key: {key}')
            result[key] = value
        return result
    result = json.loads(text, object_pairs_hook=unique)
    if not isinstance(result, dict):
        raise ValueError('native descriptor must be an object')
    return result


def verify_case_inventory_extension(previous: list[dict[str, object]], current: list[dict[str, object]], explicit_renames: dict[str, str]) -> None:
    """Require canonical case additions to preserve every prior case byte-for-byte.

    A case identity can move only through a sealed old-name -> new-name migration;
    after changing that one field, source, effective comparison settings, and expected
    rows must match. An explicit ``comparison = "rule"`` equals the old default.
    """
    previous_by_name = {case.get('name'): case for case in previous}
    current_by_name = {case.get('name'): case for case in current}
    if len(previous_by_name) != len(previous) or len(current_by_name) != len(current) or any((not isinstance(name, str) or not name for name in previous_by_name)) or any((not isinstance(name, str) or not name for name in current_by_name)):
        raise ValueError('case inventory contains a blank or duplicate identity')
    if set(explicit_renames) - set(previous_by_name):
        raise ValueError('case migration names an absent prior identity')
    targets = list(explicit_renames.values())
    if len(targets) != len(set(targets)):
        raise ValueError('case migration has colliding target identities')
    for old_name, old_case in previous_by_name.items():
        new_name = explicit_renames.get(old_name, old_name)
        current_case = current_by_name.get(new_name)
        if current_case is None:
            raise ValueError(f'canonical case was removed without migration: {old_name}')
        expected_case = dict(old_case)
        expected_case['name'] = new_name
        actual_case = dict(current_case)
        if expected_case.get('comparison', 'rule') == actual_case.get('comparison', 'rule') == 'rule':
            expected_case.pop('comparison', None)
            actual_case.pop('comparison', None)
        if actual_case != expected_case:
            raise ValueError(f'canonical case source/settings/expectations changed: {old_name}')
    if set(explicit_renames.values()) - set(current_by_name):
        raise ValueError('case migration target is absent from the current inventory')
