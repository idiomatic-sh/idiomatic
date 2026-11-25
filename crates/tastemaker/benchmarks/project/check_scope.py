"""Check project-scope invariants against a one-worker shipping reader and explicit refusals."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def check(executor, shipping):
    def file(path):
        return {"file": path}

    def table(name, columns, rows):
        return dict(name=name, columns=columns, rows=rows)

    paths = ["src.rs", "cfg.toml", "target.css"]
    edges = table("ctx.edges", [["src", "File"], ["dst", "File"]],
                  [[file(src), file(paths[2])] for src in paths[:2]])
    block = table("ctx.block", [["path", "File"]], [[file(paths[1])]])
    join = ('let pairs = select e."dst" as "dst", f."path" as "origin" '
            'from "ctx.edges" as e join "filesystem.file" as f on e."src" = f."path"; '
            'let ok = antijoin(left=pairs, right="ctx.block", left-on=["origin"], right-on=["path"]); '
            'emit ok subject "dst" key("origin");')
    count = ('let c = group-count(from="ctx.edges",keys=["dst"],count="n",distinct="src"); '
             'emit c subject "dst" key("dst");')
    stamps = table("ctx.stamps", [["dst", "File"], ["stamp", "Number"]],
                   [[file(paths[2]), 5], [file(paths[2]), 2], [file(paths[0]), 9]])
    least = ('let m = group-min(from="ctx.stamps",keys=["dst"],order="stamp"); '
             'emit m subject "dst" key("stamp");')
    greatest = ('let m = group-max(from="ctx.stamps",keys=["dst"],order="stamp"); '
                'emit m subject "dst" key("stamp");')
    host = 'let x = select * from "ctx.number"; emit x subject "id" key("id");'
    files = 'let x = select * from "filesystem.file" where glob_any(path, "*.toml"); emit x subject "path" key("path");'
    empty = table("ctx.block", block["columns"], [])
    controls = [
        ("subject-in-another-file", join, paths, [edges, block], 1),
        ("count-across-files", count, paths, [edges], 1),
        ("excluded-subject", join, paths, [edges, block], 0),
        ("excluded-nonsubject", 'let a = select * from "ctx.edges"; emit a subject "src" key("src");', paths, [edges], 1),
        ("per-row-file-filter", files, paths, [], 1),
        ("least-across-files", least, paths, [stamps], 2),
        ("greatest-across-files", greatest, paths, [stamps], 2),
        ("out-of-inventory", count, paths[:2], [edges], 0),
        ("zero-file-positive", host, [], [table("ctx.number", [["id", "Number"]], [[7]])], 1),
        ("same-text-changed-schema", host, [], [table("ctx.number", [["tag", "Text"], ["id", "Number"]], [["example", 7]])], 1),
        ("zero-file-empty", join, [], [edges, empty], 0),
        ("unverified-capture", 'let c = capture(from="filesystem.file", path="path", pattern="{x}", captures=["x"]); emit c subject "path" key("x");', [], [], "separator"),
        ("ordered-extend", 'let a = extend(from="filesystem.file",values={"z":0,"a":1}); emit a subject "path" key("z");', [], [], "declaration order"),
        ("missing-negative-input", join, [], [edges], "ctx.block"),
        ("partial-negative-input", join, [], [edges, dict(empty, partial="upstream failed")], "upstream failed"),
        ("all-partitions-skipped", files, paths, [], "partitions excluded"),
        ("unclosed-inventory", files, None, [], "incomplete project inventory"),
    ]
    entries = []
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        for path in paths:
            (root / path).write_text("")
        for name, query, inventory, hosted, reference in controls:
            pack = root / hashlib.sha256(query.encode()).hexdigest()
            rules = pack / "rules/query"
            rules.mkdir(parents=True, exist_ok=True)
            (rules / "query.ryoql").write_text(query)
            for order, stride in (("forward", 1), ("reverse", -1)):
                admission = (inventory or [])[::stride]
                include = []
                if name in ("excluded-subject", "excluded-nonsubject"):
                    include = ["*.rs"]
                    admission = [p for p in admission if p.endswith(".rs")]
                if name == "all-partitions-skipped":
                    include, admission = ["absent/**"], []
                inputs = dict(root=str(root), include=include, selected_rules=["query"], inventory=inventory,
                              files=admission, parameters={}, hosted=[dict(t, rows=t["rows"][::stride])
                                                                      for t in hosted[::stride]])
                identity = f"{name}/{order}"
                request = root / f"{name}-{order}.json"
                request.write_text(json.dumps(inputs))
                if isinstance(reference, int):
                    output = root / "shipping.jsonl"
                    subprocess.run([shipping, "--shipping", pack, request, output], check=True,
                                   stdout=subprocess.DEVNULL)
                    buf = output.read_bytes()
                    assert len(buf.splitlines()) - int(buf == b"\n") == reference, (name, buf)
                    wanted = hashlib.sha256(buf).hexdigest()
                else:
                    wanted = reference
                entries.append((identity, pack, request, wanted, isinstance(reference, int)))

        manifest = root / "manifest.tsv"
        manifest.write_text("".join(f"{key}\t{pack}\t{request}\t-\n"
                                    for key, pack, request, _, _ in entries))
        results = root / "receipts.tsv"
        subprocess.run([executor, "--tm-batch", manifest, results, "--workers", "1"], check=True)
        for entry, line in zip(entries, results.read_text().splitlines(), strict=True):
            key, pack, request, wanted, answered = entry
            identity, status, digest, *unknowns = line.split("\t")
            assert identity == key and not unknowns, (key, line)
            if answered:
                assert status != "refused" and digest == wanted, (key, wanted, line)
            else:
                assert status == "refused", (key, wanted, line)
                # Batch receipts omit the refusal sentence. Read it through the
                # single-request replay of the same input.
                single = subprocess.run([executor, "--mixed", pack, request, root / "unused"],
                                        capture_output=True, text=True)
                assert single.returncode and wanted in single.stderr, (key, wanted, single.stderr)
            print(key, digest if answered else wanted)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executor", type=Path)
    parser.add_argument("shipping", type=Path)
    args = parser.parse_args()
    check(args.executor, args.shipping)
