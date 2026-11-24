"""Cold check and replay of independent copies in one process and one analysis worker."""
import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path

from measure import SESSION_PARTS, process, retained_files

SIZES = (1, 2, 4)


def copy(source, dest):
    subprocess.run(["cp", "-a", "--reflink=auto", source, dest], check=True)


def prepare(args):
    out = args.directory.resolve()
    out.mkdir(parents=True, exist_ok=True)
    live = args.live_tree.resolve()
    repo = Path(subprocess.check_output(
        ["git", "-C", live, "rev-parse", "--show-toplevel"], text=True).strip())
    relative = live.relative_to(repo)
    authority = json.loads(args.authorities.read_bytes())["roots"]
    source_parents = {Path(root).parent for root in authority}
    if len(source_parents) != 1:
        raise ValueError("the captured input roots must share one fixture directory")
    source = source_parents.pop()
    invocations = [line.split("\t") for line in args.manifest.read_text().splitlines()]
    reports_by_size = {n: [] for n in SIZES}
    roots = {}
    manifests = {n: (out / f"replay-{n}.tsv").open("w") for n in SIZES}
    try:
        for ordinal in range(max(SIZES)):
            replica = out / f"copy-{ordinal}"
            replica.mkdir()
            checkout = replica / "worktree"
            subprocess.run(["git", "clone", "-q", "--shared", "--no-checkout", repo, checkout],
                           check=True)
            subprocess.run(["git", "-C", checkout, "read-tree", "HEAD"], check=True)
            for item in repo.iterdir():
                if item.name not in (".git", ".tastemaker_cache"):
                    copy(item, checkout / item.name)
            inputs = replica / "inputs"
            copy(source, inputs)
            requests = replica / "requests"
            requests.mkdir()
            copied = {}
            for root, policy in authority.items():
                dest = inputs / Path(root).name
                if isinstance(policy, dict):
                    disk = replica / "authority" / Path(root).name
                    disk.parent.mkdir(exist_ok=True)
                    copy(Path(policy["Disk"]), disk)
                    policy = {"Disk": str(disk)}
                roots[str(dest)] = policy
            for request_id, pack, request, expected, *inventory in invocations:
                if request not in copied:
                    value = json.loads(Path(request).read_bytes())
                    original = Path(value["root"])
                    if original.parent != source or str(original) not in authority:
                        raise ValueError(f"unknown captured root {original}")
                    value["root"] = str(inputs / original.name)
                    new = requests / Path(request).name
                    new.write_text(json.dumps(value, separators=(",", ":")))
                    copied[request] = new
                line = "\t".join(map(str, (f"{ordinal}:{request_id}", pack, copied[request],
                                           expected, *inventory)))
                for n, manifest in manifests.items():
                    if ordinal < n:
                        print(line, file=manifest)
            for n in SIZES:
                if ordinal < n:
                    reports_by_size[n].append(str(checkout / relative))
            for n in SIZES:
                if ordinal + 1 == n:
                    (out / f"authorities-{n}.json").write_text(json.dumps({"roots": roots}))
    finally:
        for stream in manifests.values():
            stream.close()
    (out / "live-roots.json").write_text(json.dumps(reports_by_size))


def reading(path):
    return json.loads(path.read_text())


def reports(text):
    decoder = json.JSONDecoder()
    text = text.lstrip()
    while text:
        result, end = decoder.raw_decode(text)
        yield result
        text = text[end:].lstrip()


def normalized(report, root):
    # Keep every SARIF field, including ordering, except the replica path.
    return json.loads(json.dumps(report).replace(str(root.parent.parent), "$REPOSITORY"))


def run(args):
    n = args.copies
    if args.budget <= 0:
        raise ValueError("a positive caller budget is required")
    scratch = args.directory.resolve()
    target = args.output.resolve()
    target.mkdir(parents=True, exist_ok=True)
    prefix = target / f"{args.mode}-{n}"
    exe = args.worker.resolve()
    env = dict(os.environ, RAYON_NUM_THREADS="1", TM_BENCH_MEMORY_BYTES=str(args.budget))
    roots = [Path(p) for p in reading(scratch / "live-roots.json")[str(n)]]
    report_path = prefix.with_suffix(".out")
    log = prefix.with_suffix(".log")
    stdout_path = prefix.with_suffix(".stdout")
    stores = []
    if args.mode == "live":
        env["TM_BENCH_MULTIPLE_ROOTS"] = "1"
        stores = [path / ".tastemaker_cache" for root in roots for path in (root, root.parent.parent)]
        for path in stores:
            if path.exists():
                raise ValueError(f"cold check would use an existing store at {path}")
        command = [exe, "--tm-check-roots", *roots]
    else:
        env["TM_CORPUS_PROJECTS"] = str(scratch / f"authorities-{n}.json")
        command = [exe, "--tm-batch", scratch / f"replay-{n}.tsv", report_path]
    if args.heap:
        symbols = subprocess.check_output(["nm", "--defined-only", exe], text=True)
        ctl = next(line.split()[0] for line in symbols.splitlines() if line.endswith(" _rjem_mallctl"))
        env.update(MAP_CTL=ctl, LD_PRELOAD=str(args.heap.resolve()), MAP_HEAP=str(prefix.with_suffix(".heap")))
    try:
        with log.open("w+b") as stderr, stdout_path.open("wb") as stdout:
            measured = process(command, cwd=roots[0], stdout=stdout, stderr=stderr, env=env,
                               allowed=(0, 1) if args.mode == "live" else (0,))
        if args.mode == "live":
            measured["store_bytes"] = sum(retained_files(path)[0] for path in stores if path.exists())
    finally:
        for path in stores:
            if path.exists():
                shutil.rmtree(path)
    lines = (log if args.mode == "live" else stdout_path).read_text().splitlines()
    key = "CHECK_SESSION live " if args.mode == "live" else "CHECK_SESSION "
    line = next(s for s in lines if s.startswith(key))
    fields = dict(item.split("=", 1) for item in line.split() if "=" in item)
    measured["parts_cpu_s"] = {part: float(fields[part]) for part in SESSION_PARTS}
    measured["ryo_cpu_s"] = sum(measured["parts_cpu_s"].values())
    measured.update(copies=n, budget=args.budget, mode=args.mode)
    measured["counters"] = {key: int(value) for key, value in fields.items() if value.isdecimal()}
    measured["output_bytes"] = (stdout_path if args.mode == "live" else report_path).stat().st_size
    if args.mode == "live":
        result = list(reports(stdout_path.read_text()))
        expected = int(args.requests) * n
        assert len(result) == n and int(fields["requests"]) == expected
        assert int(fields["failed_calls"]) == 0, line
        # Copies have distinct Git roots; compare the full SARIF including suppressions.
        first = args.live_oracle or target / "live-oracle.json"
        candidate = [normalized(report, root) for root, report in zip(roots, result)]
        if n == 1 and args.live_oracle is None:
            first.write_text(json.dumps(candidate[0]))
        assert all(report == reading(first) for report in candidate), "replica SARIF differs"
        measured["requests"] = expected
    else:
        count = 0
        with report_path.open("rb") as observed:
            for ordinal in range(n):
                copied = f"{ordinal}:".encode()
                with args.receipts.open("rb") as original:
                    for expected in original:
                        answer = observed.readline()
                        if answer != copied + expected:
                            raise ValueError(f"receipt differs at copy {ordinal}, request {count}: {answer[:80]!r}")
                        count += 1
            if observed.read(1):
                raise ValueError("extra receipts after the final copy")
        measured["requests"] = count
    encoded = json.dumps(measured)
    prefix.with_suffix(".json").write_text(encoded + "\n")
    print(encoded)


def gate(args):
    failures = []
    for mode in ("live", "replay"):
        rows = [reading(args.output / f"{mode}-{n}.json") for n in SIZES]
        assert len({r["budget"] for r in rows}) == 1 and all(r["workers"] == 1 for r in rows)
        assert [r["copies"] for r in rows] == list(SIZES)
        assert [r["requests"] for r in rows] == [rows[0]["requests"] * n for n in SIZES]
        start, end = rows[0]["peak_mib"], rows[-1]["peak_mib"]
        tolerance = max(1, start * 0.005)
        if end > start + tolerance:
            failures.append(f"{mode} fails at 4x: {end:.1f} MiB versus {start:.1f} MiB")
    if failures:
        raise SystemExit("\n".join(failures))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prep = commands.add_parser("prepare")
    for name in ("directory", "live-tree", "manifest", "authorities"):
        prep.add_argument(f"--{name}", type=Path, required=True)
    probe = commands.add_parser("run")
    for name in ("directory", "worker", "output", "receipts"):
        probe.add_argument(f"--{name}", type=Path, required=True)
    probe.add_argument("--mode", choices=("live", "replay"), required=True)
    probe.add_argument("--copies", type=int, choices=SIZES, required=True)
    probe.add_argument("--budget", type=int, required=True)
    probe.add_argument("--requests", type=int, default=46831)
    probe.add_argument("--heap", type=Path)
    probe.add_argument("--live-oracle", type=Path)
    check = commands.add_parser("gate")
    check.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    {"prepare": prepare, "run": run, "gate": gate}[args.command](args)


if __name__ == "__main__":
    main()
