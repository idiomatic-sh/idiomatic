# RUF051 — if-key-in-dict-del

The query recognizes one membership test and one matching subscript deletion, with no else/extra statements/targets. It requires a statically supported dictionary receiver and matching name or constant keys. The type helper uses actual binding identity, initializer/annotation evidence and lexical type-parameter scope, not Ruff checker-time builtin-name shortcuts. Annotations are declared static type evidence, not runtime argument enforcement. A function object does not inherit its return type.

All 69 original source bodies and the full upstream fixture are preserved. Four valid sync/async self/different context-return controls are added. The generic assignment helper no longer treats a context manager as its as-target: the target receives enter/aenter's result. Standard IO inference remains separate under resolved known IOBase initialization and its self-return protocol; arbitrary managers stay unknown. No provider or Rust change is introduced.

Targeted current-source native replays pass all 73 canonical and 73 Python3.12 cases. Every affected transitive consumer has complete targeted evidence; the existing shared full413 runner must produce a fresh baseline before publication. Initial failures remain preserved; certificates alone do not establish PASS.

## Shared context-target and value-inference audit

These are exact diagnostic-contract differences, not a blanket waiver for invalid programs. A diagnostic requiring a known receiver type cannot use the context manager as evidence of the enter result. All seven protocol cases use builtin dict/set/list managers with no enter/aenter method; no legitimate target type is lost. Actual Python3.10/3.13 controls cover every original source, valid self-return and different-return managers, and known StringIO self-return. Unknown custom managers remain uninferred.

### ruf051-function-definition

```python
def d()->dict: ...
if key in d: del d[key]
```

Pinned Ruff0.16.5: RUF051 32..42. Intended: none.

The receiver d is a function object, not its annotated return value. Independent exact-source execution raises TypeError for membership and verifies callable(d) and not isinstance(d,dict). Annotation metadata does not establish a dictionary instance. Unsupported generic-header diagnostics remain.

### ruf051-future-parameter-shadow

```python
from __future__ import annotations
def f(d:dict,dict):
 if key in d: del d[key]
```

Pinned Ruff0.16.5: none. Intended: RUF051 69..79.

Annotations are static declared-type evidence, not guaranteed runtime argument types. Eager dict annotation object does not change after a later name rebind; ordinary function-body or value-parameter names do not redefine the annotation namespace. Actual dictionary argument controls exercise deletion. Dict literals/constructors establish instance values independently of the builtin spelling at later use. Function return annotation does not make the function itself a dictionary. The original namespace and live dictionary-argument controls establish the exact applicable receiver independently. This is declared-type/definition-phase inference, not a guarantee for arbitrary runtime calls.

### ruf051-eager-module-rebind

```python
def f(d:dict):
 if key in d: del d[key]
dict=custom
```

Pinned Ruff0.16.5: none. Intended: RUF051 29..39.

Annotations are static declared-type evidence, not guaranteed runtime argument types. Eager dict annotation object does not change after a later name rebind; ordinary function-body or value-parameter names do not redefine the annotation namespace. Actual dictionary argument controls exercise deletion. Dict literals/constructors establish instance values independently of the builtin spelling at later use. Function return annotation does not make the function itself a dictionary. The original namespace and live dictionary-argument controls establish the exact applicable receiver independently. This is declared-type/definition-phase inference, not a guarantee for arbitrary runtime calls.

### ruf051-future-body-earlier-bind

```python
from __future__ import annotations
def f(d:dict):
 dict=custom
 if key in d: del d[key]
```

Pinned Ruff0.16.5: none. Intended: RUF051 77..87.

Annotations are static declared-type evidence, not guaranteed runtime argument types. Eager dict annotation object does not change after a later name rebind; ordinary function-body or value-parameter names do not redefine the annotation namespace. Actual dictionary argument controls exercise deletion. Dict literals/constructors establish instance values independently of the builtin spelling at later use. Function return annotation does not make the function itself a dictionary. The original namespace and live dictionary-argument controls establish the exact applicable receiver independently. This is declared-type/definition-phase inference, not a guarantee for arbitrary runtime calls.

### ruf051-with-target

```python
with {} as d:
 if key in d: del d[key]
```

Pinned Ruff0.16.5: RUF051 28..38. Intended: none.

With and async-with assign __enter__/__aenter__ result, not the context manager. Builtin dict/set/list managers in these exact7sources have neither method. No legitimate target type is lost: no target value exists. This is an unknown-result inference correction, not a blanket rule suppression whenever any runtime error exists. The diagnostic requires a known result receiver type; a manager initializer supplies no such evidence. Valid self-return/different-return controls preserve the uncertainty rather than pretending all managers return themselves.

### ruf051-async-with-target

```python
async def f():
 async with {} as d:
  if key in d: del d[key]
```

Pinned Ruff0.16.5: RUF051 51..61. Intended: none.

With and async-with assign __enter__/__aenter__ result, not the context manager. Builtin dict/set/list managers in these exact7sources have neither method. No legitimate target type is lost: no target value exists. This is an unknown-result inference correction, not a blanket rule suppression whenever any runtime error exists. The diagnostic requires a known result receiver type; a manager initializer supplies no such evidence. Valid self-return/different-return controls preserve the uncertainty rather than pretending all managers return themselves.

### ruf051-eager-module-rebind-unseen-later-function

```python
def f(d:dict):
 if key in d: del d[key]
def g():
 import builtins
dict=custom
```

Pinned Ruff0.16.5: none. Intended: RUF051 29..39.

Annotations are static declared-type evidence, not guaranteed runtime argument types. Eager dict annotation object does not change after a later name rebind; ordinary function-body or value-parameter names do not redefine the annotation namespace. Actual dictionary argument controls exercise deletion. Dict literals/constructors establish instance values independently of the builtin spelling at later use. Function return annotation does not make the function itself a dictionary. The original namespace and live dictionary-argument controls establish the exact applicable receiver independently. This is declared-type/definition-phase inference, not a guarantee for arbitrary runtime calls.

### ruf051-builtin-variable-name-shadow

```python
dict={}
if key in dict: del dict[key]
```

Pinned Ruff0.16.5: none. Intended: RUF051 24..37.

Annotations are static declared-type evidence, not guaranteed runtime argument types. Eager dict annotation object does not change after a later name rebind; ordinary function-body or value-parameter names do not redefine the annotation namespace. Actual dictionary argument controls exercise deletion. Dict literals/constructors establish instance values independently of the builtin spelling at later use. Function return annotation does not make the function itself a dictionary. The original namespace and live dictionary-argument controls establish the exact applicable receiver independently. This is declared-type/definition-phase inference, not a guarantee for arbitrary runtime calls.

### ruf051-constructor-before-current-rebind

```python
d=dict()
def f():
 if key in d: del d[key]
dict=custom
```

Pinned Ruff0.16.5: none. Intended: RUF051 32..42.

Annotations are static declared-type evidence, not guaranteed runtime argument types. Eager dict annotation object does not change after a later name rebind; ordinary function-body or value-parameter names do not redefine the annotation namespace. Actual dictionary argument controls exercise deletion. Dict literals/constructors establish instance values independently of the builtin spelling at later use. Function return annotation does not make the function itself a dictionary. The original namespace and live dictionary-argument controls establish the exact applicable receiver independently. This is declared-type/definition-phase inference, not a guarantee for arbitrary runtime calls.

### ruf051-function-type-parameter-return-fast

```python
def d[dict]()->dict: ...
if key in d: del d[key]
```

Pinned Ruff0.16.5: invalid-syntax 5..11; RUF051 38..48. Intended: invalid-syntax 5..11.

The receiver d is a function object, not its annotated return value. Independent exact-source execution raises TypeError for membership and verifies callable(d) and not isinstance(d,dict). Annotation metadata does not establish a dictionary instance. Unsupported generic-header diagnostics remain.

### ruf051-future-function-generic-return-slow

```python
from __future__ import annotations
import builtins
def d[dict]()->dict: ...
if key in d: del d[key]
```

Pinned Ruff0.16.5: invalid-syntax 56..62; RUF051 89..99. Intended: invalid-syntax 56..62.

The receiver d is a function object, not its annotated return value. Independent exact-source execution raises TypeError for membership and verifies callable(d) and not isinstance(d,dict). Annotation metadata does not establish a dictionary instance. Unsupported generic-header diagnostics remain.

Primary basis: [with-statement result assignment](https://docs.python.org/3.10/reference/compound_stmts.html#the-with-statement), [IOBase context protocol](https://docs.python.org/3.10/library/io.html#io.IOBase), [function creation](https://docs.python.org/3.10/reference/compound_stmts.html#function-definitions), and [annotation/runtime distinction](https://docs.python.org/3.10/library/typing.html#typing.get_type_hints).

The exact source dictionaries, complete observed/intended arrays and declared execution environments are retained in cases.toml certificates. [Actual protocol controls](../../profiles/RUF051/evidence/context-target-protocol-py310.json), [valid manager controls](../../profiles/RUF051/evidence/context-target-valid-context-py310.json), [value/annotation controls](../../profiles/RUF051/evidence/context-target-values-py310.json) and [shared semantic contract](../../profiles/RUF051/evidence/context-target-contract.json) are source bound. Python3.13 controls cover supported generic syntax; Python3.10 retains unrelated unsupported-header findings. No upstream issue was filed.
