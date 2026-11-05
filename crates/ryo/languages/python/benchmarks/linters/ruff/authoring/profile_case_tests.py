"""Profile case tests: linter-specific authoring and proof utilities."""
from __future__ import annotations
import hashlib
import json
import os
import shutil
import tempfile
from pathlib import Path
import tomllib
from authoring import case_records
from authoring import profile_config
from authoring import profile_proofs
from authoring import profile_sources
from authoring import profile_worker
from authoring import reference_discrepancies

def project_inputs_identity_self_test() -> None:
    """Rust and Python hash the same explicit project input envelope."""
    inventory = {'completeness': 'complete-within-declared-scope', 'entries': [
        {'path': 'src/pkg', 'state': 'directory'},
        {'path': 'vendor/pkg.py', 'state': 'missing'},
    ]}
    assert case_records.project_inputs({}, {}) is None
    assert case_records.project_inputs_digest(None) is None
    inputs = case_records.project_inputs({'project-search-roots': ['src', 'vendor'], 'given-project-inventory': inventory}, {})
    assert case_records.project_inputs_digest(inputs) == 'ff9b78535b775babaadae518c198876aa1c27ef21d0abb75e04588c25638df7d'
    assert case_records.project_inputs({'project-search-roots': []}, {})['project_search_roots'] == []
    for invalid in ({'project-search-roots': ['../outside']}, {'given-project-inventory': {**inventory, 'entries': [{'path': '../outside', 'state': 'file'}]}}):
        try:
            case_records.project_inputs(invalid, {})
        except ValueError:
            continue
        raise AssertionError('unsafe project input authority was accepted')

def execution_metadata_self_test() -> None:
    """Version boundaries retain old diagnostics without inventing known execution or waiving fresh candidate guards."""
    row = {'observation_schema': 'benchmark-case/v2', 'execution_metadata': {'availability': 'complete'}, 'files': ['case.py'], 'skipped': [], 'coverage': {'selected_rule_file_pairs': 1, 'evaluated_rule_file_pairs': 1, 'skipped_rule_file_pairs': 0, 'unknown_or_unread_rule_file_pairs': 0}}
    assert case_records.record_execution_complete(row)
    assert case_records.record_execution_comparison(row, row) == 'exact'
    unavailable = {**row, 'execution_metadata': {'availability': 'unavailable', 'reason': 'findings-only format'}, 'coverage': None}
    assert case_records.record_execution_comparison(unavailable, row) == 'incomparable'
    legacy = {key: value for key, value in row.items() if key not in {'observation_schema', 'execution_metadata'}}
    assert not case_records.record_execution_complete(legacy)
    assert case_records.record_execution_comparison(unavailable, legacy) == 'incomparable'
    assert case_records.record_execution_comparison(legacy, row) == 'incomparable'
    for invalid in ({**row, 'execution_metadata': {'availability': 'unknown'}}, {**row, 'coverage': None}, {**row, 'coverage': {**row['coverage'], 'unknown_or_unread_rule_file_pairs': 1}}, {**row, 'coverage': {**row['coverage'], 'selected_rule_file_pairs': 2}}):
        assert not case_records.record_execution_complete(invalid)
        assert case_records.record_execution_comparison(unavailable, invalid) == 'different'
    skipped = {**row, 'skipped': [{'rule': 'UP025', 'path': 'case.py', 'reason': 'syntax-rejected'}], 'coverage': {**row['coverage'], 'evaluated_rule_file_pairs': 0, 'skipped_rule_file_pairs': 1}}
    assert case_records.record_execution_complete(skipped)
    assert case_records.record_execution_comparison(row, skipped) == 'different'
    assert case_records.record_execution_comparison(skipped, skipped) == 'exact'
    assert case_records.record_execution_comparison(unavailable, skipped) == 'incomparable'

def context_guard_self_test() -> None:
    """Independent native receipt rejects missing, mixed, and undeclared context evidence."""
    authored = {'files': {'mod.py': 'from pkg.mod import *\n'}, 'context': {'logical-root': 'pkg', 'auxiliary-files': {'pkg/__init__.py': ''}, 'missing': ['__init__.py']}}
    identity = case_records.expected_context_identity(authored, 'listed-disk')
    if os.name != 'nt':
        assert identity['manifest_hash'] == 'e245d4740b16dcfc7622f5b1fb7197ef320c88a48245d856a9410eca78485563'
    manifest = {'mode': 'listed-disk', 'logical_root': 'pkg', 'actual_root': str(Path(tempfile.gettempdir()).resolve() / 'sealed-context' / 'pkg'), 'manifest_hash': identity['manifest_hash']}
    case = {'project_context_mode': 'listed-disk', 'context_identity': identity}
    sides = {'reference': {'context_manifest': manifest, 'project_context': None}, 'candidate': {'context_manifest': manifest, 'project_context': {**manifest, 'dependencies': {'../__init__.py': None, '__init__.py': [175, 19, 73, 185, 245, 249, 161, 166, 160, 64, 77, 234, 54, 220, 201, 73]}}}}
    assert case_records.context_receipts_valid(case, sides)
    assert not case_records.context_receipts_valid({'project_context_mode': 'isolated'}, sides)
    for field, value in (('mode', 'isolated'), ('manifest_hash', '0' * 64), ('logical_root', 'other'), ('actual_root', 'relative')):
        invalid = json.loads(json.dumps(sides))
        invalid['candidate']['context_manifest'][field] = value
        assert not case_records.context_receipts_valid(case, invalid)
    for field in ('context_manifest', 'project_context'):
        invalid = json.loads(json.dumps(sides))
        invalid['candidate'][field] = None
        assert not case_records.context_receipts_valid(case, invalid)
    for dependencies in ({'undeclared.py': [0] * 16}, {'__init__.py': None}, {'../__init__.py': [0] * 16}, {'__init__.py': [0] * 15}, {'../../escape.py': None}):
        invalid = json.loads(json.dumps(sides))
        invalid['candidate']['project_context']['dependencies'] = dependencies
        assert not case_records.context_receipts_valid(case, invalid)
    altered = json.loads(json.dumps(authored))
    altered['context']['auxiliary-files']['pkg/__init__.py'] = '# changed\n'
    assert not case_records.context_receipts_valid({**case, 'context_identity': case_records.expected_context_identity(altered, 'listed-disk')}, sides)
    for mode, source in (('isolated', authored), ('listed-disk', {'files': authored['files']})):
        try:
            case_records.expected_context_identity(source, mode)
        except ValueError:
            pass
        else:
            raise AssertionError('mixed or missing context admission was accepted')
    with tempfile.TemporaryDirectory(prefix='ryo-context-stdin-refusal-') as temporary:
        root = Path(temporary)
        (root / 'benchmark.toml').write_text('project-context-mode = "listed-disk"\n', encoding='utf-8')
        try:
            profile_worker.verify_reference_profile('context', root, root / 'must-not-launch', root / 'output', {})
        except ValueError as error:
            assert 'stdin transport' in str(error)
        else:
            raise AssertionError('ListedDisk was silently transported over stdin')

def report_cache_contract_self_test() -> None:
    """Bind enabled CLI cache paths without weakening fresh-case defaults."""
    with tempfile.TemporaryDirectory() as temporary:
        cache = Path(temporary) / 'trial-cache'
        settings = {'defaults': ['GUARD001'], 'parser_configuration': 'python-target=3.10', 'project_context_mode': 'isolated', 'rule_applicability': {'GUARD001': 'accepted-syntax'}, 'flags': {}}
        request = {'root': temporary, 'rules': temporary, 'query_modules': {}, 'rule_applicability': settings['rule_applicability'], 'parser_configuration': settings['parser_configuration'], 'files': ['case.py'], 'cache': str(cache)}
        report = json.dumps({'execution_metadata': {'availability': 'complete'}, 'diagnostics': [], 'files': ['case.py'], 'skipped': [], 'coverage': {'selected': 1, 'evaluated': 1, 'skipped': 0, 'unknown_or_unread': 0}})
        case_records.complete_run_ryo_report(0, report, '', ['case.py'], ['GUARD001'], request=request, settings=settings, expected_cache_path=cache)
        cache.mkdir()
        normalized = case_records.complete_run_ryo_report(0, report, '', ['case.py'], ['GUARD001'], request=request, settings=settings, expected_cache_path=cache)
        raw = json.loads(report)
        assert normalized == {key: raw[key] for key in ('diagnostics', 'skipped', 'coverage', 'files')}
        for capability in (None, {'availability': 'unknown'}, {'availability': 'unavailable', 'reason': 'not observed'}):
            invalid = {**raw}
            if capability is None:
                invalid.pop('execution_metadata')
            else:
                invalid['execution_metadata'] = capability
            try:
                case_records.complete_run_ryo_report(0, json.dumps(invalid), '', ['case.py'], ['GUARD001'], request=request, settings=settings, expected_cache_path=cache)
            except ValueError:
                pass
            else:
                raise AssertionError('missing or unavailable fresh candidate metadata was accepted')
        for arguments in ({'request': request, 'settings': settings}, {'request': {**request, 'cache': None}, 'settings': settings, 'expected_cache_path': cache}, {'request': {**request, 'cache': str(cache / 'wrong')}, 'settings': settings, 'expected_cache_path': cache}, {'expected_cache_path': cache}, {'request': {**request, 'cache_dir': str(cache)}, 'settings': settings, 'expected_cache_path': cache}):
            try:
                case_records.complete_run_ryo_report(0, report, '', ['case.py'], ['GUARD001'], **arguments)
            except ValueError:
                continue
            raise AssertionError('unbound, overridden or default-enabled cache was accepted')

def self_test_standard_suppression() -> None:
    """Reject declaration-only, dead-call, missing-module and copied-body forms."""
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        profile = root / 'benchmark.toml'
        module = root / 'fragments/suppress.ryoql'
        module.parent.mkdir()
        module.write_text('return candidates;\n', encoding='utf-8')
        query = root / 'rules/GUARD001/query.ryoql'
        query.parent.mkdir(parents=True)
        declared = '[query-modules]\n"python/ruff/suppress" = "fragments/suppress.ryoql"\n'
        imported = 'import hide from "python/ruff/suppress";\n'
        called = 'let filtered = hide(candidates=candidate_locations);\n'
        emitted = 'let violations = select id, file, span from filtered;\nemit violations subject file key(id);\n'
        policies = {'GUARD001': 'standard-module'}
        profile.write_text(declared, encoding='utf-8')
        query.write_text(imported + called + emitted, encoding='utf-8')
        profile_proofs.check_standard_suppression(profile, ['GUARD001'], policies)
        for source in (imported + 'emit candidate_locations subject file key(id);\n', imported + called + 'emit candidate_locations subject file key(id);\n', called + emitted, imported + called + emitted + 'let mapping_edges = select * from "semantic-child";\n'):
            query.write_text(source, encoding='utf-8')
            try:
                profile_proofs.check_standard_suppression(profile, ['GUARD001'], policies)
            except ValueError:
                continue
            raise AssertionError('invalid standard suppression source was accepted')
        query.write_text(imported + called + emitted, encoding='utf-8')
        profile.write_text('', encoding='utf-8')
        try:
            profile_proofs.check_standard_suppression(profile, ['GUARD001'], policies)
        except ValueError:
            pass
        else:
            raise AssertionError('undeclared standard suppression module was accepted')
        profile.write_text(declared, encoding='utf-8')
        module.unlink()
        try:
            profile_proofs.check_standard_suppression(profile, ['GUARD001'], policies)
        except ValueError:
            pass
        else:
            raise AssertionError('unstaged standard suppression module was accepted')
        module.write_text('import core from "python/ruff/suppress-directives";\nreturn candidates;\n', encoding='utf-8')
        try:
            profile_proofs.check_standard_suppression(profile, ['GUARD001'], policies)
        except ValueError:
            pass
        else:
            raise AssertionError('undeclared transitive suppression module was accepted')
        profile.write_text(declared + '"python/ruff/suppress-directives" = "fragments/suppress-directives.ryoql"\n', encoding='utf-8')
        try:
            profile_proofs.check_standard_suppression(profile, ['GUARD001'], policies)
        except ValueError:
            pass
        else:
            raise AssertionError('unstaged transitive suppression module was accepted')
        (module.parent / 'suppress-directives.ryoql').write_text('return candidates;\n', encoding='utf-8')
        profile_proofs.check_standard_suppression(profile, ['GUARD001'], policies)
        for policy_name, wrapper in (('unmapped-module', 'suppress-unmapped'), ('additional-lines-module', 'suppress-with-additional-lines')):
            variant = {'GUARD001': policy_name}
            profile.write_text('[query-modules]\n' + f'"python/ruff/{wrapper}" = "fragments/{wrapper}.ryoql"\n', encoding='utf-8')
            (module.parent / (wrapper + '.ryoql')).write_text('return candidates;\n', encoding='utf-8')
            query.write_text(imported.replace('python/ruff/suppress', 'python/ruff/' + wrapper) + called + emitted, encoding='utf-8')
            profile_proofs.check_standard_suppression(profile, ['GUARD001'], variant)
            query.write_text(imported + called + emitted, encoding='utf-8')
            try:
                profile_proofs.check_standard_suppression(profile, ['GUARD001'], variant)
            except ValueError:
                pass
            else:
                raise AssertionError('wrong mapped/unmapped suppression import was accepted')
        profile_proofs.check_standard_suppression(profile, ['UNMAPPED001'], policies)
        # The explicit matching family must feed both diagnostic filtering and named publication.
        profile.write_text('[query-modules]\n"python/ruff/suppression-edges" = "fragments/suppression-edges.ryoql"\n', encoding='utf-8')
        (module.parent / 'suppression-edges.ryoql').write_text('return candidates;\n', encoding='utf-8')
        explicit = 'import matching from "python/ruff/suppression-edges";\nlet matched = matching(candidates=candidate_locations);\nlet visible = antijoin(left=candidate_locations, right=matched, left-on=[id], right-on=[id]);\n'
        publication = 'let published = select directive, file from matched;\nobserve published as used;\n'
        emission = 'emit visible subject file key(id);\n'
        query.write_text(explicit + publication + emission, encoding='utf-8')
        profile_proofs.check_standard_suppression(profile, ['GUARD001'], {'GUARD001': 'explicit-edges-module'})
        for source in (explicit + emission, explicit + 'observe candidate_locations as used;\n' + emission, explicit + publication + 'emit candidate_locations subject file key(id);\n'):
            query.write_text(source, encoding='utf-8')
            try:
                profile_proofs.check_standard_suppression(profile, ['GUARD001'], {'GUARD001': 'explicit-edges-module'})
            except ValueError:
                continue
            raise AssertionError('explicit matching family lost its Emit or named-output dependency')
    valid_report = {'execution_metadata': {'availability': 'complete'}, 'diagnostics': [], 'skipped': [], 'files': ['case.py'], 'coverage': {'selected': 1, 'evaluated': 1, 'skipped': 0, 'unknown_or_unread': 0}}
    case_records.complete_run_ryo_report(0, json.dumps(valid_report), '', ['case.py'], ['GUARD001'])
    phases = json.loads(profile_config.RULE_PHASES_FILE.read_text(encoding='utf-8'))
    for rule in ('EXE005', 'PYI033'):
        settings = profile_sources.canonical_run_ryo_settings({'tool': 'ruff', 'version': profile_config.PINNED_RUFF_VERSION, 'defaults': [rule]}, [rule], phases)
        assert settings['rule_applicability'] == {rule: 'always'}
        request = {'root': '/corpus', 'rules': '/profile/rules', 'query_modules': {}, 'rule_applicability': settings['rule_applicability'], 'parser_configuration': settings['parser_configuration'], 'files': ['case.py'], 'cache': None}
        case_records.complete_run_ryo_report(0, json.dumps(valid_report), '', ['case.py'], [rule], request=request, settings=settings)
        for wrong in ({**request, 'rule_applicability': {rule: 'accepted-syntax'}}, {**request, 'parser_configuration': 'python-target=3.11'}, {**request, 'project_context': {'mode': 'listed-disk'}}, {**request, 'preview': True}):
            try:
                case_records.complete_run_ryo_report(0, json.dumps(valid_report), '', ['case.py'], [rule], request=wrong, settings=settings)
            except ValueError:
                continue
            raise AssertionError('complete report with noncanonical settings was accepted')
    for status, stdout, stderr in ((1, '', 'query module is a symlink: fragments/suppress.ryoql\n'), (1, '', 'same refusal from both workers\n'), (0, '', ''), (0, '{}', ''), (0, json.dumps({**valid_report, 'coverage': {'selected': 1}}), ''), (0, json.dumps({**valid_report, 'files': []}), ''), (0, json.dumps({**valid_report, 'coverage': {**valid_report['coverage'], 'evaluated': 0}}), ''), (0, json.dumps({**valid_report, 'coverage': {**valid_report['coverage'], 'unknown_or_unread': 1}}), '')):
        for _side in ('baseline', 'candidate'):
            try:
                case_records.complete_run_ryo_report(status, stdout, stderr, ['case.py'], ['GUARD001'])
            except (ValueError, json.JSONDecodeError):
                continue
            raise AssertionError('incomplete direct worker report was accepted')


def reference_discrepancy_self_test() -> None:
    """Two concrete independent-language fixtures exercise admission, not diagnostic ignores."""
    cross = json.loads((profile_config.REPOSITORY / 'crates/ryo/crates/benchmark/tests/fixtures/reference-discrepancy-canonical.json').read_text())
    assert reference_discrepancies.discrepancy_json_hash(cross['output']) == cross['sha256']
    source_digest = hashlib.sha256()
    for filename, text in sorted(cross['source_files'].items()):
        source_digest.update(filename.encode())
        source_digest.update(b'\x00')
        source_digest.update(text.encode())
    assert source_digest.hexdigest() == cross['source_sha256']
    assert json.dumps(cross['output'], sort_keys=True, separators=(',', ':'), ensure_ascii=False) == cross['canonical_utf8']
    source = profile_config.RUFF_DIR / 'fixtures/reference-discrepancy'
    with tempfile.TemporaryDirectory(prefix='reference-discrepancy-guard-') as temporary:
        profile = Path(temporary)
        shutil.copyfile(source / 'benchmark.toml', profile / 'benchmark.toml')
        shutil.copytree(source / 'evidence', profile / 'evidence')
        cases = tomllib.loads((source / 'rules/F821/cases.toml').read_text())['cases']
        case = cases[0]
        certificate = reference_discrepancies.validate_reference_discrepancy(profile, case, 'F821')
        assert certificate is not None
        coverage = {'selected_rule_file_pairs': 1, 'evaluated_rule_file_pairs': 1, 'skipped_rule_file_pairs': 0, 'unknown_or_unread_rule_file_pairs': 0}
        reference = {'complete': True, 'error': None, 'files': sorted(case['files']), 'skipped': [], 'diagnostics': certificate['reference_expected'], 'coverage': None}
        candidate = {**reference, 'diagnostics': certificate['intended_expected'], 'coverage': coverage}
        kwargs = {'actual_executable_sha256': certificate['approved_executable_sha256'][1], 'actual_settings_sha256': certificate['identity']['settings-sha256']}
        result = reference_discrepancies.classify_reference_discrepancy(profile, case, 'F821', reference, candidate, **kwargs)
        assert result['intended_behavior_pass'] and (not result['raw_parity_match'])

        def rejected(changed_case: dict[str, object], ref: dict[str, object]=reference, actual: dict[str, object]=candidate, options: dict[str, str]=kwargs) -> None:
            try:
                reference_discrepancies.classify_reference_discrepancy(profile, changed_case, 'F821', ref, actual, **options)
            except (ValueError, OSError, UnicodeError):
                return
            raise AssertionError('invalid discrepancy was accepted')
        clone = lambda value: json.loads(json.dumps(value))
        with_context = clone(case)
        with_context['files']['unselected.py'] = 'pass\n'
        with_context['requested-files'] = ['case.pyi']
        source_digest = hashlib.sha256()
        for filename, text in sorted(with_context['files'].items()):
            source_digest.update(filename.encode())
            source_digest.update(b'\x00')
            source_digest.update(text.encode())
        with_context['reference-discrepancy']['identity']['source-sha256'] = source_digest.hexdigest()
        selected_certificate = reference_discrepancies.validate_reference_discrepancy(profile, with_context, 'F821')
        assert selected_certificate is not None
        assert selected_certificate['identity']['intended-output-sha256'] == certificate['identity']['intended-output-sha256']
        assert selected_certificate['identity']['reference-output-sha256'] == certificate['identity']['reference-output-sha256']
        assert reference_discrepancies.classify_reference_discrepancy(profile, with_context, 'F821', reference, candidate, **kwargs)['intended_behavior_pass']
        stale_inventory = clone(with_context)
        stale_identity = stale_inventory['reference-discrepancy']['identity']
        stale_identity['intended-output-sha256'] = reference_discrepancies.discrepancy_json_hash(reference_discrepancies.discrepancy_output(list(stale_inventory['files']), stale_inventory['expected']))
        stale_identity['reference-output-sha256'] = reference_discrepancies.discrepancy_json_hash(reference_discrepancies.discrepancy_output(list(stale_inventory['files']), stale_inventory['reference-discrepancy']['expected']))
        rejected(stale_inventory)
        mutated_context = clone(with_context)
        mutated_context['files']['unselected.py'] = 'return\n'
        rejected(mutated_context)
        unselected_diagnostic = clone(with_context)
        unselected_diagnostic['expected'] = [{'rule': 'F821', 'file': 'unselected.py', 'start': 0, 'end': 4}]
        rejected(unselected_diagnostic)
        changed = clone(case)
        changed['tool-contract-difference'] = clone(changed['reference-discrepancy'])
        rejected(changed)
        try:
            reference_discrepancies.declared_discrepancy_artifacts(profile, [changed])
        except ValueError:
            pass
        else:
            raise AssertionError('dual discrepancy authorities were staged')
        changed = clone(case)
        changed['reference-discrepancy']['evidence'] = []
        rejected(changed)
        changed = clone(case)
        changed['comparison'] = 'rule'
        rejected(changed)
        changed = clone(case)
        changed['reference-discrepancy']['expected'] = clone(changed['expected'])
        rejected(changed)
        changed = clone(case)
        changed['reference-discrepancy']['identity']['reference-output-sha256'] = '0' * 64
        rejected(changed)
        changed = clone(case)
        changed['expected'] = clone(changed['reference-discrepancy']['expected'])
        rejected(changed)
        changed = clone(case)
        changed['expected'] = [{'rule': 'F821', 'file': 'case.pyi', 'start': 0, 'end': 1}]
        rejected(changed)
        changed = clone(case)
        changed['reference-discrepancy']['identity']['profile-sha256'] = '0' * 64
        rejected(changed)
        changed = clone(case)
        changed['files']['case.pyi'] += 'pass\n'
        rejected(changed)
        changed = clone(case)
        changed['reference-discrepancy']['evidence'][0]['sha256'] = '0' * 64
        rejected(changed)
        changed = clone(case)
        changed['reference-discrepancy']['evidence'][0]['kind'] = 'competitor-output'
        rejected(changed)
        changed = clone(case)
        changed['reference-discrepancy']['evidence'][0]['target-configuration'] = 'python-target=3.11'
        rejected(changed)
        rejected(case, actual={**candidate, 'diagnostics': reference['diagnostics']})
        rejected(case, ref={**reference, 'diagnostics': candidate['diagnostics']})
        rejected(case, actual={**candidate, 'coverage': {**coverage, 'unknown_or_unread_rule_file_pairs': 1}})
        rejected(case, actual={**candidate, 'coverage': None})
        rejected(case, actual={**candidate, 'complete': False, 'error': 'worker failed'})
        rejected(case, options={**kwargs, 'actual_executable_sha256': '0' * 64})
        rejected(case, options={**kwargs, 'actual_settings_sha256': '0' * 64})
        for annotated in cases[:2]:
            assert reference_discrepancies.validate_reference_discrepancy(profile, annotated, 'F821') is not None

def discrepancy_artifact_staging_self_test() -> None:
    """Declared evidence is shared exactly once and cannot escape or replace inputs."""
    fixture = profile_config.RUFF_DIR / 'fixtures/reference-discrepancy'
    with tempfile.TemporaryDirectory(prefix='ryo-evidence-staging-') as temporary:
        directory = Path(temporary)
        source = directory / 'source'
        shutil.copytree(fixture, source)
        cases = tomllib.loads((source / 'rules/F821/cases.toml').read_text())['cases'][:2]
        destination = directory / 'stage'
        destination.mkdir()
        shutil.copyfile(source / 'benchmark.toml', destination / 'benchmark.toml')
        artifacts = reference_discrepancies.declared_discrepancy_artifacts(source / 'benchmark.toml', cases)
        assert len(artifacts) == 4, 'two cases must share the four pinned artifacts'
        reference_discrepancies.stage_discrepancy_artifacts(source / 'benchmark.toml', destination, cases, 'F821')
        assert {str(path.relative_to(destination)) for path in (destination / 'evidence').rglob('*') if path.is_file()} == set(artifacts), 'undeclared evidence was copied'
        assert all((profile_config.sha256(destination / path) == digest for path, (_source, digest) in artifacts.items()))

        def declaration(path: str, digest: str) -> list[dict[str, object]]:
            return [{'reference-discrepancy': {'evidence': [{'path': path, 'sha256': digest}]}}]

        def refused(rows: list[dict[str, object]], *, stage: Path | None=None) -> None:
            try:
                if stage is None:
                    reference_discrepancies.declared_discrepancy_artifacts(source / 'benchmark.toml', rows)
                else:
                    reference_discrepancies.stage_discrepancy_artifacts(source / 'benchmark.toml', stage, rows, 'F821')
            except ValueError:
                return
            raise AssertionError('unsafe discrepancy evidence was accepted')
        path, (_file, digest) = next(iter(artifacts.items()))
        for unsafe in ['../outside.json', '/outside.json', 'evidence\\outside.json', 'C:/outside.json']:
            refused(declaration(unsafe, digest))
        refused(declaration('evidence/missing.json', digest))
        refused(declaration(path, '0' * 64))
        conflicting = declaration(path, digest) + declaration(path, '0' * 64)
        refused(conflicting)
        outside = directory / 'outside.json'
        outside.write_text('external language evidence')
        (source / 'evidence/escape.json').symlink_to(outside)
        refused(declaration('evidence/escape.json', profile_config.sha256(outside)))
        collision = 'fragments/occupied.ryoql'
        (source / collision).write_text('a declared source artifact')
        (destination / 'fragments').mkdir()
        occupied = destination / collision
        occupied.write_text('a canonical staged module')
        refused(declaration(collision, profile_config.sha256(source / collision)), stage=destination)
        assert occupied.read_text() == 'a canonical staged module'
        escape_stage = directory / 'escape-stage'
        escape_stage.mkdir()
        (escape_stage / 'evidence').symlink_to(directory)
        refused(declaration(path, digest), stage=escape_stage)
