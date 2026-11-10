# PLC0206

## Shared context-target inference discrepancy

A with-target receives enter/aenter result, not its manager. The exact source below uses a builtin manager lacking the protocol; no legitimate target type is lost. This corrects unknown-result inference, rather than waiving diagnostics merely because a program has runtime errors. Valid self-return/different-return managers and standard IOBase self-return controls are retained.

### assignment-with

```python
with {} as d:
 pass
for k in d:
 print(d[k])
```

Pinned Ruff0.16.5: PLC0206 24..30. Intended: none.

With and async-with assign __enter__/__aenter__ result, not the context manager. Builtin dict/set/list managers in these exact7sources have neither method. No legitimate target type is lost: no target value exists. This is an unknown-result inference correction, not a blanket rule suppression whenever any runtime error exists. The diagnostic requires a known result receiver type; a manager initializer supplies no such evidence. Valid self-return/different-return controls preserve the uncertainty rather than pretending all managers return themselves.

Primary basis: [with-result assignment](https://docs.python.org/3.10/reference/compound_stmts.html#the-with-statement) and [IOBase self-return](https://docs.python.org/3.10/library/io.html#io.IOBase). [Exact actual protocol controls](../../profiles/PLC0206/evidence/context-target-protocol-py310.json), [valid manager controls](../../profiles/PLC0206/evidence/context-target-valid-context-py310.json) and [shared contract](../../profiles/PLC0206/evidence/context-target-contract.json) preserve original sources, explicit environments and version differences. The complete reference multiset remains in the case certificate. Strict replay remains required; no upstream issue was filed.

[Targeted current-source proof and raw archive](../../results/context-target-inference-2026-11-03/README.md) retain every original fixture source and complete affected-consumer replays. A fresh shared full413 baseline is required before publication; historical receipts are unchanged.
