"""Compare an isolated, editable full-session capture with fresh evaluation.

The fixture directory has manifest.tsv, edit.json and authorities.json. edit.json
names a private disk root, its SDK snapshot root, an admitted Rust source, the
request files and inventories whose admitted membership can change, and a private
query. The shipped capture and product checkout are never edited.
"""
import argparse
import csv
import hashlib
import json
import os
import re
import runpy
import shutil
import subprocess
import tempfile
from pathlib import Path


def parent_lookup(executable, scratch, env):
    """A new module satisfies a lookup and removes a finding on its unchanged parent."""
    root=scratch/"lookup"
    pack=root/"pack/rules/query"
    pack.mkdir(parents=True)
    (pack/"query.ryoql").write_text(
        'let listed = select l."path" as "file" from "filesystem.file" as l;\n'
        'let parent = select * from listed where glob_any(file, "src/lib.rs");\n'
        'let child = select * from listed where glob_any(file, "src/child.rs");\n'
        'let needs = extend(from=parent, values={"witness": 1});\n'
        'let has = extend(from=child, values={"witness": 1});\n'
        'let missing = antijoin(left=needs,right=has,left-on=["witness"],right-on=["witness"]);\n'
        'emit missing subject file key(file);\n'
    )
    (root/"src").mkdir()
    (root/"src/lib.rs").write_text("pub mod child;\n")
    manifest=root/"manifest.tsv"
    req=root/"req"
    inventory=root/"inventory"
    manifest.write_text(f"lookup\t{pack.parents[1]}\t{req}\tunused\t{inventory}\n")
    baseline=None
    for present in (False,True,False):
        child=root/"src/child.rs"
        if present:child.write_text("pub fn child() {}\n")
        else:child.unlink(missing_ok=True)
        files=["src/lib.rs"]+(["src/child.rs"] if present else [])
        inventory.write_text(json.dumps(files))
        req.write_text(json.dumps(dict(root=str(root),files=files,parameters={},hosted=[])))
        results=[]
        for args in (["--tm-batch-retained",root],["--tm-batch"]):
            out=root/"receipt"
            subprocess.run([executable,*args,manifest,out],env=env,stdout=subprocess.DEVNULL,check=True)
            results.append(out.read_bytes())
        assert results[0]==results[1] and b"refused" not in results[0]
        if baseline is None:baseline=results[0]
        assert (results[0]!=baseline)==present


def typed_packets(cache):
    """Read active request references; compare complete typed bytes, including order."""
    reader_path=Path(__file__).resolve().parents[3]/"ryo/benchmarks/retention/edit_matrix.py"
    reader=runpy.run_path(str(reader_path))["Postcard"]
    packets={}
    for path in cache.glob("answers-*.postcard"):
        r=reader(path.read_bytes())
        for _ in range(r.uint()):
            key=bytes(r.take(32))
            digest=r.string()
            unknowns=[(r.string(),r.string()) for _ in range(r.uint())]
            payload=bytes(r.take(r.uint()))
            packets[key]=(digest,unknowns,payload)
        assert r.at==len(r.data)
    active={}
    for path in cache.glob("requests-*.postcard"):
        r=reader(path.read_bytes())
        for _ in range(r.uint()):
            key=bytes(r.take(32))
            r.take(64*r.uint())
            result=packets[bytes(r.take(32))]
            unread=[r.string() for _ in range(r.uint())]
            active[key]=(result,unread)
        assert r.at==len(r.data)
    return active


def contribution_edits(executable, scratch, env):
    """An edit changes a local multiset and an untouched parent's closed answer."""
    root = scratch / "contributions"
    pack = root / "pack/rules/query"
    pack.mkdir(parents=True)
    src = root / "src"
    src.mkdir()
    parent, child = "src/parent.py", "src/child.py"
    (root / parent).write_text("def parent(): return 1\ndef member(): return 1\n")
    local = 'let nodes = select id, file, span from "semantic-syntax" where kind = "function-def";\nemit nodes subject file key(id, span);\n'
    closed = (
        'let listed = select l."path" as "file" from "filesystem.file" as l;\n'
        'let target = select * from listed where glob_any(file, "src/parent.py");\n'
        'let nodes = select file from "semantic-syntax" where kind = "function-def" and glob_any(file, "src/child.py");\n'
        'let needs = extend(from=target, values={"witness": 1});\n'
        'let has = extend(from=nodes, values={"witness": 1});\n'
        'let missing = antijoin(left=needs,right=has,left-on=["witness"],right-on=["witness"]);\n'
        'emit missing subject file key(file);\n'
    )
    (pack / "query.ryoql").write_text(local)
    second = root / "closed/rules/query"
    second.mkdir(parents=True)
    (second / "query.ryoql").write_text(closed)
    req = root / "request"
    inventory = root / "inventory"
    manifest = root / "manifest"
    manifest.write_text("".join(f"{i}\t{directory}\t{req}\tunused\t{inventory}\n"
                                for i,directory in [("local",pack.parents[1]),("closed",second.parents[1])]))
    result = {}
    for label,files in [("forward",[parent,child]),("reverse",[child,parent])]:
        req.write_text(json.dumps(dict(root=str(root),files=files,parameters={},hosted=[])))
        inventory.write_text(json.dumps(files))
        cache=root/label
        for step,source in enumerate(["pass\n","def child(): return 2\n"]):
            (root/child).write_text(source)
            reports=[]
            for mode in (["--tm-batch-retained",cache],["--tm-batch"],["--tm-batch-retained",cache]):
                out=root/"out"
                p=subprocess.run([executable,*mode,manifest,out],env=env,capture_output=True,check=True)
                assert b"refused" not in out.read_bytes(),p.stdout
                reports.append(out.read_bytes().splitlines())
            assert reports[0]==reports[1]==reports[2]
            assert b"evaluated_pairs=0 evaluated_closed=0 written=0" in p.stderr
            fresh=root/f"fresh-{label}-{step}"
            subprocess.run([executable,"--tm-batch-retained",fresh,manifest,root/"fresh-receipt"],
                           env=env,capture_output=True,check=True)
            warm_packets=typed_packets(cache/".ryo_cache")
            fresh_packets=typed_packets(fresh/".ryo_cache")
            assert len(warm_packets)==2 and warm_packets==fresh_packets
            if step==0: baseline=reports[0]
            else:
                assert all(a!=b for a,b in zip(baseline,reports[0])),(label,baseline,reports[0])
                result[label]=reports[0]
    assert result["forward"]==result["reverse"]
    return result["forward"]


def run(fixture, exe, other_build, evidence):
    spec = json.loads((fixture / "edit.json").read_bytes())
    root = fixture / spec["root"]
    disk = fixture / spec["disk"]
    rel = Path(spec["source"])
    originals = {}
    source = root / rel
    extra = rel.with_name("warm_matrix_added.rs")
    manifest = fixture / "manifest.tsv"
    store_root = fixture / "store"
    store_root.mkdir(exist_ok=True)
    cache = store_root / ".ryo_cache"
    assert root.is_relative_to(fixture) and disk.is_relative_to(fixture)
    assert not cache.exists(), "use a fresh store directory for this matrix"
    output = fixture / "receipt"
    evidence.mkdir(exist_ok=True, parents=True)
    env = dict(os.environ, TM_CORPUS_PROJECTS=str(fixture / "authorities.json"), RAYON_NUM_THREADS="1")

    def write(path, value):
        assert path.resolve().is_relative_to(fixture)
        if path not in originals:
            originals[path] = (path.read_bytes(), path.stat()) if path.exists() else None
        if value is None:
            path.unlink(missing_ok=True)
        else:
            path.write_bytes(value)

    def change(files, altered):
        for path in files:
            write(path, altered(path))

    def membership(name, add):
        def alter(j):
            files = j if isinstance(j, list) else j["files"]
            if add:
                if str(rel) in files and name not in files:
                    files.append(name)
            elif name in files:
                files.remove(name)
            return json.dumps(j).encode()
        for name_of_file in spec["requests"] + spec["inventories"]:
            path = fixture / name_of_file
            write(path, alter(json.loads(path.read_bytes())))

    def reset():
        for path, info in originals.items():
            if info is None:
                path.unlink(missing_ok=True)
            else:
                data, stat = info
                path.write_bytes(data)
                os.utime(path, ns=(stat.st_atime_ns,stat.st_mtime_ns))
        originals.clear()

    counters = {}

    def invoke(binary, retain, directory=store_root):
        output.unlink(missing_ok=True)
        args = [binary, "--tm-batch-retained", directory] if retain else [binary, "--tm-batch"]
        p = subprocess.run([*args,manifest,output],capture_output=True,env=env)
        diagnostics = sorted(x for x in p.stdout.splitlines() if x.startswith(b"REFUSAL"))
        if p.returncode:
            raise RuntimeError((p.returncode,p.stderr[-1200:]))
        if retain:
            counters.clear()
            counters.update((k.decode(), int(v)) for k, v in
                            re.findall(rb"(request_hits|pair_hits|closed_hits|evaluated_pairs|evaluated_closed|written)=(\d+)", p.stderr))
            assert "written" in counters, "retained run did not report its publication"
        return output.read_bytes(), diagnostics

    with tempfile.TemporaryDirectory(dir=fixture) as tmp:
        parent_lookup(exe,Path(tmp),env)
        rows=contribution_edits(exe,Path(tmp),env)
        with (evidence/"contribution-matrix.tsv").open("w") as f:
            f.write("scenario\tstatus\tchanged_receipt\n")
            for name,row in [("several-file",rows[0]),("closed-contribution",rows[1]),("same-edit-forward",rows[0]),("same-edit-reversed",rows[1])]:
                f.write(f"{name}\tPASS\t{row.decode()}\n")
        backup = Path(tmp) / "seed"
        original = invoke(exe, True)
        seeded = counters.copy()
        assert original == invoke(exe, False)
        shutil.copytree(cache, backup)
        passed = True
        try:
            for case in ("unchanged","comment","answer-change","same-stamp","add","delete",
                         "manifest","host-table","rule","build","refusal","truncated"):
                bin_path = other_build if case == "build" else exe
                shutil.rmtree(cache)
                shutil.copytree(backup,cache)
                sources = [root / rel, disk / rel]
                if case == "comment":
                    change(sources,lambda p:p.read_bytes()+b"\n// retained inspection\n")
                elif case == "answer-change":
                    change(sources,lambda _:b"pub fn retained_changed() {}\n")
                elif case == "same-stamp":
                    for path in sources:
                        prior=path.stat()
                        write(path,bytes(b if b in (10,13) else 32 for b in path.read_bytes()))
                        os.utime(path,ns=(prior.st_atime_ns,prior.st_mtime_ns))
                elif case == "add":
                    for directory in (root,disk):
                        added=directory/extra
                        assert not added.exists()
                        write(added,b"pub fn added() {}\n")
                    membership(str(extra),True)
                elif case == "delete":
                    for path in sources:
                        write(path,None)
                    membership(str(rel),False)
                elif case == "manifest":
                    change([root / "Cargo.toml",disk / "Cargo.toml"],lambda p:p.read_bytes()+b"\n[broken\n")
                elif case == "host-table":
                    path=fixture/spec["host_request"]
                    j=json.loads(path.read_bytes())
                    for table in j["hosted"]:
                        if table["rows"]:
                            table["rows"]=[]
                            break
                    else:
                        raise AssertionError("fixture needs a populated host table")
                    write(path,json.dumps(j).encode())
                elif case in ("rule","refusal"):
                    query=fixture/spec["query"]
                    write(query,b"not a query!\n" if case=="refusal" else query.read_bytes()+b"\n")
                elif case=="truncated":
                    pair=cache/"session.postcard"
                    with pair.open("r+b") as f:
                        f.truncate(pair.stat().st_size//2)
                warm=invoke(bin_path,True)
                trial=counters.copy()
                cold=invoke(bin_path,False)
                hit=invoke(bin_path,True)
                with tempfile.TemporaryDirectory(dir=fixture) as fresh_root:
                    reference=invoke(bin_path,True,Path(fresh_root))
                    truth=typed_packets(Path(fresh_root)/".ryo_cache")
                    observed=typed_packets(cache)
                    typed_equal=all(observed.get(k)==v for k,v in truth.items())
                ok=warm==cold==hit==reference and typed_equal and bool(truth)
                if case in ("answer-change","same-stamp","delete","host-table"):
                    ok &= cold[0]!=original[0]
                if case=="host-table":
                    changed=sum(a!=b for a,b in zip(cold[0].splitlines(),original[0].splitlines()))
                    ok &= 0 < changed < len(cold[0].splitlines()) and trial["request_hits"]>0
                if case=="refusal":
                    ok &= bool(cold[1])
                if case == "unchanged":
                    ok &= trial["written"] == trial["evaluated_pairs"] == trial["evaluated_closed"] == 0
                if case in ("build","truncated"):
                    ok &= trial["written"] > 0 and trial["evaluated_pairs"] == seeded["evaluated_pairs"]
                passed &= ok
                with (evidence/"session-matrix.tsv").open("a") as f:
                    w=csv.writer(f,delimiter="\t",lineterminator="\n")
                    if f.tell()==0:
                        w.writerow(["scenario","pass","bytes","warm_sha256","cold_sha256","refusals","pairs_scheduled","closed_scheduled","written_B","typed_requests_checked","typed_equal"])
                    w.writerow([case,ok,len(cold[0]),hashlib.sha256(warm[0]).hexdigest(),
                                hashlib.sha256(cold[0]).hexdigest(),len(cold[1]),
                                trial["evaluated_pairs"],trial["evaluated_closed"],trial["written"],len(truth),typed_equal])
                reset()
        finally:
            reset()
        return passed


if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    for arg in ("fixture","executable","other_build","evidence"):
        parser.add_argument(arg,type=Path)
    args=parser.parse_args()
    raise SystemExit(int(not run(args.fixture.resolve(),args.executable.resolve(),args.other_build.resolve(),args.evidence.resolve())))
