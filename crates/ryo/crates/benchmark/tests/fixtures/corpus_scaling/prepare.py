"""Prepare nested corpus requests for the existing run-ryo/collector protocol.

Preparation never executes a contestant. Reports must be admitted by the existing
verify_profiles.complete_run_ryo_report before canonical equality is compared.
"""
from pathlib import Path
import argparse
import hashlib
import json
import shutil


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def representative_order(inventory, anchors):
    """Interleave size strata and source families; keep largest/tail anchors first."""
    by_size = sorted(inventory, key=lambda row: (row["bytes"], row["file"]))
    groups = {}
    for rank, row in enumerate(by_size):
        family = row["file"].split("/", 1)[0]
        stratum = min(3, 4 * rank // len(by_size))
        groups.setdefault((family, stratum), []).append(row["file"])
    for key, files in groups.items():
        groups[key] = sorted(files, key=lambda file: hashlib.sha256(file.encode()).digest())
    order = list(dict.fromkeys(anchors))
    seen = set(order)
    while any(groups.values()):
        for key in sorted(groups):
            if groups[key]:
                file = groups[key].pop()
                if file not in seen:
                    order.append(file)
                    seen.add(file)
    assert len(order) == len(inventory) == len(seen)
    return order


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--frozen-corpus", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--request", type=Path, required=True)
    parser.add_argument("--baseline-worker", type=Path, required=True)
    parser.add_argument("--collector", type=Path, required=True)
    parser.add_argument("--validator", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(exist_ok=False)
    frozen = args.frozen_corpus.resolve()
    original = json.loads((frozen / "manifest.json").read_text())
    inventory = json.loads((frozen / "source-inventory.json").read_text())
    files = json.loads((frozen / "files.json").read_text())
    assert len(files) == 2875 and files == [row["file"] for row in inventory]
    assert digest(frozen / "source-inventory.json") == original["django"]["source_inventory_sha256"]
    assert digest(frozen / "files.json") == original["django"]["files_sha256"]
    for row in inventory:
        source = frozen / "corpus" / row["file"]
        assert source.is_file() and not source.is_symlink()
        assert source.stat().st_size == row["bytes"] and digest(source) == row["sha256"]
    profile = out / "profile"
    shutil.copytree(args.profile, profile)
    assert not any(path.is_symlink() for path in profile.rglob("*"))
    profile_inputs = {str(p.relative_to(profile)): digest(p) for p in sorted(profile.rglob("*")) if p.is_file()}
    request = json.loads(args.request.read_text())
    assert len(request["rule_applicability"]) == 310
    assert request["parser_configuration"] == "python-target=3.10"
    worker = json.loads(args.baseline_worker.read_text())
    assert digest(Path(worker["executable"])) == worker["sha256"]
    tails = ["tests/admin_views/tests.py", "tests/migrations/test_operations.py", "tests/schema/tests.py", "tests/forms_tests/tests/test_forms.py"]
    largest = [r["file"] for r in sorted(inventory, key=lambda r: (-r["bytes"], r["file"]))[:10]]
    anchors = tails + largest + files[:1] + files[-1:] + original["django"]["historical_2874_subset_difference"]
    order = representative_order(inventory, anchors)
    rows = {row["file"]: row for row in inventory}
    workloads = []
    for count, cap in [(100, 120), (500, 240), (1000, 360), (2875, 600)]:
        selected = order[:count]
        stage = out / f"files-{count}"
        request_at = dict(request, root=str(frozen / "corpus"), rules=str(profile / "rules"), files=selected, cache=str(stage / "runs/REPLACE_WITH_UNIQUE_INVOCATION/absent-cache"))
        write(stage / "request-template.json", request_at)
        write(stage / "files.json", selected)
        write(stage / "source-inventory.json", [rows[file] for file in selected])
        workloads.append({"files": count, "cap_seconds_per_process": cap, "request_template": str(stage / "request-template.json"), "request_template_sha256": digest(stage / "request-template.json"), "file_order_sha256": digest(stage / "files.json"), "source_inventory_sha256": digest(stage / "source-inventory.json"), "source_bytes": sum(rows[file]["bytes"] for file in selected), "selected_rule_file_pairs": count * 310})
    write(out / "plan.json", {"schema": "corpus-scaling-requests/v1", "status": "prepared-not-executed", "preparer_sha256": digest(Path(__file__)), "frozen_source_manifest_sha256": digest(frozen / "manifest.json"), "django_revision": original["django"]["revision"], "corpus_root": str(frozen / "corpus"), "source_inventory_sha256": digest(frozen / "source-inventory.json"), "profile_inputs": profile_inputs, "selection": list(request["rule_applicability"]), "rule_applicability": request["rule_applicability"], "query_modules": request["query_modules"], "parser_configuration": request["parser_configuration"], "project_context_mode": "isolated", "environment": {"RAYON_NUM_THREADS": "1"}, "benchmark_worker_pool": 1, "baseline": worker, "candidate": None, "collector": {"executable": str(args.collector.resolve()), "sha256": digest(args.collector)}, "validator": {"path": str(args.validator.resolve()), "sha256": digest(args.validator), "function": "complete_run_ryo_report"}, "selection_policy": "Nested prefixes of deterministic size-quartile/source-family interleave, with four measured tail files, ten largest files, lexical endpoints and known intentional-syntax fixture anchored in first100. Execution order is pinned; not a random population estimate.", "anchors": list(dict.fromkeys(anchors)), "workloads": workloads, "execution_policy": "Do not run baseline until candidate seal and small preflight succeed. Same request/profile/settings for both binaries, exact canonical report and coverage/skip equality required. Capped/signaled/partial reports remain censored, never equal-success. Preserve native wall/CPU/RSS, raw stdout/stderr, load/process overlap and all samples. Three alternating pairs only at affordable completed sizes; larger capped observations are not medians or slope evidence.", "cache_lifecycles": {"cold": "New process and unique absent enabled toolcache per invocation; OS cache uncontrolled.", "retained_tool_cache": "Existing CLI suite repeats separate processes against the same trial cache; exact cold/retained reports required, no inferred hits.", "edited_tool_cache": "Existing CLI suite compares both edited retained-cache contestants with fresh-cache executions; persistent transport is not required.", "same_session_warm": "Optional separate future metric, not a blocker for CLI retained-cache warm/edit; never infer from separate processes or warm OS pages."}})
    print(json.dumps({"plan": str(out / "plan.json"), "plan_sha256": digest(out / "plan.json"), "counts": [x["files"] for x in workloads]}))


if __name__ == "__main__":
    main()
