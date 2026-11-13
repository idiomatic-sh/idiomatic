"""Profile cli tests: linter-specific authoring and proof utilities."""
from __future__ import annotations
import tempfile
from authoring import profile_guard_tests
from authoring import profile_guard_staging_tests
from authoring import profile_guard_proof_tests


def self_test() -> int:
    code, state = profile_guard_tests.guards_1()
    if code:
        return code
    sample_inventory, = state
    code, state = profile_guard_tests.guards_2(sample_inventory)
    if code:
        return code
    code, state = profile_guard_tests.guards_3()
    if code:
        return code
    case, fake_runner, root, ruff = state
    code, state = profile_guard_tests.guards_4()
    if code:
        return code
    manifest, = state
    profile_guard_staging_tests.staged_request_self_test()
    with tempfile.TemporaryDirectory(prefix='ryo-ruff-profile-guards-') as temporary:
        code, state = profile_guard_staging_tests.guards_5(fake_runner, manifest, root, temporary)
        if code:
            return code
        build_calls, build_identity, build_record_path, build_root, durable, frozen_worker, original_build_record, root, scheduled, successful_build_runner = state
        code, state = profile_guard_staging_tests.guards_6(build_calls, build_identity, build_record_path, build_root, durable, frozen_worker, manifest, original_build_record, root, scheduled, successful_build_runner)
        if code:
            return code
        expanded_descriptor, shape_descriptor = state
        code, state = profile_guard_staging_tests.guards_7(expanded_descriptor, manifest, root, shape_descriptor)
        if code:
            return code
        factory_controls, helper_relative, lambda_controls, pinned, relative, required = state
        code, state = profile_guard_proof_tests.guards_8(case, factory_controls, helper_relative, lambda_controls, manifest, pinned, relative, required, root, ruff)
        if code:
            return code
        case, fake_bin, helper, last_profile, last_rule, manifest, marker, pristine_manifest, ruff, temporary_manifest = state
        code, state = profile_guard_proof_tests.guards_9(fake_bin, helper, last_profile, last_rule, manifest, marker, pristine_manifest, root, ruff, temporary_manifest)
        if code:
            return code
    print('profile selection and hash-mismatch guards passed')
    return 0
