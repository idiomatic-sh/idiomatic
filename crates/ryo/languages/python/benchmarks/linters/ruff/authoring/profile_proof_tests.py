"""Profile proof tests: linter-specific authoring and proof utilities."""
from __future__ import annotations
import contextlib
import hashlib
import json
import subprocess
import tarfile
import tempfile
from pathlib import Path
import tomllib
from authoring import profile_config
from authoring import profile_proofs
from authoring import native_standalone
from authoring import profile_sources
from authoring import profile_staging
from authoring import profile_worker

def output_directory_self_test() -> None:
    """A second replay cannot replace the first run's raw evidence."""
    with tempfile.TemporaryDirectory(prefix='ryo-exclusive-replay-') as temporary:
        destination = Path(temporary) / 'run'
        profile_worker.create_new_output_directory(destination)
        sentinel = destination / 'cases.jsonl'
        sentinel.write_bytes(b'first source-bound raw receipt\n')
        try:
            profile_worker.create_new_output_directory(destination)
        except FileExistsError:
            pass
        else:
            raise AssertionError('an existing replay output was reused')
        assert sentinel.read_bytes() == b'first source-bound raw receipt\n'
        profile = destination / 'profile'
        profile.mkdir()
        (profile / 'benchmark.toml').write_text('defaults = ["F401"]\n')
        calls = []

        def fake_runner(command, **options):
            receipt = Path(next(arg.split('=', 1)[1] for arg in command if arg.startswith('--case-receipts=')))
            assert receipt.parent.is_dir()
            assert not receipt.exists()
            calls.append(command)
            return subprocess.CompletedProcess(command, 1, b'first stdout\n', b'first stderr\n')
        command = ['fake-worker', 'verify-cases', 'fake-ruff', str(profile)]
        assert profile_worker.verify_profile('mock', profile, command, destination, {}, runner=fake_runner) == 1
        record = destination / 'profiles' / 'mock'
        assert len(calls) == 1
        assert (record / 'stdout.bin').read_bytes() == b'first stdout\n'
        sentinel = record / 'cases.jsonl'
        sentinel.write_bytes(b'first raw cases\n')
        before = {name: (record / name).read_bytes() for name in ('stdout.bin', 'stderr.bin', 'result.json', 'cases.jsonl')}
        try:
            profile_worker.verify_profile('mock', profile, command, destination, {}, runner=fake_runner)
        except FileExistsError:
            pass
        else:
            raise AssertionError('a profile verifier reused an existing raw receipt')
        assert len(calls) == 1
        assert before == {name: (record / name).read_bytes() for name in before}

def native_standalone_self_test() -> None:
    """Test current registration/staging guards; frozen native evidence remains immutable."""
    original_run = subprocess.run

    def no_process(*args, **kwargs):
        raise AssertionError('native descriptor tests must not build or execute contestants')
    subprocess.run = no_process
    try:
        base = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
        frozen_bytes = profile_config.NATIVE_DESCRIPTOR_FILE.read_bytes()
        frozen = profile_proofs.native_descriptor_json(frozen_bytes.decode())
        # This is a test registration, not a rewrite or replay of native151 evidence.
        descriptor = json.loads(json.dumps(frozen))
        staging_names = ['native-PYI035', 'native-ASYNC115', 'native-UP044', 'native-PLE1142']
        descriptor['profiles'] = [entry for entry in descriptor['profiles']
                                  if entry['profile_name'] in staging_names]
        # Current singleton registrations use their actual declared per-rule contexts.
        # The canonical corpus profile may require ListedDisk while standalone cases are isolated.
        for entry in descriptor['profiles']:
            entry['profile_file'] = f"profiles/{entry['rule_id']}/benchmark.toml"
        phases = profile_proofs.verify_rule_phase_registry(base)
        for record in [descriptor, *descriptor['profiles']]:
            entry = record is not descriptor
            profile_file = record['profile_file'] if entry else record['source_profile_file']
            profile_path = profile_sources.prepared_profile_path('test-current', {'profile_file': profile_file})
            profile = tomllib.loads(profile_path.read_text())
            settings = profile_sources.canonical_run_ryo_settings(profile, profile['defaults'], phases)
            if entry:
                record['effective_settings'] = profile_sources.canonical_run_ryo_settings(profile, record['selected_rules'], phases)
                query = profile_sources.prepared_candidate_path(record['rule_id'], record)
                cases_path = profile_sources.prepared_case_path(record['rule_id'], record)
                cases = tomllib.loads(cases_path.read_text())['cases']
                record.update(query_sha256=profile_config.sha256(query), cases_sha256=profile_config.sha256(cases_path),
                              case_names=[case['name'] for case in cases], case_count=len(cases))
            if not entry or profile_file != descriptor['source_profile_file']:
                registry = profile_sources.profile_modules(profile)
                record.update(source_settings=settings, module_registry=registry,
                              module_sha256={logical: profile_config.sha256(profile_config.RUFF_DIR / path)
                                             for logical, path in registry.items()})
                record['profile_sha256' if entry else 'source_profile_sha256'] = profile_config.sha256(profile_path)
        descriptor.update(query_count=len(descriptor['profiles']),
                          case_count=sum(entry['case_count'] for entry in descriptor['profiles']),
                          phase_registry_sha256=profile_config.sha256(profile_config.RULE_PHASES_FILE),
                          intent='Self-test current registration only; no contestant execution or historical certification.')
        resolved = native_standalone.native_standalone_manifest(base, descriptor)
        assert len(resolved['profiles']) == descriptor['query_count']
        assert sum((p['case_count'] for p in resolved['profiles'].values())) == descriptor['case_count']
        assert resolved['combined_profiles'] == {} and (not resolved['native_identity']['historical_certification'])

        def changed(edit):
            value = json.loads(json.dumps(descriptor))
            edit(value)
            return value
        # Exercise module validation only after the current registration is admitted.
        module = next(iter(descriptor['module_sha256']))
        wrong_module = changed(lambda value: value['module_sha256'].update({module: '0' * 64}))
        try:
            native_standalone.native_standalone_manifest(base, wrong_module)
        except ValueError as error:
            assert str(error) == f'native module identity mismatch: {module}', str(error)
        else:
            raise AssertionError('native descriptor accepted a changed module identity')
        mutations = [('omission', lambda d: d['profiles'].pop()), ('duplicate', lambda d: d['profiles'].append(d['profiles'][0])), ('empty-selection', lambda d: d['profiles'][0].update(selected_rules=[])), ('wrong-selection', lambda d: d['profiles'][0].update(selected_rules=['PYI010'])), ('unknown-rule', lambda d: d['profiles'][0].update(rule_id='NOT_A_PINNED_RULE')), ('wrong-query-hash', lambda d: d['profiles'][0].update(query_sha256='0' * 64)), ('substitute-candidate', lambda d: d['profiles'][0].update(candidate_file='rules/ASYNC115/candidate.ryoql')), ('wrong-case-hash', lambda d: d['profiles'][0].update(cases_sha256='0' * 64)), ('case-omission', lambda d: d['profiles'][0]['case_names'].pop()), ('case-total', lambda d: d.update(case_count=d['case_count'] - 1)), ('count-schema', lambda d: d.update(query_count=float(d['query_count']))), ('module-schema', lambda d: d.update(module_sha256=[])), ('schema', lambda d: d.update(schema='native-standalone-cases/unknown')), ('unknown-field', lambda d: d.update(guessed_execution=True)), ('entry-schema', lambda d: d['profiles'][0].update(unknown=True)), ('invented-history', lambda d: d.update(historical_certification=True)), ('source-profile', lambda d: d.update(source_profile_sha256='0' * 64)), ('context', lambda d: d['source_settings'].update(project_context_mode='isolated' if d['source_settings']['project_context_mode'] != 'isolated' else 'listed-disk')), ('parser-settings', lambda d: d['profiles'][0]['effective_settings'].update(parser_configuration='python-target=3.11')), ('unreviewed-flags', lambda d: d['profiles'][0]['effective_settings'].update(flags={'ignore': True})), ('phase', lambda d: d['profiles'][0]['effective_settings'].update(rule_applicability={})), ('phase-registry', lambda d: d.update(phase_registry_sha256='0' * 64)), ('module-omission', lambda d: d['module_registry'].pop(next(iter(d['module_registry'])))), ('module-hash', lambda d: d['module_sha256'].update({next(iter(d['module_sha256'])): '0' * 64}))]
        if descriptor['schema'] == 'native-standalone-cases/v2':
            custom_index = next((i for i, entry in enumerate(descriptor['profiles']) if entry['profile_file'] != descriptor['source_profile_file']))
            mutations.extend([('entry-profile-hash', lambda d: d['profiles'][custom_index].update(profile_sha256='0' * 64)), ('entry-source-target', lambda d: d['profiles'][custom_index]['source_settings'].update(parser_configuration='python-target=3.11')), ('entry-source-phase', lambda d: d['profiles'][custom_index]['source_settings'].update(rule_applicability={})), ('entry-module-hash', lambda d: d['profiles'][custom_index]['module_sha256'].update({next(iter(d['profiles'][custom_index]['module_sha256'])): '0' * 64})), ('entry-module-omission', lambda d: d['profiles'][custom_index]['module_registry'].pop(next(iter(d['profiles'][custom_index]['module_registry'])))), ('entry-module-schema', lambda d: d['profiles'][custom_index].update(module_sha256=None)), ('entry-profile-substitution', lambda d: d['profiles'][custom_index].update(profile_file='profiles/UP044/benchmark.toml'))])
            historical = profile_proofs.native_descriptor_json(profile_config.HISTORICAL_NATIVE_DESCRIPTOR_FILE.read_text(encoding='utf-8'))
            assert profile_config.sha256(profile_config.HISTORICAL_NATIVE_PROFILE_FILE) == historical['source_profile_sha256']
            current_by_rule = {entry['rule_id']: entry for entry in frozen['profiles']}
            for entry in historical['profiles']:
                current = current_by_rule[entry['rule_id']]
                for key in ('candidate_file', 'query_sha256', 'cases_file', 'cases_sha256', 'case_names', 'case_count'):
                    assert current[key] == entry[key], 'historical native source identity changed'
        for name, edit in mutations:
            try:
                native_standalone.native_standalone_manifest(base, changed(edit))
            except ValueError:
                pass
            else:
                raise AssertionError(f'native descriptor accepted {name}')
        try:
            profile_proofs.native_descriptor_json('{"schema":1,"schema":2}')
        except ValueError:
            pass
        else:
            raise AssertionError('duplicate descriptor keys were silently overwritten')
        source_profile = profile_sources.prepared_profile_path('native', {'profile_file': descriptor['source_profile_file']})
        source_sha = profile_config.sha256(source_profile)
        with tempfile.TemporaryDirectory(prefix='native-standalone-guards-') as temporary:
            with contextlib.ExitStack() as stack:
                staged = profile_staging.stage_binding(resolved, staging_names, stack, Path(temporary))
                for name, root in staged:
                    recorded = resolved['profiles'][name]
                    rule = recorded['rule_id']
                    effective = tomllib.loads((root / 'benchmark.toml').read_text())
                    assert effective['defaults'] == [rule]
                    assert set(effective['rule-applicability']) == {rule}
                    assert profile_sources.profile_modules(effective) == recorded.get('module_registry', descriptor['module_registry'])
                    assert profile_sources.canonical_run_ryo_settings(effective, [rule], profile_proofs.verify_rule_phase_registry(base)) == recorded['effective_settings']
                    assert sorted((p.name for p in (root / 'rules').iterdir())) == [rule]
                    assert profile_config.sha256(root / 'rules' / rule / 'query.ryoql') == recorded['query_sha256']
                    assert profile_config.sha256(root / 'rules' / rule / 'cases.toml') == recorded['cases_sha256']
                    assert not effective.get('deferred') and (not effective.get('pending'))
                    if rule == 'UP044':
                        assert effective['parser-configuration'] == 'python-target=3.11', 'declared target was coerced to the base target'
                    if rule == 'PLE1142':
                        assert (root / 'evidence/cpython-3.10.19.json').is_file(), 'declared native evidence was not staged'
                    if rule == 'PYI035':
                        cases = tomllib.loads((root / 'rules' / rule / 'cases.toml').read_text())['cases']
                        control = next((c for c in cases if c['name'] == 'function-scope-special-2-pyi-False'))
                        assert control['comparison'] == 'full' and control['expected'] == []
                        assert not (root / 'rules' / 'PYI010').exists()
                    if rule == 'ASYNC115':
                        archived = profile_config.RUFF_DIR / 'results/canonical-query-sources-2026-11-02/divergent-candidates.tar.gz'
                        with tarfile.open(archived) as archive:
                            alternate = archive.extractfile('rules/ASYNC115/candidate.ryoql')
                            assert alternate is not None
                            assert alternate.read() != (root / 'rules' / rule / 'query.ryoql').read_bytes()
                assert profile_config.sha256(source_profile) == source_sha, 'source profile must never be rewritten'
        assert profile_config.NATIVE_DESCRIPTOR_FILE.read_bytes() == frozen_bytes, 'frozen native descriptor must not be rewritten'
        print(f"native standalone guards passed: {descriptor['query_count']} current registration fixtures, {descriptor['case_count']} cases, {len(mutations) + 2} refusals, four staging controls; frozen evidence unchanged, no contestants executed")
    finally:
        subprocess.run = original_run

def standalone_registry_self_test() -> None:
    """A standalone proof cannot silently acquire the global module registry."""
    manifest = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
    rule = 'PLW0406'
    canonical = profile_sources.profile_modules(tomllib.loads(profile_sources.prepared_profile_path(rule, manifest['profiles'][rule]).read_text()))
    global_registry = profile_sources.profile_modules(tomllib.loads((profile_config.RUFF_DIR / 'benchmark.toml').read_text()))
    assert set(global_registry) - set(canonical), 'fixture needs an unrelated global module'
    with tempfile.TemporaryDirectory(prefix='ryo-standalone-registry-') as directory:
        with contextlib.ExitStack() as stack:
            staged = profile_staging.stage_binding(manifest, [rule], stack, Path(directory))
            assert len(staged) == 1
            root = staged[0][1]
            actual = profile_sources.profile_modules(tomllib.loads((root / 'benchmark.toml').read_text()))
            assert actual == canonical, 'standalone registry was silently expanded'
            assert {path.relative_to(root).as_posix() for path in (root / 'fragments').rglob('*') if path.is_file()} == set(canonical.values()), 'undeclared module bytes were staged'

def actual_proof_asset_guard_self_test() -> None:
    """A valid commit/source hash cannot bless different executed query bytes."""
    with tempfile.TemporaryDirectory(prefix='ryo-actual-proof-guard-') as temporary:
        root = Path(temporary)
        prefix = profile_config.RUFF_DIR.relative_to(profile_config.REPOSITORY)
        profile_path = prefix / 'profiles/F401/benchmark.toml'
        query_path = prefix / 'rules/F401/query.ryoql'
        (root / profile_path).parent.mkdir(parents=True, exist_ok=True)
        (root / query_path).parent.mkdir(parents=True, exist_ok=True)
        (root / profile_path).write_text('defaults = ["F401"]\n', encoding='utf-8')
        (root / query_path).write_text('valid source-commit bytes', encoding='utf-8')
        profile = {'rule': 'F401', 'profile_file': profile_path.as_posix(), 'settings': {'defaults': ['F401'], 'parser_configuration': 'python-target=3.10', 'project_context_mode': 'isolated'}, 'module_registry': {}, 'modules': {}, 'query': {'canonical_path': query_path.as_posix(), 'source_commit': '48acfc0321046a0a3d0c44b5d122dbe09eb51cc6', 'proof_sha256': hashlib.sha256(b'different actual executed proof bytes').hexdigest()}}
        (root / 'receipt.json').write_text(json.dumps({'schema': 'actual-proof-assets/v1', 'profiles': [profile]}), encoding='utf-8')
        manifest = {'profiles': {'F401': {'candidate_file': 'rules/F401/query.ryoql'}}, 'actual_proof_receipts': ['receipt.json'], 'files': {'query.ryoql': profile_config.sha256(root / query_path)}}
        try:
            profile_proofs.verify_actual_proof_assets(manifest, root)
        except ValueError as error:
            assert 'actual-proof asset mismatch' in str(error)
        else:
            raise AssertionError('source-commit metadata bypassed actual executed query identity')
        (root / query_path).write_bytes(b'different actual executed proof bytes')
        profile_proofs.verify_actual_proof_assets(manifest, root, historical_only=True)
        (root / profile_path).write_text('defaults = ["F401"]\n[query-modules]\n"python/ruff/suppress" = "fragments/suppress.ryoql"\n', encoding='utf-8')
        module = prefix / 'fragments/suppress.ryoql'
        (root / module).parent.mkdir(parents=True, exist_ok=True)
        (root / module).write_text('valid source-commit helper', encoding='utf-8')
        profile['module_registry'] = {'python/ruff/suppress': 'fragments/suppress.ryoql'}
        profile['modules'] = {'python/ruff/suppress': {'canonical_path': module.as_posix(), 'proof_sha256': hashlib.sha256(b'actual executed helper').hexdigest()}}
        (root / 'receipt.json').write_text(json.dumps({'schema': 'actual-proof-assets/v1', 'profiles': [profile]}), encoding='utf-8')
        try:
            profile_proofs.verify_actual_proof_assets(manifest, root)
        except ValueError as error:
            assert 'actual-proof asset mismatch' in str(error)
        else:
            raise AssertionError('dependency source metadata bypassed actual proof bytes')
        (root / module).write_bytes(b'actual executed helper')
        profile_proofs.verify_actual_proof_assets(manifest, root, historical_only=True)
        try:
            profile_proofs.verify_actual_proof_assets(manifest, root)
        except ValueError as error:
            assert 'historical v1 proof' in str(error)
        else:
            raise AssertionError('historical v1 proof was promoted to current admission')
        phase_path = profile_config.RULE_PHASES_FILE.relative_to(profile_config.REPOSITORY)
        (root / phase_path).parent.mkdir(parents=True, exist_ok=True)
        (root / phase_path).write_bytes(profile_config.RULE_PHASES_FILE.read_bytes())
        phases = json.loads((root / phase_path).read_text(encoding='utf-8'))
        (root / profile_path).write_text('tool = "ruff"\nversion = "0.16.5"\ndefaults = ["F401"]\n[query-modules]\n"python/ruff/suppress" = "fragments/suppress.ryoql"\n', encoding='utf-8')
        profile['settings'] = profile_sources.canonical_run_ryo_settings(tomllib.loads((root / profile_path).read_text(encoding='utf-8')), ['F401'], phases)
        current = {'schema': 'actual-proof-assets/v2', 'profiles': [profile], 'phase_registry': {'canonical_path': phase_path.as_posix(), 'proof_sha256': profile_config.sha256(root / phase_path)}}
        (root / 'receipt.json').write_text(json.dumps(current), encoding='utf-8')
        profile_proofs.verify_actual_proof_assets(manifest, root)
        artifact = root / 'executed-case-records.jsonl'
        artifact.write_bytes(b'complete execution receipt')
        current['execution_artifacts'] = {artifact.name: profile_config.sha256(artifact)}
        (root / 'receipt.json').write_text(json.dumps(current), encoding='utf-8')
        profile_proofs.verify_actual_proof_assets(manifest, root)
        artifact.write_bytes(b'changed execution receipt')
        try:
            profile_proofs.verify_actual_proof_assets(manifest, root)
        except ValueError as error:
            assert 'execution artifact mismatch' in str(error)
        else:
            raise AssertionError('changed raw execution evidence was admitted')
        artifact.write_bytes(b'complete execution receipt')
        correct_settings = dict(profile['settings'])
        for changed in ({'rule_applicability': {'F401': 'always'}}, {'parser_configuration': 'python-target=3.11'}, {'project_context_mode': 'listed-disk'}, {'flags': {'preview': True}}, {'unreviewed_flag': True}):
            profile['settings'] = {**correct_settings, **changed}
            (root / 'receipt.json').write_text(json.dumps(current), encoding='utf-8')
            try:
                profile_proofs.verify_actual_proof_assets(manifest, root)
            except ValueError:
                continue
            raise AssertionError('v2 proof with noncanonical execution settings was accepted')
        profile['settings'] = correct_settings

def parser_target_configuration_self_test() -> None:
    """Both old and native annotation targets stage explicitly; no implicit/moving mode is accepted."""
    assert profile_sources.parser_configuration('py37') == 'python-target=3.7'
    assert profile_sources.parser_configuration('py314') == 'python-target=3.14'
    command = profile_worker.run_ryo_command('worker', {'parser_configuration': 'python-target=3.10'})
    assert command[-1] == 'python-target=3.10'
    for request in ({}, {'parser_configuration': 'python-target=latest'}):
        try:
            profile_worker.run_ryo_command('worker', request)
        except ValueError:
            continue
        raise AssertionError('direct worker accepted a missing or moving parser configuration')
    for value in [None, 'latest', 'py36', 'py316', 'py3.10', 'py310 ']:
        try:
            profile_sources.parser_configuration(value)
        except ValueError:
            continue
        raise AssertionError('unknown or missing target was accepted')
    with tempfile.TemporaryDirectory(prefix='ryo-target-staging-') as temporary:
        root = Path(temporary)
        source = root / 'source.toml'
        source.write_text('tool="ruff"\nversion="0.16.5"\nsha256=["pin"]\ndefaults=["UP045"]\n')
        destination = root / 'benchmark.toml'
        for configuration in profile_config.PARSER_CONFIGURATIONS.values():
            profile_sources.write_staged_profile(source, destination, {}, configuration, {'UP045': 'accepted-syntax'})
            staged = tomllib.loads(destination.read_text())
            assert staged['parser-configuration'] == configuration
            assert staged['rule-applicability'] == {'UP045': 'accepted-syntax'}
            assert staged['defaults'] == ['UP045']
        for configuration in [None, 'python-target=latest', 'python-target=3.6', 'python-target=3.16']:
            try:
                profile_sources.write_staged_profile(source, destination, {}, configuration, {'UP045': 'accepted-syntax'})
            except ValueError:
                continue
            raise AssertionError('unsupported staged target was accepted')

def authoring_profile_fixed_point_self_test() -> None:
    with tempfile.TemporaryDirectory(prefix='ryo-authoring-fixed-point-') as temporary:
        root = Path(temporary)
        profile = root / 'benchmark.toml'
        profile.write_text('tool = "ruff"\nversion = "0.16.5"\nsha256 = ["pin"]\ndefaults = ["F"]\nparser-configuration = "python-target=3.10"\n[rule-applicability]\nF = "accepted-syntax"\n', encoding='utf-8')
        normalized = root / 'normalized.toml'
        profile_sources.write_staged_profile(profile, normalized, {}, 'python-target=3.10', {'F': 'accepted-syntax'}, module_root=root)
        assert profile.read_bytes() != normalized.read_bytes()
        try:
            profile_sources.assert_staged_profile_fixed_point(root)
        except ValueError as error:
            assert 'admission normalization' in str(error)
        else:
            raise AssertionError('nonfixed authoring profile was accepted')
        cases = root / 'rules' / 'F' / 'cases.toml'
        cases.parent.mkdir(parents=True)
        cases.write_text('[[cases]]\nname = "one"\nfiles = { "case.py" = "x=1\\n" }\nexpected = []\nreference-discrepancy = {}\n', encoding='utf-8')
        output = root / 'output'
        output.mkdir()
        called = []
        status = profile_worker.verify_profile('F', root, ['worker', 'verify-cases', 'ruff', str(root)], output, {}, runner=lambda *_args, **_kwargs: called.append(True))
        receipt = json.loads((output / 'profiles' / 'F' / 'result.json').read_text())
        assert status != 0 and (not called) and ('normalization' in receipt['error'])
        profile.write_bytes(normalized.read_bytes())
        profile_sources.assert_staged_profile_fixed_point(root)

def profile_accounting_projection_self_test() -> None:
    with tempfile.TemporaryDirectory(prefix='profile-accounting-guards-') as temporary:
        source = Path(temporary) / 'source.toml'
        destination = Path(temporary) / 'staged.toml'
        source.write_text('defaults = ["A", "B"]\n[deferred]\npromoted = []\nremaining = ["B"]\n[pending]\nfinished = []\n')
        before = source.read_bytes()
        profile_sources.write_staged_profile(source, destination, {})
        staged = tomllib.loads(destination.read_text())
        assert staged['deferred'] == {'remaining': ['B']}
        assert 'pending' not in staged
        profile_sources.write_staged_profile(source, destination, {}, selected_rules=['A'])
        staged = tomllib.loads(destination.read_text())
        assert 'deferred' not in staged and 'pending' not in staged
        assert source.read_bytes() == before
