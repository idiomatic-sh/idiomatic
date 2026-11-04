"""Profile sources: linter-specific authoring and proof utilities."""
from __future__ import annotations
import json
import shutil
import tempfile
from pathlib import Path
import tomllib
from authoring import profile_config

def toml_value(value: object) -> str:
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, bool):
        return 'true' if value else 'false'
    if isinstance(value, int):
        return str(value)
    if isinstance(value, list):
        return '[' + ', '.join((toml_value(item) for item in value)) + ']'
    if isinstance(value, dict):
        entries = ', '.join((f'{json.dumps(str(key), ensure_ascii=False)} = {toml_value(item)}' for key, item in value.items()))
        return '{ ' + entries + ' }'
    raise TypeError(f'unsupported benchmark TOML value: {type(value).__name__}')

def write_cases(path: Path, cases: list[dict[str, object]]) -> None:
    """Preserve admitted case context; omission and explicitly empty declarations differ."""
    lines = []
    primary = {'name', 'comparison', 'files', 'expected'}
    for case in cases:
        lines.extend(['[[cases]]', f"name = {toml_value(case['name'])}", f"comparison = {toml_value(case.get('comparison', 'rule'))}", f"files = {toml_value(case['files'])}", f"expected = {toml_value(case['expected'])}"])
        lines.extend(f'{toml_value(key)} = {toml_value(value)}' for key, value in case.items() if key not in primary)
        lines.append('')
    path.write_text('\n'.join(lines), encoding='utf-8')


def prepared_profile_path(profile_name: str, recorded: dict[str, object]) -> Path:
    """Resolve a sealed profile TOML, allowing target-specific profile aliases."""
    value = recorded.get('profile_file', f'profiles/{profile_name}/benchmark.toml')
    if not isinstance(value, str):
        raise ValueError(f'prepared profile path must be a string for {profile_name}')
    path = Path(value)
    if path.is_absolute() or not path.parts or any((part in ('.', '..') for part in path.parts)):
        raise ValueError(f'unsafe prepared profile path for {profile_name}: {value}')
    source = profile_config.RUFF_DIR
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f'prepared profile path traverses a symlink for {profile_name}: {value}')
    resolved = source.resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(profile_config.RUFF_DIR.resolve()):
        raise ValueError(f'prepared profile path is outside Ruff profile for {profile_name}: {value}')
    return resolved

def module_path(value: object, source_root: Path | None=None) -> str:
    if not isinstance(value, str):
        raise ValueError('query module paths must be strings')
    path = Path(value)
    if path.is_absolute() or not path.parts or any((part in ('.', '..') for part in path.parts)):
        raise ValueError(f'unsafe query module path: {value}')
    root = source_root or profile_config.RUFF_DIR
    source = root
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f'query module path traverses a symlink: {value}')
    resolved = source.resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(root.resolve()):
        raise ValueError(f'query module is outside the Ruff profile: {value}')
    return resolved.relative_to(profile_config.REPOSITORY if source_root is None else root).as_posix()

def prepared_case_path(rule: str, recorded: dict[str, object]) -> Path:
    """Resolve a profile-specific frozen case snapshot, or its canonical rule case file."""
    value = recorded.get('cases_file', f'rules/{rule}/cases.toml')
    if not isinstance(value, str):
        raise ValueError(f'prepared case file path must be a string for {rule}')
    path = Path(value)
    if path.is_absolute() or not path.parts or any((part in ('.', '..') for part in path.parts)):
        raise ValueError(f'unsafe prepared case path for {rule}: {value}')
    source = profile_config.RUFF_DIR
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f'prepared case path traverses a symlink for {rule}: {value}')
    resolved = source.resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(profile_config.RUFF_DIR.resolve()):
        raise ValueError(f'prepared case path is outside the Ruff profile for {rule}: {value}')
    return resolved

def prepared_candidate_path(rule: str, recorded: dict[str, object]) -> Path:
    """Resolve a profile-local frozen query snapshot or the canonical candidate."""
    value = recorded.get('candidate_file', f'rules/{rule}/query.ryoql')
    if value in {f'rules/{rule}/candidate.ryoql', f'rules/{rule}/query.ryoql'} and (profile_config.RUFF_DIR / 'rules' / rule / 'query.ryoql').is_file():
        value = f'rules/{rule}/query.ryoql'
    if not isinstance(value, str):
        raise ValueError(f'prepared candidate file path must be a string for {rule}')
    path = Path(value)
    if path.is_absolute() or not path.parts or any((part in ('.', '..') for part in path.parts)):
        raise ValueError(f'unsafe prepared candidate path for {rule}: {value}')
    source = profile_config.RUFF_DIR
    for part in path.parts:
        source = source / part
        if source.is_symlink():
            raise ValueError(f'prepared candidate path traverses a symlink for {rule}: {value}')
    resolved = source.resolve(strict=True)
    if not resolved.is_file() or not resolved.is_relative_to(profile_config.RUFF_DIR.resolve()):
        raise ValueError(f'prepared candidate path is outside the Ruff profile for {rule}: {value}')
    return resolved

def profile_modules(profile_data: dict[str, object], source_root: Path | None=None) -> dict[str, str]:
    registry = profile_data.get('query-modules', {})
    if not isinstance(registry, dict):
        raise ValueError('query-modules must be a table')
    result = {}
    for logical_id, path in registry.items():
        if not isinstance(logical_id, str) or not logical_id.strip():
            raise ValueError('query module IDs must be nonblank')
        module_path(path, source_root)
        result[logical_id] = path
    return result

def write_staged_profile(source: Path, destination: Path, registry: dict[str, str], parser_configuration: str='python-target=3.10', rule_applicability: dict[str, str] | None=None, selected_rules: list[str] | None=None, module_root: Path | None=None) -> None:
    """Serialize the supported profile fields, replacing any existing module table."""
    parsed = tomllib.loads(source.read_text(encoding='utf-8'))
    if selected_rules is not None:
        if not selected_rules or len(set(selected_rules)) != len(selected_rules) or (not set(selected_rules) <= set(parsed['defaults'])):
            raise ValueError('explicit profile selection must be a nonempty unique subset')
        parsed['defaults'] = selected_rules
        for disposition in ('deferred', 'pending'):
            if disposition in parsed:
                parsed[disposition] = {reason: chosen for reason, rules in parsed[disposition].items() if (chosen := [rule for rule in rules if rule in selected_rules])}
    for disposition in ('deferred', 'pending'):
        if disposition in parsed:
            chosen = {reason: rules for reason, rules in parsed[disposition].items() if rules}
            if chosen:
                parsed[disposition] = chosen
            else:
                parsed.pop(disposition)
    current = profile_modules(parsed, module_root)
    registry = profile_modules({'query-modules': registry}, module_root)
    if any((key in registry and registry[key] != value for key, value in current.items())):
        raise ValueError('staged profile conflicts with its query module registry')
    if parser_configuration not in profile_config.PARSER_CONFIGURATIONS.values():
        raise ValueError(f'unsupported staged parser configuration: {parser_configuration}')
    merged = {**current, **registry}
    applicability = rule_applicability or {}
    if set(applicability.values()) - {'always', 'accepted-syntax'}:
        raise ValueError('staged rule applicability contains an unsupported value')
    parsed.pop('query-modules', None)
    parsed.pop('rule-applicability', None)
    parsed['parser-configuration'] = parser_configuration
    if merged:
        parsed['query-modules'] = merged
    if applicability:
        parsed['rule-applicability'] = applicability
    rendered = ''.join((f'{key} = {toml_value(value)}\n' for key, value in parsed.items()))
    destination.write_text(rendered, encoding='utf-8')

def assert_staged_profile_fixed_point(profile_root: Path) -> None:
    """Refuse authored certificate identities that admission would normalize away.

    This invokes the same serializer as `stage_binding`, with the author's
    profile root for module resolution, before any contestant can run.
    """
    profile = profile_root / 'benchmark.toml'
    parsed = tomllib.loads(profile.read_text(encoding='utf-8'))
    parser_target = parsed.get('parser-configuration')
    if parser_target not in profile_config.PARSER_CONFIGURATIONS.values():
        raise ValueError('authoring profile needs a pinned parser configuration')
    registry = parsed.get('query-modules', {})
    applicability = parsed.get('rule-applicability', {})
    with tempfile.TemporaryDirectory(prefix='ryo-authoring-profile-') as temporary:
        normalized = Path(temporary) / 'benchmark.toml'
        write_staged_profile(profile, normalized, registry, parser_target, applicability, module_root=profile_root)
        if profile.read_bytes() != normalized.read_bytes():
            raise ValueError(f'authoring profile differs from admission normalization; authored={profile_config.sha256(profile)} staged={profile_config.sha256(normalized)}')

def parser_configuration(target_version: object) -> str:
    """Map explicit versions available in the pinned reference and linked parser."""
    if not isinstance(target_version, str) or target_version not in profile_config.PARSER_CONFIGURATIONS:
        raise ValueError(f'unsupported prepared Python target version: {target_version}')
    return profile_config.PARSER_CONFIGURATIONS[target_version]

def copy_modules(registry: dict[str, str], destination: Path) -> None:
    for source_path in sorted(set(registry.values())):
        source = profile_config.RUFF_DIR / source_path
        target = destination / source_path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)

def combined_modules(profile_data: dict[str, object], selected_rules: list[str], manifest: dict[str, object]) -> dict[str, str]:
    registry = profile_modules(profile_data)
    sources = [tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text(encoding='utf-8'))]
    sources.extend((tomllib.loads((profile_config.PROFILE_DIR / rule / 'benchmark.toml').read_text(encoding='utf-8')) for rule in selected_rules if rule in manifest['profiles']))
    for source_data in sources:
        for logical_id, source_path in profile_modules(source_data).items():
            if logical_id in registry and registry[logical_id] != source_path:
                raise ValueError(f'combined profile conflicts with module source for {logical_id}')
            registry[logical_id] = source_path
    return registry

def profile_tool_matches(profile_data: dict[str, object], manifest: dict[str, object]) -> bool:
    return profile_data.get('tool') == 'ruff' and profile_data.get('version') == manifest['reference_version'] and (profile_data.get('sha256') == manifest['reference_sha256'])

def rule_phase(phases: dict[str, object], rule: str) -> dict[str, object]:
    """Resolve a registered definition without changing the shipped-default inventory."""
    defaults = phases['rules']
    supplemental = phases.get('registered_nondefault_rules', {})
    if set(defaults) & set(supplemental):
        raise ValueError('phase registration duplicates a shipped default')
    if rule in defaults:
        return defaults[rule]
    if rule in supplemental:
        return supplemental[rule]
    raise ValueError(f'no pinned execution phase for registered rule {rule}')

def canonical_run_ryo_settings(profile: dict[str, object], rules: list[str], phases: dict[str, object]) -> dict[str, object]:
    """Normalize a selected profile without silently ignoring new execution flags."""
    allowed = {'tool', 'version', 'sha256', 'defaults', 'parser-configuration', 'source-language', 'project-context-mode', 'file-observation', 'query-modules', 'rule-applicability', 'supporting-rules', 'rule-observations', 'rule-context', 'deferred'}
    if set(profile) - allowed:
        raise ValueError('unreviewed execution settings in canonical profile')
    if profile.get('tool') != 'ruff' or profile.get('version') != profile_config.PINNED_RUFF_VERSION:
        raise ValueError('canonical execution settings have a different tool pin')
    if set(rules) - set(profile['defaults']):
        raise ValueError('execution selection is not in the canonical profile')
    if phases.get('ruff_version') != profile_config.PINNED_RUFF_VERSION or phases.get('ruff_source_commit') != profile_config.PINNED_RUFF_SOURCE_COMMIT:
        raise ValueError('execution phase registry has a different source pin')
    applicability = {rule: rule_phase(phases, rule)['applicability'] for rule in [*rules, *profile.get('supporting-rules', [])]}
    if set(applicability.values()) - {'always', 'accepted-syntax'}:
        raise ValueError('unreviewed canonical execution phase')
    declared = profile.get('rule-applicability', {})
    if any((declared.get(rule, value) != value for rule, value in applicability.items())):
        raise ValueError('canonical profile conflicts with the pinned execution phase')
    context = profile.get('project-context-mode', 'isolated')
    if context not in {'isolated', 'listed-disk'}:
        raise ValueError('unreviewed canonical project context mode')
    selection = profile.get('source-language')
    if selection is not None and (not isinstance(selection, str) or not selection or selection.strip() != selection):
        raise ValueError('source language must be an exact nonblank provider name')
    extra = {'source_language': selection} if selection is not None else {}
    if profile.get('rule-context') is not None:
        extra['rule_context_declaration'] = profile['rule-context']
    if profile.get('rule-observations'):
        extra['rule_observations'] = profile['rule-observations']
    observation = profile.get('file-observation')
    if observation is not None:
        if observation != 'posix-executable':
            raise ValueError('unsupported file observation requirement')
        extra['file_observation'] = observation
    return {**extra, 'defaults': sorted(set(rules)), 'parser_configuration': profile.get('parser-configuration', 'python-target=3.10'), 'project_context_mode': context, 'rule_applicability': applicability, 'flags': {}}

def registered_query_path(rule: str) -> Path:
    """Resolve the same admitted canonical definition for every explicit cohort route."""
    return profile_config.REPOSITORY / module_path(f'rules/{rule}/query.ryoql')

def copy_supporting_queries(profile: Path, destination: Path) -> None:
    """Copy exact declared executable support definitions, never synthesize cases/default credit."""
    data = tomllib.loads(profile.read_text(encoding='utf-8'))
    for rule in data.get('supporting-rules', []):
        source = registered_query_path(rule)
        target = destination / 'rules' / rule / 'query.ryoql'
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
