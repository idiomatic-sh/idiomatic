"""Profile config: linter-specific authoring and proof utilities."""
from __future__ import annotations
import hashlib
import json
import os
from pathlib import Path
RUFF_DIR = Path(__file__).resolve().parents[1]
REPOSITORY = RUFF_DIR.parents[6]
PROFILE_DIR = RUFF_DIR / 'profiles'
MANIFEST = Path(os.environ.get('RYO_RUFF_PREPARED_MANIFEST', PROFILE_DIR / 'prepared.json'))
SHAPE_PROFILE = 'shape-110'
RULE_PHASES_FILE = PROFILE_DIR / 'pinned-rule-phases.json'
NATIVE_EVIDENCE_SUMMARIES = (RUFF_DIR / 'results/native143-split-2026-11-02/summary.json',
                             RUFF_DIR / 'results/native151-supplement-2026-11-02/summary.json')
HISTORICAL_NATIVE_DESCRIPTOR_FILE = PROFILE_DIR / 'native-standalone-104.json'
HISTORICAL_NATIVE_PROFILE_FILE = PROFILE_DIR / 'native-standalone-104-base.toml'
PARSER_CONFIGURATIONS = {'py37': 'python-target=3.7', 'py38': 'python-target=3.8', 'py39': 'python-target=3.9', 'py310': 'python-target=3.10', 'py311': 'python-target=3.11', 'py312': 'python-target=3.12', 'py313': 'python-target=3.13', 'py314': 'python-target=3.14', 'py315': 'python-target=3.15'}
PINNED_RUFF_VERSION = '0.16.5'
PINNED_RUFF_SOURCE_COMMIT = '9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a'
RUFF_PHASES = {'ast', 'filesystem', 'io', 'imports', 'logical-lines', 'noqa', 'physical-lines', 'toml', 'tokens'}
ENTRYPOINT = RUFF_DIR / 'verify_profiles.py'

def discrepancy_source_root(rule: str, profile: Path) -> Path:
    """Return the declared source of pinned evidence for a profile's rule."""
    return profile.parent

def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def atomic_json_write(path: Path, value: object) -> None:
    """Commit a machine-readable receipt by rename after its bytes reach disk."""
    temporary = path.with_name(f'.{path.name}.{os.getpid()}.tmp')
    with temporary.open('w', encoding='utf-8') as output:
        output.write(json.dumps(value, indent=2, ensure_ascii=False) + '\n')
        output.flush()
        os.fsync(output.fileno())
    temporary.replace(path)
