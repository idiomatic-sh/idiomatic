# TC007 — unquoted explicit aliases

The query checks unquoted names in explicit module/class `TypeAlias` declarations that depend on definitions guarded by resolved `typing.TYPE_CHECKING`. Quoted types, Literal values, Annotated metadata, stubs, unsupported value expressions and function-local explicit aliases are excluded. Ordinary undefined names remain F821's responsibility. Runtime binding/deletion order and real namespace identities govern fallback availability.

Both Python 3.12 and unchanged-source Python 3.10 controls passed all 32 cases on sealed worker8234: 30 raw diagnostic matches and two documented differences each. The full pinned upstream fixture remains intact; Python 3.10 retains unsupported `type` statement diagnostics. No provider or engine changes were needed.

## Exact reference differences

Pinned Ruff0.16.5, selected TC007 alone:

```python
from typing import TYPE_CHECKING, TypeAlias
class Foo: ...
del Foo
if TYPE_CHECKING:
 from external import Foo
Alias:TypeAlias=Foo
```

Ruff reports nothing; intended TC007 covers the final `Foo` at bytes127..130. The earlier runtime binding has been deleted. Exact CPython3.10 execution raises NameError; the otherwise identical nondeleted fallback control succeeds. TC007's documented runtime-availability purpose applies to the guarded import, rather than the stale binding. [Deletion semantics](https://docs.python.org/3.12/reference/simple_stmts.html#the-del-statement) establish the independent basis.

```python
from typing import TYPE_CHECKING, TypeAlias
TYPE_CHECKING=True
if TYPE_CHECKING:
 class Foo: ...
Alias:TypeAlias=Foo
```

Ruff reports TC007 at bytes113..116; intended output is empty. Exact execution verifies `Alias is Foo`. Ruff's intentional spelling-based TYPE_CHECKING heuristic treats this body as type-only despite the explicit True rebinding. This is a case-specific availability false positive from that conservative policy, not a claim that TC007 should diagnose every runtime failure. [TYPE_CHECKING documentation](https://docs.python.org/3.12/library/typing.html#typing.TYPE_CHECKING) and [assignment semantics](https://docs.python.org/3.12/reference/simple_stmts.html#assignment-statements) describe the independent basis.

## Permanent scope controls

The first draft incorrectly diagnosed annotation-only and ordinary forward names, plus two function-local aliases shadowing guarded imports. Their original source bodies remain negative controls. The actual namespace audit selects F821,F823,TC007: ordinary undefined names produce F821, while the two function-local aliases receive no TC007. [PEP613 scope restrictions](https://peps.python.org/pep-0613/#scope-restrictions) exclude function-local explicit aliases; a live class-scope positive prevents blanket namespace exclusion. Runtime NameError or UnboundLocalError alone is insufficient to extend TC007's purpose.

[Source-bound proof and controls](../../results/ci-80be-full-inventory-2026-11-03/tc007-target-supplement/README.md) bind exact query, cases, settings, helper closure and executables. Its raw archive retains the initial strict failures, namespace audit, independent Python assertions, original source coordinates and both successful native replays. Per-case certificate evidence is also retained in [the singleton profile](../../profiles/TC007/evidence/). Historical combined413 evidence is unchanged; no other rule was reverified.
