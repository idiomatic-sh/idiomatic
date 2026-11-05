import builtins
import hashlib
import io
import json
from pathlib import Path
import symtable
import types
import typing

base = Path(__file__).parent
cases = {case["name"]: case for case in json.loads((base / "semantic-cases.json").read_text())}


def source(name):
    return cases[name]["files"]["case.py"]


def run(name, *, checking=None, rows=None):
    sink = io.StringIO()
    env = {"rows": ["z"] if rows is None else rows, "open": lambda *_: sink}
    if checking is not None:
        env["unknown"] = types.SimpleNamespace(TYPE_CHECKING=checking)
    original_checking = typing.TYPE_CHECKING
    try:
        exec(compile(source(name), "case.py", "exec"), env)
        outcome = {"status": "ok", "x": env.get("x", "<unbound>"), "written": sink.getvalue()}
    except Exception as error:
        outcome = {"status": type(error).__name__, "message": str(error)}
    finally:
        typing.TYPE_CHECKING = original_checking
    return outcome


generic_source = source("io-annotation-3")
generic_env = {"rows": ["a", "b"]}
exec(compile(generic_source, "case.py", "exec"), generic_env)
generic_sink = io.StringIO()
generic_env["fn"](generic_sink)

outer_source = source("outer-runtime-history-3")
outer_table = symtable.symtable(outer_source, "case.py", "exec")
outer_function = next(child for child in outer_table.get_children() if child.get_name() == "fn")
outer_globals = {"unknown": object(), "rows": ["z"], "open": lambda *_: io.StringIO()}
exec(compile(outer_source, "case.py", "exec"), outer_globals)

result = {
    "interpreter_version": __import__("sys").version,
    "cases": {name: {"source_sha256": hashlib.sha256(source(name).encode()).hexdigest()} for name in (
        "io-annotation-3",
        "outer-runtime-history-3",
        "runtime-builtin-0-1",
        "runtime-shadow-history-4",
        "runtime-shadow-history-5",
        "runtime-shadow-history-6",
        "imported-type-checking-false",
        "imported-typing-attribute-false",
        "rebound-type-checking-true",
        "rebound-typing-attribute-true",
    )},
    "generic_io": {
        "origin_is_typing_IO": typing.get_origin(typing.IO[str]) is typing.IO,
        "interface_has_writelines": callable(getattr(typing.IO, "writelines", None)),
        "framed_exact_loop_output": generic_sink.getvalue(),
    },
    "deleted_outer_builtin": {
        "function_list_is_local": outer_function.lookup("list").is_local(),
        "deleted_module_list_restores_builtin": outer_globals.get("list", builtins.list) is builtins.list,
    },
    "conditional": {
        "bare_name": run("runtime-shadow-history-4"),
        "bare_name_with_else": run("runtime-shadow-history-6"),
        "unknown_attribute_false": run("runtime-shadow-history-5", checking=False),
        "unknown_attribute_true": run("runtime-shadow-history-5", checking=True),
        "imported_direct_empty_rows": run("imported-type-checking-false", rows=[]),
        "imported_attribute_empty_rows": run("imported-typing-attribute-false", rows=[]),
        "rebound_direct_empty_rows": run("rebound-type-checking-true", rows=[]),
        "rebound_attribute_empty_rows": run("rebound-typing-attribute-true", rows=[]),
    },
    "limits": "Exact authored sources run with framed rows and open; these observations do not prove universal rewrite equivalence.",
}
print(json.dumps(result, indent=2))
