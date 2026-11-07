"""Profile guards: certified fixtures, active combined profiles and refusals."""
from __future__ import annotations
import collections
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
import tomllib
from authoring import profile_config
from authoring import reference_discrepancies


def guards_8(case, factory_controls, helper_relative, lambda_controls, manifest, pinned, relative, required, root, ruff) -> tuple[int, tuple]:
    for rule, (source_hash, reference_hash) in pinned.items():
        recorded = manifest['profiles'][rule]
        cases_relative = profile_config.RUFF_DIR.relative_to(profile_config.REPOSITORY) / recorded['cases_file']
        cases_path = root / cases_relative
        if not cases_path.is_file():
            cases_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(profile_config.REPOSITORY / cases_relative, cases_path)
        cases = tomllib.loads(cases_path.read_text(encoding='utf-8'))['cases']
        names = [case['name'] for case in cases]
        previous = recorded['case_names']
        appended = factory_controls + (lambda_controls if rule == 'FURB132' else [])
        if names != previous + appended:
            print(f'{rule} fixture changed beyond its certified appended controls', file=sys.stderr)
            return 1, ()
        original = cases[:len(previous)]
        source_inputs = [{key: value for key, value in case.items()
                          if key not in {'expected', 'reference-discrepancy'}} for case in original]
        raw_reference = [case.get('reference-discrepancy', {}).get('expected', case['expected'])
                         for case in original]
        if (reference_discrepancies.discrepancy_json_hash(source_inputs) != source_hash
                or reference_discrepancies.discrepancy_json_hash(raw_reference) != reference_hash):
            print(f'{rule} pinned source inputs or raw reference observations changed', file=sys.stderr)
            return 1, ()
        for case in cases:
            reference_discrepancies.validate_reference_discrepancy(
                profile_config.RUFF_DIR / 'profiles' / rule, case, rule)
        recorded['case_names'] = names
        recorded['case_count'] = len(names)
    manifest['files'] = {relative: profile_config.sha256(root / relative) for relative in sorted(required)}
    temporary_manifest = root / profile_config.MANIFEST.relative_to(profile_config.REPOSITORY)
    temporary_manifest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    helper = root / helper_relative
    ruff = str(Path(sys.executable).resolve())
    pristine_manifest = json.loads(json.dumps(manifest))
    invalid_selection = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(root / 'invalid-output'), 'INVALID-RULE'], capture_output=True, text=True, check=False)
    if invalid_selection.returncode != 2 or 'unknown or duplicate profile rule' not in invalid_selection.stderr:
        print(f'invalid profile selection did not fail closed: status={invalid_selection.returncode}; stderr={invalid_selection.stderr!r}', file=sys.stderr)
        return 1, ()
    duplicate_selection = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(root / 'duplicate-output'), 'F401', 'F401'], capture_output=True, text=True, check=False)
    if duplicate_selection.returncode != 2 or 'unknown or duplicate profile rule' not in duplicate_selection.stderr:
        print('duplicate profile selection did not fail closed', file=sys.stderr)
        return 1, ()
    candidate = root / next((path for path in manifest['files'] if path.endswith('/rules/F401/candidate.ryoql')))
    original_candidate = candidate.read_bytes()
    candidate.write_bytes(original_candidate + b'\n')
    changed_input = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(root / 'changed-output'), 'F401'], capture_output=True, text=True, check=False)
    if changed_input.returncode != 2 or 'prepared input hash mismatch' not in changed_input.stderr:
        print('prepared input hash mismatch did not fail closed', file=sys.stderr)
        return 1, ()
    candidate.write_bytes(original_candidate)
    manifest = json.loads(temporary_manifest.read_text(encoding='utf-8'))
    last_profile = max(manifest['profiles'])
    last_rule = manifest['profiles'][last_profile].get('rule_id', last_profile)
    manifest['profiles'][last_profile]['case_count'] += 1
    temporary_manifest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    fake_bin = root / 'fake-bin'
    fake_bin.mkdir()
    marker = root / 'worker-was-started'
    fake_cargo = fake_bin / 'cargo'
    fake_git = fake_bin / 'git'
    fake_git.write_text('#!/bin/sh\ncase "$1" in\n  rev-parse) echo self-test-revision ;;\n  diff|status|ls-files) : ;;\n  *) exit 2 ;;\nesac\n', encoding='utf-8')
    fake_git.chmod(493)
    cargo_build_marker = root / 'cargo-build-count'
    reference_worker = root / 'reference-worker'
    reference_worker.write_text(f"""#!/bin/sh\nif [ "$1" = audit ]; then echo 'ruff 0.16.5: 104 queries authored / 413 shipped defaults (not a parity certificate)'; exit 0; fi\nprintf '%s\\n' "$*" >> {marker}\nexit 0\n""", encoding='utf-8')
    reference_worker.chmod(493)
    artifact_json = json.dumps({'reason': 'compiler-artifact', 'target': {'name': 'ruff', 'kind': ['bench'], 'src_path': '/repo/languages/python/benchmarks/linters/ruff/bench.rs', 'test': False}, 'executable': str(reference_worker)})
    fake_cargo.write_text(f"""#!/bin/sh\nif [ "$1" = "--version" ]; then echo 'cargo test'; exit 0; fi\nprintf '%s\\n' "$*" >> {cargo_build_marker}\nprintf '%s\\n' '{artifact_json}'\n""", encoding='utf-8')
    fake_cargo.chmod(493)
    pinned_ruff = shutil.which('ruff')
    if pinned_ruff is None:
        print('reference-only self-test requires the pinned Ruff executable on PATH', file=sys.stderr)
        return 1, ()
    inventory_collector = fake_bin / 'inventory-collector'
    inventory_collector.write_text('#!/bin/sh\nexit 0\n', encoding='utf-8')
    inventory_collector.chmod(493)
    inventory_output = root / 'default-inventory-output'
    default_inventory = subprocess.run([sys.executable, str(helper), pinned_ruff, '--worker', str(reference_worker), '--inventory-runner', str(inventory_collector), '--output-dir', str(inventory_output), 'F401'], capture_output=True, text=True, check=False, env={**os.environ, 'RYO_RUFF_PREPARED_MANIFEST': str(temporary_manifest)})
    registration_path = inventory_output / 'registration.json'
    registration = json.loads(registration_path.read_text(encoding='utf-8')) if registration_path.is_file() else {}
    rows = registration.get('rules', [])
    if default_inventory.returncode != 0 or len(rows) != 413 or [row['rule_id'] for row in rows if row['command'] is not None] != ['F401'] or sum(row['reason'] == 'not selected for this invocation' for row in rows) != 412:
        print(f'default canonical inventory routing failed: status={default_inventory.returncode}; stderr={default_inventory.stderr!r}', file=sys.stderr)
        return 1, ()
    reference_output = root / 'reference-only-output'
    reference_only = subprocess.run([sys.executable, str(helper), pinned_ruff, '--prepared', '--output-dir', str(reference_output), '--reference-only', 'F401'], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}", 'RYO_RUFF_PREPARED_MANIFEST': str(temporary_manifest)})
    builds = cargo_build_marker.read_text().splitlines() if cargo_build_marker.exists() else []
    reference_records = reference_output / 'profiles' / 'F401' / 'reference-cases.jsonl'
    expected_reference_names = {case['name'] for case in tomllib.loads((profile_config.REPOSITORY / 'crates/ryo/languages/python/benchmarks/linters/ruff/rules/F401/cases.toml').read_text(encoding='utf-8'))['cases']}
    observed_reference_names = [json.loads(line)['name'] for line in reference_records.read_text(encoding='utf-8').splitlines()] if reference_records.is_file() else []
    if reference_only.returncode != 0 or builds or (not reference_records.is_file()) or (len(observed_reference_names) != len(expected_reference_names)) or (set(observed_reference_names) != expected_reference_names):
        print(f'reference-only mode did not verify all cases without building a candidate: status={reference_only.returncode}; stderr={reference_only.stderr!r}; builds={builds!r}; records={reference_records.is_file()}', file=sys.stderr)
        return 1, ()
    cargo_build_marker.unlink(missing_ok=True)
    invalid_rule_manifest = json.loads(json.dumps(pristine_manifest))
    invalid_rule_manifest['combined_profiles']['expanded-defaults']['selected_rules'].append('NOT-SHIPPED')
    temporary_manifest.write_text(json.dumps(invalid_rule_manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    invalid_rule = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(root / 'invalid-rule-output'), '--profile', 'expanded-defaults'], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
    if invalid_rule.returncode != 2 or 'combined profile contains a non-default rule' not in invalid_rule.stderr or marker.exists():
        print('non-default combined rule did not fail before candidate work', file=sys.stderr)
        return 1, ()
    temporary_manifest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    return 0, (case, fake_bin, helper, last_profile, last_rule, manifest, marker, pristine_manifest, ruff, temporary_manifest)


def guards_9(fake_bin, helper, last_profile, last_rule, manifest, marker, pristine_manifest, root, ruff, temporary_manifest) -> tuple[int, tuple]:
    def later_combined_preflight_failure() -> tuple[bool, str]:
        trial_manifest = json.loads(json.dumps(pristine_manifest))
        later_name = next((name for name, descriptor in sorted(trial_manifest['combined_profiles'].items()) if descriptor.get('validation_status') == 'active-current'))
        for item in trial_manifest['combined_profiles'].values():
            item['validation_status'] = 'archived-historical'
        descriptor = trial_manifest['combined_profiles'][later_name]
        descriptor['validation_status'] = 'active-current'
        descriptor['case_count'] += 1
        temporary_manifest.write_text(json.dumps(trial_manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
        marker.unlink(missing_ok=True)
        preflight_output = root / 'later-preflight-output'
        check = subprocess.run([sys.executable, str(helper), ruff, '--prepared', '--output-dir', str(preflight_output), '--profile', later_name, 'F401'], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}", 'RYO_RUFF_PREPARED_MANIFEST': str(temporary_manifest)})
        receipt_path = preflight_output / 'preflight-error.json'
        receipt = json.loads(receipt_path.read_text(encoding='utf-8')) if receipt_path.is_file() else {}
        valid = check.returncode == 2 and 'complete expected inventory count mismatch' in check.stderr and (not marker.exists()) and (receipt.get('manifest_path') == temporary_manifest.relative_to(root).as_posix()) and (receipt.get('manifest_sha256') == profile_config.sha256(temporary_manifest))
        return (valid, check.stderr)
    later_combined_refused, later_combined_output = later_combined_preflight_failure()
    if not later_combined_refused:
        print('later active combined profile did not fail before candidate work', file=sys.stderr)
        print(later_combined_output, file=sys.stderr)
        return 1, ()
    temporary_manifest.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    overlay_relative = pristine_manifest['combined_profiles'][profile_config.SHAPE_PROFILE]['expectations_file']
    baseline_overlay = (profile_config.REPOSITORY / overlay_relative).read_bytes()
    (root / pristine_manifest['combined_profiles']['expanded-defaults']['expectations_file']).write_bytes((profile_config.REPOSITORY / pristine_manifest['combined_profiles']['expanded-defaults']['expectations_file']).read_bytes())
    (root / pristine_manifest['combined_profiles']['shape-110']['expectations_file']).write_bytes(baseline_overlay)
    preflight = subprocess.run([sys.executable, str(helper), str(Path(sys.executable).resolve()), '--prepared', '--output-dir', str(root / 'inventory-output'), 'F401', last_rule], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
    if preflight.returncode != 2 or f'case inventory mismatch for {last_profile}' not in preflight.stderr or marker.exists():
        print(f'later-profile preflight guard failed: status={preflight.returncode}; stderr={preflight.stderr.strip()!r}; marker={marker.exists()}', file=sys.stderr)
        return 1, ()

    def shape_preflight_failure(kind: str, expected_error: str) -> bool:
        trial_manifest = json.loads(json.dumps(pristine_manifest))
        descriptor = trial_manifest['combined_profiles'][profile_config.SHAPE_PROFILE]
        overlay_path = root / descriptor['expectations_file']
        overlay = json.loads(overlay_path.read_text(encoding='utf-8'))
        if kind == 'source-hash':
            overlay['cases'][0]['case_source_sha256'] = '0' * 64
        else:
            overlay['cases'][0]['case'] = 'unknown-case-key'
        overlay_path.write_text(json.dumps(overlay, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
        overlay_hash = profile_config.sha256(overlay_path)
        descriptor['expectations_sha256'] = overlay_hash
        trial_manifest['files'][descriptor['expectations_file']] = overlay_hash
        temporary_manifest.write_text(json.dumps(trial_manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
        marker.unlink(missing_ok=True)
        check = subprocess.run([sys.executable, str(helper), ruff, '--output-dir', str(root / f'shape-{kind}-output'), '--profile', profile_config.SHAPE_PROFILE], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
        valid = check.returncode == 2 and expected_error in check.stderr and (not marker.exists())
        if not valid:
            print(f'shape guard status={check.returncode}; stderr={check.stderr!r}', file=sys.stderr)
        return valid
    for kind, message in (('source-hash', 'case source hash mismatch'), ('case-key', 'overlay case is absent')):
        (root / overlay_relative).write_bytes(baseline_overlay)
        if not shape_preflight_failure(kind, message):
            print(f'shape {kind} refusal did not fail before candidate work', file=sys.stderr)
            return 1, ()
    (root / overlay_relative).write_bytes(baseline_overlay)
    active_name = next((name for name, item in pristine_manifest['combined_profiles'].items() if item.get('validation_status') == 'active-current'))
    ready_descriptor = pristine_manifest['combined_profiles'][active_name]
    ready_file = ready_descriptor.get('ready_assets_file')
    if ready_file is not None:
        ready_path = root / ready_file
        pristine_ready = ready_path.read_bytes()
        for kind in ('query', 'module'):
            trial_manifest = json.loads(json.dumps(pristine_manifest))
            receipt = json.loads(pristine_ready)
            query_paths = set(ready_descriptor['query_files'].values())
            asset = next((item for item in receipt['assets'] if (item['path'] in query_paths) == (kind == 'query')))
            asset['sha256'] = '0' * 64
            ready_path.write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
            trial_manifest['files'][ready_file] = profile_config.sha256(ready_path)
            temporary_manifest.write_text(json.dumps(trial_manifest, indent=2) + '\n', encoding='utf-8')
            marker.unlink(missing_ok=True)
            check = subprocess.run([sys.executable, str(helper), ruff, '--output-dir', str(root / f'ready-{kind}-output'), '--profile', active_name], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
            if check.returncode != 2 or 'READY query/module bytes differ from receipt' not in check.stderr or marker.exists():
                print(f'READY {kind} receipt disagreement did not fail before candidate work: {check.stderr!r}', file=sys.stderr)
                return 1, ()
        ready_path.write_bytes(pristine_ready)
    return 0, (kind,)


def guards_10(case, fake_bin, helper, kind, marker, pristine_manifest, root, ruff, temporary_manifest) -> tuple[int, tuple]:
    def expanded_preflight_failure(kind: str, expected_error: str) -> bool:
        trial_manifest = json.loads(json.dumps(pristine_manifest))
        active_name = next((name for name, item in trial_manifest['combined_profiles'].items() if item.get('validation_status') == 'active-current'))
        descriptor = trial_manifest['combined_profiles'][active_name]
        overlay_path = root / descriptor['expectations_file']
        overlay = json.loads(overlay_path.read_text(encoding='utf-8'))
        if kind == 'source-hash':
            overlay['cases'][0]['case_source_sha256'] = '0' * 64
        elif kind == 'case-key':
            overlay['cases'][0]['case'] = 'unknown-case-key'
        elif kind == 'missing-case':
            overlay['cases'].pop()
        elif kind == 'missing-rule':
            item = next((row for row in overlay['cases'] if row['expected']))
            item['expected'][0].pop('rule')
        elif kind == 'duplicate':
            item = duplicate = None
            for row in overlay['cases']:
                cases_path = profile_config.REPOSITORY / row['cases_file']
                source_cases = tomllib.loads(cases_path.read_text(encoding='utf-8'))['cases']
                canonical = next((case for case in source_cases if case['name'] == row['case']))
                original = collections.Counter(((finding.get('rule', row['rule']), finding['file'], finding['start'], finding['end']) for finding in canonical['expected']))
                current = collections.Counter(((finding['rule'], finding['file'], finding['start'], finding['end']) for finding in row['expected']))
                extra_duplicate = next((key for key, count in current.items() if count > 1 and count > original[key]), None)
                if extra_duplicate is not None:
                    item, duplicate = (row, extra_duplicate)
                    break
            if item is None or duplicate is None:
                raise ValueError('no externally added duplicate to exercise')
            item['expected'].remove(next((finding for finding in item['expected'] if (finding['rule'], finding['file'], finding['start'], finding['end']) == duplicate)))
            overlay['expected_diagnostic_count'] -= 1
        else:
            raise ValueError(f'unknown self-test: {kind}')
        overlay_path.write_text(json.dumps(overlay, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
        overlay_hash = profile_config.sha256(overlay_path)
        descriptor['expectations_sha256'] = overlay_hash
        trial_manifest['files'][descriptor['expectations_file']] = overlay_hash
        temporary_manifest.write_text(json.dumps(trial_manifest, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
        marker.unlink(missing_ok=True)
        check = subprocess.run([sys.executable, str(helper), ruff, '--output-dir', str(root / f'active-{kind}-output'), '--profile', active_name], capture_output=True, text=True, check=False, env={**os.environ, 'PATH': f"{fake_bin}:{os.environ['PATH']}"})
        success = check.returncode == 2 and expected_error in check.stderr and (not marker.exists())
        if not success:
            print(f'active {kind}: status={check.returncode}; stderr={check.stderr.strip()!r}; cargo-started={marker.exists()}', file=sys.stderr)
        return success
    active_name = next((name for name, item in pristine_manifest['combined_profiles'].items() if item.get('validation_status') == 'active-current'))
    expanded_overlay_relative = pristine_manifest['combined_profiles'][active_name]['expectations_file']
    expanded_overlay_bytes = (root / expanded_overlay_relative).read_bytes()
    for kind, message in (('source-hash', 'case source hash mismatch'), ('case-key', 'overlay case is absent or duplicated'), ('missing-case', 'complete expected inventory count mismatch'), ('missing-rule', f'invalid {active_name} expected finding'), ('duplicate', 'overlay diagnostics differ from the sealed oracle')):
        (root / expanded_overlay_relative).write_bytes(expanded_overlay_bytes)
        if not expanded_preflight_failure(kind, message):
            print(f'active full-report {kind} refusal did not fail before candidate work', file=sys.stderr)
            return 1, ()
    return 0, ()
