"""Replay unchanged case files against baseline and migrated queries; never run Ruff."""

from pathlib import Path
import argparse, hashlib, json, shutil, subprocess, tomllib, time, os

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--candidate-root", type=Path, required=True)
parser.add_argument("--baseline-root", type=Path, required=True)
parser.add_argument("--worker", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument(
    "--source-receipt",
    type=Path,
    default=Path(__file__).with_name("source-receipt.json"),
)
args = parser.parse_args()
root = args.output.resolve()
root.mkdir(exist_ok=False)
import importlib.util

spec = importlib.util.spec_from_file_location(
    "guard",
    args.candidate_root
    / "crates/ryo/languages/python/benchmarks/linters/ruff/verify_profiles.py",
)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)
guard.self_test_standard_suppression()
src = args.candidate_root.resolve()
canonical = args.baseline_root.resolve()
rel = Path("crates/ryo/languages/python/benchmarks/linters/ruff")
receipt_path = args.source_receipt.resolve()
receipt = json.loads(receipt_path.read_text())
worker = args.worker.resolve()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
(root / "validator-source.py").write_bytes(Path(guard.__file__).read_bytes())
phase_path = src / rel / "profiles/pinned-rule-phases.json"
phases = json.loads(phase_path.read_text())
shutil.copyfile(phase_path, root / "pinned-rule-phases.json")
settings = {}
for row in receipt["rows"]:
    rule = row["rule"]
    profile_path = src / rel / "profiles" / rule / "benchmark.toml"
    if not profile_path.is_file():
        profile_path = src / rel / "benchmark.toml"
    destination = root / "canonical-profiles" / (rule + ".toml")
    destination.parent.mkdir(exist_ok=True)
    shutil.copyfile(profile_path, destination)
    settings[rule] = {
        "profile_source": profile_path.relative_to(src).as_posix(),
        "profile_sha256": sha(profile_path),
        "effective": guard.canonical_run_ryo_settings(
            tomllib.loads(profile_path.read_text()), [rule], phases
        ),
    }
profiles = {}
for side in ("baseline", "candidate"):
    stage = root / side
    shutil.copytree(
        (canonical if side == "baseline" else src) / rel / "fragments",
        stage / "fragments",
        dirs_exist_ok=True,
    )
    registry = {
        "python/ruff/" + p.stem: "fragments/" + p.name
        for p in sorted((stage / "fragments").glob("*.ryoql"))
    }
    hashes = {k: sha(stage / v) for k, v in registry.items()}
    profiles[side] = {"path": stage, "registry": registry, "modules": hashes}
    for r in receipt["rows"]:
        target = stage / "single" / r["rule"] / "rules" / r["rule"]
        target.mkdir(parents=True, exist_ok=True)
        p = (canonical if side == "baseline" else src) / r["path"]
        assert sha(p) == r["before_sha256" if side == "baseline" else "after_sha256"], (
            r["rule"]
        )
        shutil.copyfile(p, target / "query.ryoql")
    # rules parent locates fragments
    for r in receipt["rows"]:
        link = stage / "single" / r["rule"] / "fragments"
        if link.is_symlink():
            link.unlink()
        shutil.copytree(stage / "fragments", link, dirs_exist_ok=True)
manifest = {
    "worker": str(worker),
    "worker_sha256": sha(worker),
    "source_receipt_sha256": sha(receipt_path),
    "suppress_sha256": profiles["candidate"]["modules"]["python/ruff/suppress"],
    "profiles": {
        s: {"module_registry": p["registry"], "module_sha256": p["modules"]}
        for s, p in profiles.items()
    },
    "validator_sha256": sha(Path(guard.__file__)),
    "rule_phase_registry_sha256": sha(phase_path),
    "canonical_settings": settings,
    "policy": "Exact current baseline vs migrated full-report equality. Existing cases expected diagnostics checked separately; no Ruff executable invoked.",
}
(root / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
failures = []
total = 0
expected_failures = []
started = time.monotonic()
output = root / "records.jsonl"
with output.open("w") as records:
    for index, r in enumerate(receipt["rows"]):
        rule = r["rule"]
        case_path = canonical / rel / "rules" / rule / "cases.toml"
        if not case_path.exists():
            raise RuntimeError(str(case_path))
        cases = tomllib.loads(case_path.read_text())["cases"]
        case_sha = sha(case_path)
        for profile in profiles.values():
            shutil.copyfile(
                case_path,
                profile["path"] / "single" / rule / "rules" / rule / "cases.toml",
            )
        assert len({case["name"] for case in cases}) == len(cases), (
            rule,
            "duplicate case identity",
        )
        for case in cases:
            corpus = root / "corpus"
            shutil.rmtree(corpus, ignore_errors=True)
            corpus.mkdir()
            for file, source in case["files"].items():
                p = corpus / file
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text(source)
            observed = {}
            for side, profile in profiles.items():
                request = {
                    "root": str(corpus),
                    "rules": str(profile["path"] / "single" / rule / "rules"),
                    "query_modules": profile["registry"],
                    "rule_applicability": settings[rule]["effective"][
                        "rule_applicability"
                    ],
                    "parser_configuration": settings[rule]["effective"][
                        "parser_configuration"
                    ],
                    "files": list(case["files"]),
                    "cache": None,
                }
                wire = json.dumps(request, sort_keys=True)
                before = time.monotonic()
                try:
                    p = subprocess.run(
                        [str(worker), "run-ryo", wire],
                        text=True,
                        capture_output=True,
                        timeout=20,
                        env=dict(os.environ, RAYON_NUM_THREADS="1"),
                    )
                except subprocess.TimeoutExpired:
                    raise RuntimeError((rule, case["name"], side, "timeout"))
                try:
                    report = json.loads(p.stdout) if p.returncode == 0 else None
                except json.JSONDecodeError:
                    report = None
                report = guard.complete_run_ryo_report(
                    p.returncode,
                    p.stdout,
                    p.stderr,
                    list(case["files"]),
                    [rule],
                    request=request,
                    settings=settings[rule]["effective"],
                )
                observed[side] = {
                    "exit": p.returncode,
                    "report": report,
                    "stderr": p.stderr,
                    "stdout": p.stdout,
                }
                row = {
                    "rule": rule,
                    "case": case["name"],
                    "side": side,
                    "case_file_sha256": case_sha,
                    "source_sha256": hashlib.sha256(
                        json.dumps(case["files"], sort_keys=True).encode()
                    ).hexdigest(),
                    "request": request,
                    "request_sha256": hashlib.sha256(wire.encode()).hexdigest(),
                    "elapsed_seconds": time.monotonic() - before,
                    **observed[side],
                }
                records.write(json.dumps(row, separators=(",", ":")) + "\n")
                records.flush()
            total += 1
            if observed["baseline"] != observed["candidate"]:
                failures.append(
                    {"rule": rule, "case": case["name"], "observed": observed}
                )
            report = observed["candidate"]["report"]
            if guard.diagnostic_multiset(
                report["diagnostics"]
            ) != guard.diagnostic_multiset(
                [{"rule": rule, **d} for d in case["expected"]]
            ):
                expected_failures.append(
                    {
                        "rule": rule,
                        "case": case["name"],
                        "expected": case["expected"],
                        "actual": report["diagnostics"],
                    }
                )
        print(
            f"{index + 1}/{len(receipt['rows'])} {rule} cases={len(cases)} cumulative={total} deltas={len(failures)} expected_differences={len(expected_failures)}",
            flush=True,
        )
summary = {
    "rules": len(receipt["rows"]),
    "cases": total,
    "side_records": total * 2,
    "report_deltas": failures,
    "existing_expected_differences": expected_failures,
    "elapsed_seconds": time.monotonic() - started,
    "records_sha256": sha(output),
    "manifest_sha256": sha(root / "manifest.json"),
}
(root / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(
    json.dumps(
        {
            k: v
            for k, v in summary.items()
            if k not in ("report_deltas", "existing_expected_differences")
        }
    ),
    flush=True,
)

assert not failures, "migration changed a complete report; inspect summary.json"

spec = importlib.util.spec_from_file_location(
    "migration_seal", Path(__file__).with_name("seal.py")
)
sealer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sealer)
sealer.seal(root, receipt_path)
