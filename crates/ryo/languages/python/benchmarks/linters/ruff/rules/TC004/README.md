# TC004 — runtime import in a type-checking block

The rule checks guarded imports whose values are required at runtime and unavailable outside the guard. Existing binding identities and ordered events determine runtime fallback availability. Annotation evaluation uses the shared context helper; postponed/local annotations, quoted types, PEP695 aliases/bounds, stubs and references inside the guarded body are excluded. Explicit exports count unless a module-level `__getattr__` can provide them. Aliased imports use the bound-name token span.

All 17 pinned Ruff0.16.5 upstream fixture files remain intact, alongside 24 independent controls. Both full-source target controls execute successfully on sealed worker8234: Python 3.12 and Python 3.10 each pass all 41 cases, with 39 raw diagnostic matches and two exact documented differences. Python 3.10 preserves unsupported PEP695 keyword/bracket diagnostics.

## Exact documented reference differences

```python
class Foo: ...
from typing import TYPE_CHECKING
if TYPE_CHECKING:
 from external import Foo
x=Foo
```

Ruff reports TC004 on the imported Foo at bytes88..91; intended output is empty. The existing runtime class remains bound because the guard is false. Exact CPython3.10 execution verifies `x is Foo`, without importing external. This tests the rule's documented requirement that the unavailable symbol have only a guarded definition, rather than all runtime import/style policy.

```python
from typing import TYPE_CHECKING
TYPE_CHECKING=True
if TYPE_CHECKING:
 from external import Foo
x=Foo
```

Ruff reports TC004 at bytes92..95; intended output is empty. The independently executed source is unchanged. Its explicit test environment supplies a real external module and Foo class through `sys.modules`, then verifies `x is Foo`. This distinguishes availability after the guard from whether an arbitrary package is installed. Ruff intentionally recognizes the TYPE_CHECKING spelling conservatively even after rebinding; the difference is an exact availability false positive under that policy, not a claim that all conservative lint policies are Python bugs.

Primary basis: [TC004's documented purpose](https://docs.astral.sh/ruff/rules/runtime-import-in-type-checking-block/), [assignment semantics](https://docs.python.org/3.12/reference/simple_stmts.html#assignment-statements), and [typing.TYPE_CHECKING](https://docs.python.org/3.12/library/typing.html#typing.TYPE_CHECKING). The pinned source and actual outputs, rather than current documentation alone, bind the historical reference observation. No upstream issue was filed.

These two observations are documented availability-contract versus analysis/policy differences, **not established Ruff bugs**. A runtime fallback proves name availability, not identity with the import selected by a type checker. The intended contract is limited to the documented unavailable-at-runtime condition. Guard-only value/eager-annotation positives retain the imported standard False guard and no runtime fallback; class/module export controls distinguish real namespace ownership.

[Source-bound proof, target controls and raw archive](../../results/ci-80be-full-inventory-2026-11-03/tc004-target-supplement/README.md) preserve complete actual diagnostics, the first three failed drafts, executable/source identities, five independent CPython namespace controls and the additional ordinary False-guard/no-fallback NameError control. [Per-case evidence](../../profiles/TC004/evidence/) retains exact sources and the explicit external import prerequisite. Original combined413 evidence and previous rule proofs are unchanged.
