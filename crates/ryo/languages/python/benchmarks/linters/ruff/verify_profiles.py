"""Linter-specific authoring CLI; complete inventory execution belongs to shared Rust."""
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from authoring.verification_cli import main
from authoring.profile_config import RUFF_DIR, sha256
from authoring.profile_sources import assert_staged_profile_fixed_point, profile_modules, write_staged_profile, canonical_run_ryo_settings
from authoring.profile_worker import run_ryo_command
from authoring.case_records import diagnostic_multiset, profile_case_inventory, check_run_ryo_settings, complete_run_ryo_report
from authoring.profile_case_tests import self_test_standard_suppression

__all__ = ["RUFF_DIR", "sha256", "assert_staged_profile_fixed_point", "profile_modules", "write_staged_profile", "canonical_run_ryo_settings", "run_ryo_command", "diagnostic_multiset", "profile_case_inventory", "check_run_ryo_settings", "complete_run_ryo_report", "self_test_standard_suppression"]

if __name__ == "__main__":
    raise SystemExit(main())
