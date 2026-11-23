"""Encode SDK calls for the existing project and mixed-reader benchmark protocols."""
import hashlib
import json
from pathlib import Path
import os


def digest(data):
    return hashlib.sha256(data).hexdigest()


def line(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n"


def safe_path(root, name):
    relative = Path(name)
    if relative.is_absolute() or ".." in relative.parts or not relative.parts:
        raise ValueError(f"invalid captured input path {name!r}")
    return root / relative


def row_digest(rows):
    serialized = sorted(json.dumps(r, ensure_ascii=False, sort_keys=True, separators=(",", ":")) for r in rows)
    return digest(("\n".join(serialized) + "\n").encode())


class Packets:
    def __init__(self, directory):
        self.directory = directory
        self.inputs = directory / "inputs"
        self.inputs.mkdir()
        self.project = directory / "project.jsonl"
        self.mixed = directory / "mixed.jsonl"

    def source_root(self, call):
        corpus = call["corpus"]
        given = corpus.get("given", {})
        paths = sorted(set(corpus.get("listed", given)).union(call["beside"]))
        listed = "listed" in corpus
        if "given" not in corpus and not listed:
            raise ValueError("the captured corpus must declare its inventory")
        overlays = {**given, **call["beside"]}
        if listed and not overlays:
            return Path(call["root"]), paths
        key = digest(line([call["root"] if listed else "", paths, overlays]).encode())
        root = self.inputs / key
        if not root.exists():
            root.mkdir()
            for name in paths:
                dest = safe_path(root, name)
                dest.parent.mkdir(parents=True, exist_ok=True)
                if name in overlays:
                    dest.write_bytes(overlays[name].encode())
                else:
                    source = safe_path(Path(call["root"]), name)
                    if source.is_file():
                        os.link(source.resolve(), dest)
        return root, paths

    def append(self, project, mixed, call, selected=None, reverse=False):
        root, inventory = self.source_root(call)
        if reverse:
            inventory.reverse()
        for index, rule in enumerate(call["rules"]):
            if selected is not None and rule["id"] != selected:
                continue
            identity = f'{call["stamp"]}/{index}'
            text = rule["query"]
            pack = self.directory / "packs" / digest(text.encode())
            query = pack / "rules/query/query.ryoql"
            if not query.exists():
                query.parent.mkdir(parents=True)
                query.write_bytes(text.encode())
            files = sorted(set(rule["admitted"]), reverse=reverse)
            # The mixed API cannot bind an unused caller schema. Names here are
            # literal RyoQL relation references, using the shipping wire encoding.
            hosted = [t for t in call["supplied"] if json.dumps(t["name"]) in text]
            if reverse:
                hosted = [dict(t, rows=t["rows"][::-1]) for t in reversed(hosted)]
            params = rule["parameters"]
            project.write(line(dict(id=identity, query=text, parameters=params, files=files,
                                    inventory=inventory, hosted=hosted)))
            encoded = line(dict(root=str(root), files=files, selected_rules=["query"], parameters=params,
                                hosted=hosted, include=rule["include"], exclude=rule["exclude"])).encode()
            request = self.inputs / (digest(encoded) + ".json")
            if not request.exists():
                request.write_bytes(encoded)
            mixed.write(line(dict(id=identity, pack=str(pack), request=str(request), count=len(files))))


def capture_pairs(source):
    pending = {}
    with source.open() as stream:
        for raw in stream:
            record = json.loads(raw)
            stamp = record["stamp"]
            if "rules" in record:
                if stamp in pending:
                    raise ValueError(f"duplicate request {stamp}")
                pending[stamp] = record
            else:
                yield pending.pop(stamp), record
    if pending:
        raise ValueError(f"{len(pending)} missing SDK replies")
