# RUF041 — unnecessary nested Literal

The rule flattens nested `typing.Literal` under lexical static-typing name resolution. A same-spelled type parameter is not the imported typing object. The existing query, provider facts and shared helpers are unchanged; seven stale fixture expectations are corrected with exact independent evidence.

All original 41 source bodies remain intact, including both full pinned upstream fixtures. Two unrelated-parameter positives are added. Python 3.10 and unchanged-source Python 3.12 controls each pass all 43 cases: 36 raw diagnostic matches and seven exact documented differences. Canonical Python 3.10 retains every unsupported PEP695 syntax diagnostic; the supported countertarget removes only independently matched keyword/bracket diagnostics.

## Scope and independent evidence

Six exact cases incorrectly identify a PEP695 parameter named `Literal` as `typing.Literal` under postponed/quoted annotations. Actual CPython3.13 controls establish parameter identity through `get_type_hints(f)["return"] is f.__type_params__[0]`. Applying nested subscription to that parameter raises TypeError; an unrelated `T` parameter preserves genuine `typing.Literal` flattening to `Literal[1]`. Invalid annotation parameterization is distinct from a redundant nested typing.Literal; runtime failure alone does not establish this style diagnostic.

Recovered decorator and `_typeshed` programs are not claimed to execute. Their original bytes and pinned outputs remain; controlled valid signatures establish the relevant name scope. Supported-version controls prove Python3.12+ parameter semantics, not execution under the canonical Python3.10 target.

The seventh case is a **lexical static-typing scope-policy difference**, not an allegation of a runtime-introspection bug. An enclosing local typing import defines the annotation's lexical names. With the explicitly recorded module fixture `custom=list`, default `get_type_hints` produces `list[list[1]]`, while supplying the enclosing locals produces `typing.Literal[1]`. Both actual outcomes are retained on CPython3.10 and3.13. The controlled function explicitly adds `return f,locals()` for observation; that changed model program is not the unchanged recovered fixture. Flattening is equivalent under the lexical typing interpretation, not promised equivalent for arbitrary runtime annotation namespaces.

Primary basis: [PEP695 type-parameter scopes](https://peps.python.org/pep-0695/#type-parameter-scopes), [legal nested literal types](https://typing.python.org/en/latest/spec/literal.html#legal-parameters-for-literal-at-type-check-time), [PEP563 static analysis](https://peps.python.org/pep-0563/#implementation), [runtime namespace limitations](https://peps.python.org/pep-0563/#resolving-type-hints-at-runtime), and [pinned RUF041 implementation](https://github.com/astral-sh/ruff/blob/9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a/crates/ruff_linter/src/rules/ruff/rules/unnecessary_nested_literal.rs). No upstream issue was filed.

## Exact reproducers

Run each unchanged `case.py` below with `ruff check --isolated --select RUF041 --target-version py310 --output-format json case.py`. Ranges are UTF-8 byte offsets. Python3.12 control findings differ only by the explicitly retained target syntax diagnostics.

### ruf041-future-generic-literal

```python
from __future__ import annotations
from typing import Literal
def f[Literal]()->Literal[Literal[1]]: ...
```

Pinned reference: invalid-syntax 67..76; RUF041 80..99. Intended: invalid-syntax 67..76.

### ruf041-forward-module-value-overrides-local

```python
from __future__ import annotations
Literal=custom
def outer():
 from typing import Literal
 def f()->Literal[Literal[1]]: ...
```

Pinned reference: none. Intended: RUF041 101..120.

### ruf041-quoted-generic-prefers-module

```python
from typing import Literal
def f[Literal]()->"Literal[Literal[1]]": ...
```

Pinned reference: invalid-syntax 32..41; RUF041 46..65. Intended: invalid-syntax 32..41.

### ruf041-dispatch-call-register-forward-generic

```python
from __future__ import annotations
from functools import singledispatch
from typing import Literal
@singledispatch
def fun(x): ...
@fun.register()
def f[Literal](x:Literal[Literal[1]])->Literal[Literal[1]]: ...
```

Pinned reference: invalid-syntax 152..161; RUF041 164..183; RUF041 186..205. Intended: invalid-syntax 152..161.

### ruf041-dispatch-variadic-forward-generic

```python
from __future__ import annotations
from functools import singledispatch
from typing import Literal
@singledispatch
def fun(x): ...
@fun.register
def f[Literal](*x:Literal[Literal[1]],**y:Literal[Literal[1]]): ...
```

Pinned reference: invalid-syntax 150..159; RUF041 163..182; RUF041 187..206. Intended: invalid-syntax 150..159.

### ruf041-forward-typeshed-member

```python
from __future__ import annotations
from _typeshed import Literal
def f[Literal]()->Literal[Literal[1]]: ...
```

Pinned reference: invalid-syntax 70..79; RUF041 83..102. Intended: invalid-syntax 70..79.

### ruf041-dispatch-shadowed-interface-forward

```python
from __future__ import annotations
from functools import singledispatch
from typing import Literal
@singledispatch
def fun(x): ...
fun=custom
@fun.register
def f[Literal](x:Literal[Literal[1]])->Literal[Literal[1]]: ...
```

Pinned reference: invalid-syntax 161..170; RUF041 173..192; RUF041 195..214. Intended: invalid-syntax 161..170.

## Preserved proof artifacts

[Source-bound supplement and raw archive](../../results/ci-80be-full-inventory-2026-11-03/ruf041-target-supplement/README.md) preserve the old92b failures, current first41-case replay, all pinned target outputs and both successful native controls. The generic validator binds current query, cases, settings, transitive helper bytes and actual sealed executables. No previous rule proofs were reverified and the historical combined413 report is unchanged.

Independent evidence: [actual Python3.13 controls](../../profiles/RUF041/evidence/py313.json), [actual Python3.10 scope outcome](../../profiles/RUF041/evidence/py310.json), [executed control program](../../profiles/RUF041/evidence/python-program.py), [exact pinned reference sources/outputs](../../profiles/RUF041/evidence/reference-results.json), [scope/version contract](../../profiles/RUF041/evidence/scope-contract.json). Each certificate retains the full reference and intended multiset; unrelated syntax findings are not waived.
