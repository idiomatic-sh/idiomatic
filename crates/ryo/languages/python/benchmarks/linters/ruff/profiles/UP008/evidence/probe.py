"""Run exact UP008 source controls with CPython 3.10."""

from __future__ import annotations

import ast
import hashlib
import json
import platform
from pathlib import Path


CONTROLS = Path(__file__).with_name("source-controls.json")


def attempt(function):
    try:
        return {"outcome": "return", "value": function()}
    except Exception as error:
        return {"outcome": "exception", "type": type(error).__name__, "message": str(error)}


def load(source):
    namespace = {}
    exec(compile(source, "case.py", "exec"), namespace)
    return namespace


def main():
    assert platform.python_implementation() == "CPython"
    assert platform.python_version() == "3.10.19"
    controls = json.loads(CONTROLS.read_text())
    results = {}
    for name, source in controls.items():
        results[name] = {"source_sha256": hashlib.sha256(source.encode()).hexdigest()}

    name = "positional-only-self"
    source = controls[name]
    method = ast.parse(source).body[0].body[0]
    assert isinstance(method, ast.FunctionDef)
    assert [parameter.arg for parameter in method.args.posonlyargs] == ["self"]
    paired = "class Base:\n    def f(self):\n        return 'base'\n" + source.replace("class C:", "class C(Base):")
    explicit = load(paired)
    implicit = load(paired.replace("super(C, self)", "super()"))
    results[name].update({
        "first_positional_only_parameter": "self",
        "explicit_super_result": attempt(lambda: explicit["C"]().f()),
        "zero_argument_super_result": attempt(lambda: implicit["C"]().f()),
    })
    assert results[name]["explicit_super_result"] == {"outcome": "return", "value": "base"}
    assert results[name]["zero_argument_super_result"] == results[name]["explicit_super_result"]

    name = "module-class-rebound-after-definition"
    namespace = load(controls[name])
    results[name].update({
        "original_is_current_class": namespace["original"] is namespace["C"],
        "original_method_after_rebinding": attempt(lambda: namespace["original"]().f()),
    })
    assert results[name]["original_is_current_class"] is False
    assert results[name]["original_method_after_rebinding"]["type"] == "TypeError"

    for name, marker, saved, explicit in (
        ("nested-class-attribute-rebound-after-definition", "Outer.Inner = Other\n", "OriginalInner", "super(Outer.Inner, self)"),
        ("nested-parent-attribute-rebound-after-definition", "A.B = Other\n", "OriginalC", "super(A.B.C, self)"),
    ):
        source = controls[name]
        assert source.endswith(marker)
        before = load(source[: -len(marker)])
        after = load(source)
        implicit = load(source.replace(explicit, "super()"))
        prior = attempt(lambda: before[saved]().f())
        current = attempt(lambda: after[saved]().f())
        rewritten = attempt(lambda: implicit[saved]().f())
        results[name].update({"before_rebinding": prior, "after_rebinding": current,
                              "zero_argument_super_after_rebinding": rewritten})
        assert prior == {"outcome": "return", "value": None}
        assert current["outcome"] == "exception"
        assert rewritten == prior
    assert results["nested-class-attribute-rebound-after-definition"]["after_rebinding"]["type"] == "TypeError"
    assert results["nested-parent-attribute-rebound-after-definition"]["after_rebinding"]["type"] == "AttributeError"

    print(json.dumps({"schema": "up008-cpython310-controls/v1",
                      "implementation": platform.python_implementation(),
                      "version": platform.python_version(),
                      "cases": results}, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
