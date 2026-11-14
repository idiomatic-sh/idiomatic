"""Profile guards: certified fixtures, combined profiles and refusals."""
from __future__ import annotations
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
        elif kind == 'case-key':
            overlay['cases'][0]['case'] = 'unknown-case-key'
        elif kind == 'missing-rule':
            overlay['cases'][0]['expected'][0].pop('rule')
        elif kind == 'oracle':
            oracle_path = root / descriptor['oracle_file']
            oracle = json.loads(oracle_path.read_text(encoding='utf-8'))
            first = overlay['cases'][0]
            entry = next((case for case in oracle['cases'] if (case['rule'], case.get('case', case.get('name'))) == (first['rule'], first['case'])))
            entry['expected'] = [*entry['expected'], entry['expected'][0]]
            oracle_path.write_text(json.dumps(oracle, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
            trial_manifest['files'][descriptor['oracle_file']] = profile_config.sha256(oracle_path)
        else:
            raise ValueError(f'unknown self-test: {kind}')
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
    oracle_relative = pristine_manifest['combined_profiles'][profile_config.SHAPE_PROFILE]['oracle_file']
    baseline_oracle = (root / oracle_relative).read_bytes()
    for kind, message in (('source-hash', 'case source hash mismatch'), ('case-key', 'overlay case is absent'), ('missing-rule', f'invalid {profile_config.SHAPE_PROFILE} expected finding'), ('oracle', 'overlay diagnostics differ from the sealed oracle')):
        (root / overlay_relative).write_bytes(baseline_overlay)
        (root / oracle_relative).write_bytes(baseline_oracle)
        if not shape_preflight_failure(kind, message):
            print(f'shape {kind} refusal did not fail before candidate work', file=sys.stderr)
            return 1, ()
    (root / overlay_relative).write_bytes(baseline_overlay)
    (root / oracle_relative).write_bytes(baseline_oracle)
    return 0, ()
