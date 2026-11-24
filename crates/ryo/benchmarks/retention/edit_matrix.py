"""Compare a retaining benchmark process with ordinary evaluation on isolated corpus copies.

Invocation: edit_matrix.py SPEC EVIDENCE. SPEC names the private root, request,
pack, executable, alternative build, command templates and edit fixtures.
Each template is an argv array, with {root}, {pack}, {request}, {paths},
{rules}, {exe} and {output} substituted without a shell.
"""
import argparse
import csv
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import sys
import tempfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


class Postcard:
    def __init__(self, data):
        self.data = memoryview(data)
        self.at = 0

    def take(self, count):
        end = self.at + count
        if end > len(self.data):
            raise ValueError("truncated findings table")
        part = self.data[self.at:end]
        self.at = end
        return part

    def uint(self):
        n = 0
        for shift in range(0, 70, 7):
            byte = self.take(1)[0]
            n |= (byte & 127) << shift
            if byte < 128:
                return n
        raise ValueError("invalid postcard integer")

    def string(self):
        return bytes(self.take(self.uint())).decode()


def inspect_table(data, output, diagnostics, admitted):
    if data is None:
        return 0
    reader = Postcard(data)
    reader.take(64)  # payload checksum and linker identity
    count = reader.uint()
    public = {tuple(sorted(json.loads(line).items())) for line in output.splitlines() if line}
    origins = set()
    outcomes = {}
    admitted_set = set(admitted)
    for _ in range(count):
        reader.take(16)
        refusal = reader.string() if reader.take(1)[0] else None
        if refusal is not None:
            assert refusal in diagnostics
        origin = reader.string()
        assert origin in admitted_set and origin not in origins
        origins.add(origin)
        findings, refusals = [], []
        outcomes[origin] = findings, refusals
        for _ in range(reader.uint()):
            rule, subject = reader.string(), reader.string()
            start, end = reader.uint(), reader.uint()
            row = dict(rule=rule, file=subject, start=start, end=end)
            findings.append((rule, tuple(sorted(row.items()))))
        for _ in range(reader.uint()):
            emit, reason = reader.string(), reader.string()
            assert emit and reason
            refusals.append((emit, reason))
        for _ in range(reader.uint()):
            reader.string()
            if reader.take(1)[0]:
                reader.string()
            reader.string()
            origin_kind = reader.uint()
            assert origin_kind <= 3
            for _ in range((0, 1, 1, 2)[origin_kind]):
                reader.uint()
    assert reader.at == len(reader.data)
    first = {}
    visible = set()
    for origin in admitted:
        if origin not in outcomes:
            continue
        findings, refusals = outcomes[origin]
        visible.update(row for emit, row in findings if emit not in first)
        for emit, reason in refusals:
            first.setdefault(emit, f"FAILED {emit}: {origin}: {reason}")
    assert visible <= public, "stored findings include a result absent from cold"
    if count == len(admitted):
        assert visible == public, "complete snapshot omitted a finding"
        assert sorted(first.values()) == sorted(line for line in diagnostics if line.startswith("FAILED "))
    return count


def run(spec, evidence):
    request = Path(spec["request"])
    root = Path(spec["root"])
    pack = Path(spec["pack"])
    # This driver edits only a caller-supplied disposable corpus.
    assert (root.parent / "prepared").is_file()
    original = request.read_bytes()
    base = json.loads(original)
    edit = root / spec["edit"]
    original_stat = edit.stat()
    previous = edit.read_bytes()
    program = pack / "rules" / base["selected_rules"][0] / "query.ryoql"
    rule = program.read_bytes()
    added = edit.with_name("retention-added" + edit.suffix)
    settings = root / "retention-test.settings"
    assert not added.exists() and not settings.exists()
    store = root / ".ryo_cache" / "findings.postcard"
    evidence.mkdir(exist_ok=True, parents=True)

    with tempfile.TemporaryDirectory() as tmp:
        scratch = Path(tmp)
        paths = scratch / "paths"

        hits = {}

        def invoke(mode, j, exe, tag):
            paths.write_text("".join(str(root / f) + "\n" for f in j["files"]))
            output = scratch / (tag + ".out")
            output.unlink(missing_ok=True)
            fields = dict(root=root, pack=pack, request=request, paths=paths,
                          rules=",".join(j["selected_rules"]), exe=exe, output=output)
            argv = [arg.format_map(fields) for arg in spec[mode]]
            child = subprocess.run(argv, cwd=spec.get("cwd"), capture_output=True,
                                   env=dict(os.environ, **spec.get("environment", {})))
            match = re.search(rb"retained hits=(\d+)", child.stderr)
            hits[tag] = int(match[1]) if match else 0
            diagnostics = [s for s in (child.stdout + child.stderr).decode(errors="replace").splitlines()
                           if s.startswith(("FAILED ", "project-scoped ", "no catalog declaration ", "provider does not yet read "))]
            result = output.read_bytes() if output.exists() else b""
            return child.returncode, result, diagnostics

        def snap():
            return store.read_bytes() if store.exists() else None

        def restore_store(data):
            if data is None:
                store.unlink(missing_ok=True)
                if store.parent.exists():
                    store.parent.rmdir()
            else:
                store.parent.mkdir(exist_ok=True)
                store.write_bytes(data)

        def reset():
            edit.write_bytes(previous)
            os.utime(edit, ns=(original_stat.st_atime_ns, original_stat.st_mtime_ns))
            added.unlink(missing_ok=True)
            settings.unlink(missing_ok=True)
            program.write_bytes(rule)
            request.write_bytes(original)

        passed = True
        original_store = snap()
        try:
            restore_store(None)
            invoke("retained", base, spec["exe"], "seed")
            seed = snap()
            for scenario in ("unchanged","edit","finding-edit","same-stamp","add","delete","manifest-settings","rule-pack","build","refused","refused-reverse","refused-edit","refused-input","truncated"):
                reset()
                restore_store(seed)
                exe = spec["exe"]
                j = json.loads(original)
                if scenario == "edit":
                    edit.write_bytes(previous + spec["comment"].encode())
                elif scenario == "finding-edit":
                    edit.write_bytes(spec["replacement"].encode())
                elif scenario == "same-stamp":
                    stat = edit.stat()
                    edit.write_bytes(bytes(byte if byte in (10, 13) else 32 for byte in previous))
                    os.utime(edit, ns=(stat.st_atime_ns, stat.st_mtime_ns))
                elif scenario == "add":
                    added.write_bytes(previous)
                    relative = str(added.relative_to(root))
                    j["files"].append(relative)
                    if spec["edit"] in j.get("file_rules", {}):
                        j["file_rules"][relative] = j["file_rules"][spec["edit"]]
                elif scenario == "delete":
                    edit.unlink()
                    j["files"].remove(spec["edit"])
                    j.get("file_rules", {}).pop(spec["edit"], None)
                elif scenario == "manifest-settings":
                    settings.write_text("changed\n")
                    j.update(spec["settings"])
                elif scenario == "rule-pack":
                    program.write_bytes(rule+b"\n")
                elif scenario == "build":
                    exe = spec["other_build"]
                elif scenario in ("refused", "refused-reverse", "refused-edit", "refused-input"):
                    program.write_text(spec["refused_query"])
                    refusing = invoke("retained", j, exe, "refusing-seed")
                    if scenario == "refused-reverse":
                        j["files"].reverse()
                    elif scenario == "refused-edit":
                        # Keep the refusing plan fixed and change the effective bytes of its first input.
                        first = root / j["files"][0]
                        first_bytes = first.read_bytes()
                        first_stamp = first.stat()
                        first.write_bytes(first_bytes + spec["comment"].encode())
                    elif scenario == "refused-input":
                        # Change the provider relation the previously refusing emit requests.
                        program.write_bytes(rule)
                elif scenario == "truncated" and seed:
                    store.write_bytes(seed[:len(seed)//2])
                if j != base:
                    request.write_text(json.dumps(j))
                before = snap()
                stamp = store.stat() if before is not None else None
                trial = invoke("retained", j, exe, "trial")
                after = store.stat() if store.exists() else None
                repeated = invoke("retained", j, exe, "again") if scenario.startswith("refused") else trial
                stored = snap()
                restore_store(None)
                cold = invoke("cold", j, exe, "cold")
                assert not store.parent.exists(), "disabled store wrote state"
                seeded = invoke("retained", j, exe, "oracle")
                reference = snap()
                errors = []
                if cold[0] not in (0, 1):
                    errors.append("cold did not finish normally")
                if scenario != "refused" and not cold[2] and cold[0] == 0 and not stored:
                    errors.append("completed corpus did not leave a snapshot")
                if before == stored and stamp and after and (stamp.st_ino, stamp.st_mtime_ns) != (after.st_ino, after.st_mtime_ns):
                    errors.append("unchanged table was rewritten")
                if trial != cold or seeded != cold or repeated != cold:
                    errors.append("cold output, status or refusals differ")
                if stored != reference:
                    errors.append("snapshot differs from a fresh evaluation")
                if scenario == "refused-input" and cold[2] == refusing[2]:
                    errors.append("the changed provider request did not change its refusal")
                if scenario == "refused" and not cold[2] and cold[0] == 0:
                    errors.append("fixture did not refuse")
                if scenario in ("finding-edit", "same-stamp") and cold[1] == unchanged_output:
                    errors.append("edit did not change a finding")
                if scenario == "unchanged":
                    unchanged_output = cold[1]
                try:
                    rows = inspect_table(stored, cold[1], cold[2], j["files"])
                    if scenario == "refused-reverse" and spec.get("lang") == "python" and rows:
                        if hits["trial"] != rows:
                            errors.append("reverse traversal did not replay stable outcomes")
                    if scenario == "refused-edit" and spec.get("lang") == "python" and rows:
                        if hits["trial"] != rows - 1 or not cold[2]:
                            errors.append("changed refusing input must miss once and retain the diagnostic")
                    if scenario == "unchanged" and cold[0] == 0 and j["files"]:
                        if not rows or not hits["trial"]:
                            errors.append("no completed file reused on an unchanged successful corpus")
                except (ValueError, AssertionError) as failure:
                    rows = -1
                    errors.append(repr(failure))
                if scenario == "refused-edit":
                    first.write_bytes(first_bytes)
                    os.utime(first, ns=(first_stamp.st_atime_ns, first_stamp.st_mtime_ns))
                passed &= not errors
                with (evidence / "matrix.tsv").open("a") as log:
                    writer=csv.writer(log,delimiter="\t",lineterminator="\n")
                    if not log.tell():
                        writer.writerow(["project","scenario","status","rc","diagnostics","cold_output_B","warm_output_B","cold_sha","warm_sha","store_B","fresh_B","rows","reason"])
                    writer.writerow([spec["name"],scenario,"FAIL" if errors else "PASS",trial[0],len(trial[2]),len(cold[1]),len(trial[1]),digest(cold[1]),digest(trial[1]),len(stored or b""),len(reference or b""),rows," | ".join(errors)])
        finally:
            reset()
            restore_store(original_store)
    return passed


# Prepared corpus directories have root/, pack/, request.json and a prepared marker.
PROJECTS = {
    'clap': ('rust', 'clap_builder/src/util/flat_map.rs'),
    'dagster': ('python', 'python_modules/dagster/dagster/_core/definitions/asset_key.py'),
    'django': ('python', 'tests/asgi/tests.py'),
    'fastapi': ('python', 'docs_src/query_params_str_validations/tutorial012_an_py39.py'),
    'hono': ('typescript', 'benchmarks/routers-deno/src/medley-router.mts'),
    'pandas': ('python', 'pandas/core/arrays/categorical.py'),
    'py-cases': ('python', 'ASYNC100/0000/case.py'),
    'ripgrep': ('rust', 'crates/core/messages.rs'),
    'rs-cases': ('rust', 'src/almost_complete_range/0/lib.rs'),
    'ts-cases': ('typescript', 'eslint.constructor-super/106/case.ts'),
    'ts-frontend': ('typescript', 'eslint.no-debugger/23/case.js'),
    'vite': ('typescript', 'packages/vite/src/node/ssr/runtime/__tests__/fixtures/live-binding/test4/dep.js'),
}


def project_spec(name, directory, executable, other_build):
    lang, edit = PROJECTS[name]
    root = directory / "root"
    request = directory / "request.json"
    rules = json.loads(request.read_bytes())["selected_rules"]
    python = lang == "python"
    cold = (
        "{exe} funnel spec {paths} {pack} unused --facts --remap --direct --hasher fx --regex-set "
        "--words table --word-skip --finish {pack} {root} --findings-out {output} "
        "--applicability {request} --finish-rules {rules} --request {request}" if python
        else "{exe} {pack} {request} {output}"
    )
    return dict(
        name=name, root=str(root), request=str(request), pack=str(directory / "pack"),
        edit=edit, exe=str(executable), other_build=str(other_build),
        cold=cold.split(), retained="{exe} --retain-findings {pack} {request} {output}".split(),
        comment="\n" + ("#" if python else "//") + " retained inspection\n", replacement="\n",
        settings={"parser_configuration": "python-target=3.11"} if python else {"selected_rules": rules[1:]},
        environment=dict(RAYON_NUM_THREADS="1", **(dict(VEC_THREAD_CLOCK="1", VEC_PY310="1",
                         VEC_COLWALK="1", VEC_STATIC_KEYED="1") if python else {})),
        refused_query=('let diagnostics = select d.file as file, d.span as span from "manifest-diagnostic" '
                       'as d where d.schema = "python-project-metadata";\nemit diagnostics subject file key(file);\n'),
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("spec", type=Path, help="JSON spec, or prepared corpus directory with --project")
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--project", choices=PROJECTS)
    parser.add_argument("--executable", type=Path)
    parser.add_argument("--other-build", type=Path)
    args = parser.parse_args()
    if args.project:
        if args.executable is None or args.other_build is None:
            parser.error("--project requires --executable and --other-build")
        spec = project_spec(args.project, args.spec.resolve(), args.executable.resolve(), args.other_build.resolve())
    else:
        spec = json.loads(args.spec.read_bytes())
    sys.exit(0 if run(spec, args.evidence) else 1)
