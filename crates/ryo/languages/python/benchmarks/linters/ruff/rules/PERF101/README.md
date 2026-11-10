Pinned source: Ruff 0.16.5, tag commit `9e4938c4a60bed3e87a11ee1e1db1bd23f4d964a`; target Python 3.10, stable default settings. All prior expected arrays remain unchanged, including the formerly failing recursive-initializer examples and complete tagged upstream fixture.

The candidate preserves initializer shape (tuple/list/set literals), async dispatch, after-body binding lookup, identifier-based mutation checks, direct expression-call requirements, and upstream augmented-subscript behavior. Annotated assignments use their values for this helper. The shared assigned-value query now handles arbitrary nested target/value pairing, including the two previously missing findings inside the tagged upstream fixture.

Validation: 53/53 exact cases pass on actual benchmark worker `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`. Each case has complete reference/candidate records and zero unknown rule/file pairs. This bounded evidence is not a whole-corpus parity certificate. Receipts/hashes: `assigned-type-20261101/verification.json`. No prepared-manifest registration changes.

## Shared context-target inference discrepancy

A with-target receives enter/aenter result, not its manager. The exact source below uses a builtin manager lacking the protocol; no legitimate target type is lost. This corrects unknown-result inference, rather than waiving diagnostics merely because a program has runtime errors. Valid self-return/different-return managers and standard IOBase self-return controls are retained.

### with-initializer

```python
with [1] as items:
    for x in list(items):
        print(x)
```

Pinned Ruff0.16.5: PERF101 32..43. Intended: none.

With and async-with assign __enter__/__aenter__ result, not the context manager. Builtin dict/set/list managers in these exact7sources have neither method. No legitimate target type is lost: no target value exists. This is an unknown-result inference correction, not a blanket rule suppression whenever any runtime error exists. The diagnostic requires a known result receiver type; a manager initializer supplies no such evidence. Valid self-return/different-return controls preserve the uncertainty rather than pretending all managers return themselves.

Primary basis: [with-result assignment](https://docs.python.org/3.10/reference/compound_stmts.html#the-with-statement) and [IOBase self-return](https://docs.python.org/3.10/library/io.html#io.IOBase). [Exact actual protocol controls](../../profiles/PERF101/evidence/context-target-protocol-py310.json), [valid manager controls](../../profiles/PERF101/evidence/context-target-valid-context-py310.json) and [shared contract](../../profiles/PERF101/evidence/context-target-contract.json) preserve original sources, explicit environments and version differences. The complete reference multiset remains in the case certificate. Strict replay remains required; no upstream issue was filed.

Targeted current-source proof and raw archive retain every original fixture source and complete affected-consumer replays. A fresh shared full413 baseline is required before publication; historical receipts are unchanged.
