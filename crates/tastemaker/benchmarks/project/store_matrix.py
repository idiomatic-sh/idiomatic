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
import shutil
import subprocess
import tempfile
from pathlib import Path


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

    def invoke(binary, retain):
        output.unlink(missing_ok=True)
        args = [binary, "--tm-batch-retained", store_root] if retain else [binary, "--tm-batch"]
        p = subprocess.run([*args,manifest,output],capture_output=True,env=env)
        diagnostics = sorted(x for x in p.stdout.splitlines() if x.startswith(b"REFUSAL"))
        if p.returncode:
            raise RuntimeError((p.returncode,p.stderr[-1200:]))
        return output.read_bytes(), diagnostics

    with tempfile.TemporaryDirectory(dir=fixture) as tmp:
        backup = Path(tmp) / "seed"
        original = invoke(exe, True)
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
                    pair=next(cache.glob("pairs-*.postcard"))
                    with pair.open("r+b") as f:
                        f.truncate(pair.stat().st_size//2)
                warm=invoke(bin_path,True)
                cold=invoke(bin_path,False)
                hit=invoke(bin_path,True)
                ok=warm==cold==hit
                if case in ("answer-change","same-stamp","delete"):
                    ok &= cold[0]!=original[0]
                if case=="refusal":
                    ok &= bool(cold[1])
                passed &= ok
                with (evidence/"session-matrix.tsv").open("a") as f:
                    w=csv.writer(f,delimiter="\t",lineterminator="\n")
                    if f.tell()==0:
                        w.writerow(["scenario","pass","bytes","warm_sha256","cold_sha256","refusals"])
                    w.writerow([case,ok,len(cold[0]),hashlib.sha256(warm[0]).hexdigest(),
                                hashlib.sha256(cold[0]).hexdigest(),len(cold[1])])
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
