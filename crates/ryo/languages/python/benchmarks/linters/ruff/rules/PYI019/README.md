## Earlier scoped verification (historical)

The policy follows Ruff 0.16.5 `custom_type_var_for_self.rs` and the shared
`analyze/class.rs` metaclass check. PYI019 requires definite metaclass status;
PYI034 skips definite and possible metaclasses. The shared helper preserves
ordered depth first edge visits, including calls visited before the first
successful base and edges into already visited classes. The method node is the
entry vertex so the starting class is initially absent from Ruff's seen set.

Qualified origins intentionally distinguish cached `.py` class bases, current
`.pyi` PYI034 analysis, and deferred `.pyi` PYI019 analysis. A deferred load before
a later deletion restores only the immediately shadowed non-unbound binding,
as Ruff `SemanticModel::resolve_load` does; a deletion before the load remains
unbound. `lookup-symbol-events` retains real event occurrences and lexical scope
identity. It handles transient annotation/header TypeScopes separately from real
namespace bindings. These are caller policy choices, not ordinary latest-event
lookup.

The retained suites contain 139 PYI019 cases and 72 PYI034 cases, including the
original 21 PYI034 cases, upstream fixtures, Unicode and parenthesized ranges,
class header/function annotation scopes, cyclic ordered traversal, aliases,
local members, rebinding, and deletion controls. On worker SHA
`65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`, all 422
reference/candidate records are complete and pass (target Python 3.10). Receipts:
`/tmp/pyi019-sol-seven/cases.jsonl` and `/tmp/pyi034-sol-seven/cases.jsonl`.
This evidence covers the retained cases rather than universal semantic parity.

## Reference discrepancy report

**Reference:** pinned Ruff 0.16.5 (`python-target=3.10`; [profile](../../profiles/PYI019/benchmark.toml)). **Scope:** Python grammar acceptance versus restricted stub type-expression policy; this is a scope distinction, not a claim that the stub is portable or well typed.

The 1 certified case below retains their exact source bytes and full intended and observed finding arrays in [cases.toml](cases.toml). The [archived raw receipt](../../results/ci-80be-full-inventory-2026-11-03/pyi019-pyi034-target-supplement/raw-receipts.tar.gz) records an executed worker snapshot; the source and reference-output hashes in each case certificate bind the precise observation. A historical combined receipt may predate a later targeted certificate; follow the certificate identity for that case. Diagnostic spans below are UTF-8 byte offsets.

To reproduce Ruff's reference finding for `metaclass-function-annotation-typescope`, write its exact named file from [cases.toml](cases.toml) into an isolated directory and run the pinned Ruff 0.16.5 executable (SHA256 `52469aac0ad5cd499c3cd1f72993a9f3eb237fd975f53d6873b4245190e7d0e8`):

```sh
RAYON_NUM_THREADS=1 /home/vscode/.local/bin/ruff check --isolated --output-format json --cache-dir ./ruff-cache --select PYI019 --target-version py310 -- case.pyi
```

### Documented source-specific difference

Cases (1): `metaclass-function-annotation-typescope`.

Representative exact source, `metaclass-function-annotation-typescope` (`case.pyi`):

```python
from typing import TypeVar
T=TypeVar("T")
class A(type): ...
def outer(x:(A:=unknown)):
 class Subject(A):
  def m(self:T): ...
```

- Intended: none. Pinned Ruff observed: invalid-syntax case.pyi:74–84.
- Basis: Ryo records Python grammar as accepted while pinned Ruff reports generic invalid syntax for a named expression in a stub annotation; this does not establish stub typing validity or a Ruff bug. Primary evidence: [audit.py](../../profiles/PYI019/evidence/audit.py), [exact-current-failed-sources.json](../../profiles/PYI019/evidence/exact-current-failed-sources.json).
- Reproduce: use the [profile](../../profiles/PYI019/benchmark.toml), exact named source in [cases.toml](cases.toml), and [raw receipt](../../results/ci-80be-full-inventory-2026-11-03/pyi019-pyi034-target-supplement/raw-receipts.tar.gz); the case's `reference-discrepancy.identity` binds executable, settings, source, and output hashes.
