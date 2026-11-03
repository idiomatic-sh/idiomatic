# RUF020 — Never in unions

The rule reports actual `typing.Never` or `typing.NoReturn` operands in unions. A same-spelled PEP695 type parameter is a distinct type variable, not the bottom type. Existing source-fact name resolution and type-parameter barriers remain unchanged. No provider, engine or shared helper workaround is introduced.

All42 canonical cases remain, including the exact full pinned `RUF020.py` fixture. The Python3.10 profile preserves unsupported-PEP695 syntax diagnostics. Supported-version controls are separate, keep the source bodies unchanged and bind their own expected outputs.

## Examining the reference differences

Eight exact fixtures expose same-spelling type-parameter shadowing in postponed or quoted annotations. Actual CPython3.13 checks compare type-variable identities: `Never | int` contains the function parameter rather than `typing.Never`; an unrelated parameter still permits genuine imported-Never findings. A parameter named `Union` or `t` does not become `typing.Union` or the typing module. Some recovered fixtures have invalid decorators or unavailable imports; the controls do not claim those original programs execute. Their preserved signatures and independent valid controls establish the relevant lexical scope.

The ninth fixture is a **static-typing scope-policy difference**, not an allegation of a Ruff runtime bug. For an enclosing local `typing.Never` import and conflicting module name, ordinary `get_type_hints` resolves module globals, while explicitly supplying enclosing locals resolves the imported bottom type. Both observed outcomes are retained. This rule follows lexical static-typing scope for that exact source; it does not claim universal runtime introspection equivalence. PEP563 distinguishes unchanged static type analysis from the limitations of runtime postponed-annotation resolution.

The canonical target is Python3.10. PEP695 headers require3.12, so their existing `invalid-syntax` findings are retained. Actual3.13 controls prove supported-version type-parameter behavior, not execution of those sources under3.10. Separate actual3.10 controls explicitly replace the later-added `typing.Never` import by `typing.NoReturn as Never`; their changed source is labeled accordingly.

| Case | Pinned reference diagnostics | Intended diagnostics | Classification |
|---|---|---|---|
| `ruf020-future-generic-never` | RUF020 76..81; invalid-syntax 65..72 | invalid-syntax 65..72 | type-parameter shadowing reference false positive |
| `ruf020-future-generic-union` | RUF020 88..93; invalid-syntax 71..78 | invalid-syntax 71..78 | type-parameter shadowing reference false positive |
| `ruf020-forward-module-attribute-generic` | RUF020 66..73; invalid-syntax 59..62 | invalid-syntax 59..62 | type-parameter shadowing reference false positive |
| `ruf020-forward-module-value-overrides-local` | none | RUF020 97..102 | static-typing scope policy |
| `ruf020-quoted-generic-prefers-module` | RUF020 42..47; invalid-syntax 30..37 | invalid-syntax 30..37 | type-parameter shadowing reference false positive |
| `ruf020-dispatch-call-register-forward-generic` | RUF020 160..165; RUF020 174..179; invalid-syntax 150..157 | invalid-syntax 150..157 | type-parameter shadowing reference false positive |
| `ruf020-dispatch-variadic-forward-generic` | RUF020 159..164; RUF020 175..180; invalid-syntax 148..155 | invalid-syntax 148..155 | type-parameter shadowing reference false positive |
| `ruf020-forward-typeshed-member` | RUF020 79..84; invalid-syntax 68..75 | invalid-syntax 68..75 | type-parameter shadowing reference false positive |
| `ruf020-dispatch-shadowed-interface-forward` | RUF020 169..174; RUF020 183..188; invalid-syntax 159..166 | invalid-syntax 159..166 | type-parameter shadowing reference false positive |

Every range above is a UTF-8 byte offset in the unchanged `case.py`. The complete original reference multisets are bound separately in each certificate; unrelated syntax findings are not waived.

Extract any exact reproducer from `cases.toml` (replace the selected case name):

```python
from pathlib import Path
import tomllib
data = tomllib.loads(Path("cases.toml").read_text())
case = next(c for c in data["cases"] if c["name"] == "ruf020-future-generic-never")
for name, text in case["files"].items():
    Path(name).write_text(text)
```

```sh
ruff check --isolated --select RUF020 --target-version py310 case.py --output-format json
```

The independent namespace probe is `def f[Never]() -> Never | int: ...`, with `from __future__ import annotations` and `from typing import Never`. On Python3.13, `typing.get_args(typing.get_type_hints(f)["return"])` contains `f.__type_params__[0]`, and does not contain `typing.Never`. The positive control uses parameter `T` instead.

Evidence: [exact original/control sources](../../profiles/RUF020/evidence/executed-reference-inputs.json), [pinned complete outputs](../../profiles/RUF020/evidence/reference-results.json), [actual3.13 results](../../profiles/RUF020/evidence/cpython.json), [actual3.10 compatible sources](../../profiles/RUF020/evidence/cpython310-inputs.json), [actual3.10 results](../../profiles/RUF020/evidence/cpython310.json), [explicit scope/version contract](../../profiles/RUF020/evidence/scope-contract.json).

Primary basis: [PEP695 scopes](https://peps.python.org/pep-0695/#type-parameter-scopes), [get_type_hints namespaces](https://docs.python.org/3.13/library/typing.html#typing.get_type_hints), [PEP563 static analysis and runtime evaluation](https://peps.python.org/pep-0563/#implementation), [runtime namespace limitations](https://peps.python.org/pep-0563/#resolving-type-hints-at-runtime), [pinned RUF020 implementation](https://github.com/astral-sh/ruff/blob/9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a/crates/ruff_linter/src/rules/ruff/rules/never_union.rs). No upstream issue was filed.
