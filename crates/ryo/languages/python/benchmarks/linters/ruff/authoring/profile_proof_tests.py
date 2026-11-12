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
    """The descriptor generated from the live tree is admitted, and every planted drift is refused."""
    original_run = subprocess.run

    def no_process(*args, **kwargs):
        raise AssertionError('native descriptor tests must not build or execute contestants')
    subprocess.run = no_process
    try:
        base = json.loads(profile_config.MANIFEST.read_text(encoding='utf-8'))
        members = profile_sources.native_members()
        # T1: the generated descriptor admits the live tree.
        descriptor = native_standalone.native_descriptor(base)
        resolved = native_standalone.native_standalone_manifest(base, descriptor)
        live_cases = sum(len(tomllib.loads((profile_config.RUFF_DIR / 'rules' / rule / 'cases.toml').read_text(encoding='utf-8'))['cases']) for rule in members)
        assert descriptor['query_count'] == len(resolved['profiles']) == len(members)
        assert descriptor['case_count'] == live_cases == sum(p['case_count'] for p in resolved['profiles'].values())
        assert all(entry['profile_file'] == f"profiles/{entry['rule_id']}/benchmark.toml" for entry in descriptor['profiles'])
        assert resolved['combined_profiles'] == {} and (not resolved['native_identity']['historical_certification'])

        # T2: planted drift is refused, each with its own message.
        def changed(edit):
            value = json.loads(json.dumps(descriptor))
            edit(value)
            return value

        def extra_member(value):
            extra = json.loads(json.dumps(value['profiles'][0]))
            extra.update(rule_id='N999', profile_name='native-N999')
            value['profiles'].append(extra)
        module = next(iter(descriptor['module_sha256']))
        mutations = [
            ('omission', lambda d: d['profiles'].pop(), 'native declared membership differs from the native evidence'),
            ('extra-member', extra_member, 'native declared membership differs from the native evidence'),
            ('duplicate', lambda d: d['profiles'].append(d['profiles'][0]), 'duplicate or noncanonical native profile identity'),
            ('root-binding', lambda d: d['profiles'][0].update(profile_file='benchmark.toml'), "native entry must bind its rule's own profile"),
            ('entry-profile-substitution', lambda d: d['profiles'][0].update(profile_file='profiles/UP044/benchmark.toml'), "native entry must bind its rule's own profile"),
            ('empty-selection', lambda d: d['profiles'][0].update(selected_rules=[]), 'native singleton selection must be explicitly declared'),
            ('wrong-selection', lambda d: d['profiles'][0].update(selected_rules=['PYI010']), 'native singleton selection must be explicitly declared'),
            ('unknown-rule', lambda d: d['profiles'][0].update(rule_id='NOT_A_PINNED_RULE'), 'native declared membership differs from the native evidence'),
            ('wrong-query-hash', lambda d: d['profiles'][0].update(query_sha256='0' * 64), 'native query identity mismatch: ASYNC115'),
            ('substitute-candidate', lambda d: d['profiles'][0].update(candidate_file='rules/ASYNC115/candidate.ryoql'), 'native query/case path must preserve its exact authored variant'),
            ('wrong-case-hash', lambda d: d['profiles'][0].update(cases_sha256='0' * 64), 'native case identity mismatch: ASYNC115'),
            ('case-omission', lambda d: d['profiles'][0]['case_names'].pop(), 'native case inventory mismatch: ASYNC115'),
            ('case-total', lambda d: d.update(case_count=d['case_count'] - 1), 'native total case inventory mismatch'),
            ('count-schema', lambda d: d.update(query_count=float(d['query_count'])), 'native inventory counts must be positive integers'),
            ('module-schema', lambda d: d.update(module_sha256=[]), 'native module declarations must be objects'),
            ('schema', lambda d: d.update(schema='native-standalone-cases/unknown'), 'unsupported native descriptor schema'),
            ('unknown-field', lambda d: d.update(guessed_execution=True), 'unsupported native descriptor schema'),
            ('entry-schema', lambda d: d['profiles'][0].update(unknown=True), 'unsupported native profile declaration schema'),
            ('invented-history', lambda d: d.update(historical_certification=True), 'native selection is a new explicit contract, never historical certification'),
            ('source-profile', lambda d: d.update(source_profile_sha256='0' * 64), 'native source profile identity mismatch'),
            ('context', lambda d: d['source_settings'].update(project_context_mode='isolated' if d['source_settings']['project_context_mode'] != 'isolated' else 'listed-disk'), 'native source execution settings mismatch'),
            ('parser-settings', lambda d: d['profiles'][0]['effective_settings'].update(parser_configuration='python-target=3.11'), 'native selected execution settings mismatch: ASYNC115'),
            ('unreviewed-flags', lambda d: d['profiles'][0]['effective_settings'].update(flags={'ignore': True}), 'native selected execution settings mismatch: ASYNC115'),
            ('phase', lambda d: d['profiles'][0]['effective_settings'].update(rule_applicability={}), 'native selected execution settings mismatch: ASYNC115'),
            ('phase-registry', lambda d: d.update(phase_registry_sha256='0' * 64), 'native pinned phase registry identity mismatch'),
            ('module', lambda d: d['module_sha256'].update({module: '0' * 64}), f'native module identity mismatch: {module}'),
            ('module-omission', lambda d: d['module_registry'].pop(next(iter(d['module_registry']))), 'native declared module registry mismatch'),
            ('entry-profile-hash', lambda d: d['profiles'][0].update(profile_sha256='0' * 64), 'native per-rule source profile identity mismatch'),
            ('entry-source-target', lambda d: d['profiles'][0]['source_settings'].update(parser_configuration='python-target=3.11'), 'native per-rule source settings mismatch'),
            ('entry-source-phase', lambda d: d['profiles'][0]['source_settings'].update(rule_applicability={}), 'native per-rule source settings mismatch'),
            ('entry-module-hash', lambda d: d['profiles'][0]['module_sha256'].update({next(iter(d['profiles'][0]['module_sha256'])): '0' * 64}), 'native per-rule module identity mismatch'),
            ('entry-module-omission', lambda d: d['profiles'][0]['module_registry'].pop(next(iter(d['profiles'][0]['module_registry']))), 'native per-rule module registry mismatch'),
            ('entry-module-schema', lambda d: d['profiles'][0].update(module_sha256=None), 'native per-rule settings/module declarations must be objects'),
        ]
        refusals = {}
        for name, edit, _message in mutations:
            try:
                native_standalone.native_standalone_manifest(base, changed(edit))
            except ValueError as error:
                refusals[name] = str(error)
            else:
                refusals[name] = None

        def planted_settings(member, edit):
            """Generate and validate under a settings source that drifted for one member."""
            canonical = profile_sources.canonical_run_ryo_settings

            def drifted(profile, rules, phases):
                settings = canonical(profile, rules, phases)
                return edit(settings) if profile.get('defaults') == [member] else settings
            profile_sources.canonical_run_ryo_settings = drifted
            try:
                native_standalone.native_standalone_manifest(base, native_standalone.native_descriptor(base))
            except ValueError as error:
                return str(error)
            finally:
                profile_sources.canonical_run_ryo_settings = canonical
            return None
        isolated = next(rule for rule in members if not any('context' in case for case in tomllib.loads((profile_config.RUFF_DIR / 'rules' / rule / 'cases.toml').read_text(encoding='utf-8'))['cases']))
        refusals['member-context'] = planted_settings(isolated, lambda settings: {**settings, 'project_context_mode': 'listed-disk'})
        mutations.append(('member-context', None, 'native case context differs from its declared profile mode'))
        refusals['undeclared-support'] = planted_settings('C419', lambda settings: {**settings, 'rule_applicability': {rule: phase for rule, phase in settings['rule_applicability'].items() if rule != 'RUF101'}})
        mutations.append(('undeclared-support', None, 'native case selects an undeclared supporting rule'))
        wrong = {name: refusals[name] for name, _edit, message in mutations if refusals[name] != message}
        assert not wrong, f'native descriptor refusals differ: {wrong}'

        # T3: the four staging controls keep their declared settings and exact assets.
        staging_names = ['native-PYI035', 'native-ASYNC115', 'native-UP044', 'native-PLE1142']
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
                    assert profile_sources.profile_modules(effective) == recorded['module_registry']
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
        print(f"native standalone guards passed: {descriptor['query_count']} generated profiles, {descriptor['case_count']} cases, {len(mutations)} refusals, four staging controls; no contestants executed")
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
