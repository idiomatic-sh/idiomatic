142 full suppression implementations now call `python/ruff/suppress`. Candidate
construction, the exact inline/range regex literals, final columns and emit keys
are preserved. B026 retains RUF004 aliases; TRY203 retains TRY302 aliases. EXE005
and PYI033 still project `file, span` and emit `key(span)`. The measured helper is
unchanged (`72eaeed41d5a9a6393d5048cb7bf7389479a9ce622f4748db92f980e724a8b80`).

This batch excludes F401/F822 (parent-line suppression), SIM220/SIM221 (extra key
columns) and the eight unmapped variants. UP005's unused old mapping prelude is
also outside this migration. The scoped policy inventory lives in
`benchmarks/linters/ruff/results/standard-suppression-policy.json`; historical
combined snapshots retain their original sources. Canonical staged profiles must
declare and physically stage the helper, import it, and emit a source-binding
result that depends on a call. Declaration-only/dead calls and copied full bodies
are rejected. This source dependency guard does not claim arbitrary IR reachability.

`proof.json` seals 2,942 unchanged cases / 5,884 real baseline and candidate worker
processes on the existing 2ea actual benchmark worker. Every process exited zero;
full reports (including order, file inventory, coverage and skips) were identical;
Unknown was zero. Eight side records contain legitimate accepted-syntax skips.
Pinned expected diagnostics are compared with the shared `diagnostic_multiset`
normalizer, preserving duplicates and supplying implicit enclosing rule codes.
One existing F841/nonlocal-policy-early-annotation-before-late-future expectation
difference remains on both sides; its complete arrays are retained. This proves
migration equivalence, not full Ruff parity or a wall-time improvement. Ruff was
not invoked again.

Raw requests/stdout/stderr and immutable profiles are retained at
`/tmp/suppress-migration-20261101-v2`. `execution-manifest.json` binds worker,
module registry and all module bytes. `source-receipt.json` binds each query pair,
unchanged prelude, regex literals and emit clause. `seal.py` validates every raw
record independently with the captured validator, complete exact-once identities,
case hashes, all physically staged module bytes, and existing case normalization.
For a new empty output directory, replay against the exact pre-migration query
checkout and candidate checkout with:

```
python3 verify.py --candidate-root /candidate --baseline-root /baseline \
  --worker /path/to/ruff-2ea... --output /new/immutable/run
```

The first logical-staging attempt was rejected: workers refused a symlinked module
directory. Its erroneous equality summary was revoked. The original refusal raw
file was overwritten before archival and is unavailable; it was not reconstructed.
The incomplete physical retry and revoked summary/log are explicitly archived in
`/tmp/suppress-migration-20261101-rejected-v1/REVOKED.txt`. Neither is acceptance
proof. Permanent validator negatives reject equal nonzero refusals, symlink
refusals, empty stdout, malformed schema, missing files and truncated/Unknown
coverage. The sealed v2 raw hash differs from that revoked run.
