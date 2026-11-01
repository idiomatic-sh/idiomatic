"""Compile exact retained inputs without executing their code or inheriting compiler features."""
import __future__
import hashlib
import json
import sys
import warnings
from pathlib import Path

inputs = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
rows = []
for case in inputs["cases"]:
    source = case["source"]
    digest = hashlib.sha256(source.encode("utf-8")).hexdigest()
    if digest != case["source_sha256"]:
        raise ValueError("source bytes differ from sealed input")
    row = {key: case[key] for key in ["name", "kind", "source_sha256", "filename"]}
    with warnings.catch_warnings(record=True) as observed:
        warnings.simplefilter("always")
        try:
            code = compile(source, case["filename"], "exec", dont_inherit=True)
            row.update(accepted=True, annotations_future_flag=bool(code.co_flags & __future__.annotations.compiler_flag))
        except SyntaxError as error:
            row.update(accepted=False, error={
                "type": type(error).__name__, "message": error.msg,
                "line": error.lineno, "column": error.offset,
                "end_line": error.end_lineno, "end_column": error.end_offset,
                "text": error.text,
            })
        row["warnings"] = [{"category": each.category.__name__, "message": str(each.message)} for each in observed]
    rows.append(row)
print(json.dumps({
    "python": sys.version, "executable": sys.executable,
    "isolated": bool(sys.flags.isolated), "dont_inherit": True,
    "operation": "compile only; returned code objects are never executed",
    "cases": rows,
}, indent=2))
