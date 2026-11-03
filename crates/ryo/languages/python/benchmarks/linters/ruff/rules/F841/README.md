# F841 checker scope and directive policy

## Current discrepancy review (2026-11-03)

The complete 106-case profile runs on sealed worker `82341f24076f38c30dd593b0d5084665470bf8d6f63b8596ed9841f71b8ee107`: 77 raw-exact cases and 29 exact reference discrepancies. All source bytes, full upstream fixtures and selected `F841` findings are retained. No binding/query change or `F841` finding is waived. Source-bound proof: [summary](../../results/ci-80be-full-inventory-2026-11-03/f811-f841-target-supplement/summary.json), [raw receipts](../../results/ci-80be-full-inventory-2026-11-03/f811-f841-target-supplement/raw-receipts.tar.gz). The original exact80be failures remain in that archive.

These differences concern frontend validity and reporting, not an assertion that every output on an invalid program is an upstream bug. A valid AST can retain `validation` / `preserved` diagnostics and complete facts. The reporting policy chooses one original name-token span per global/nonlocal conflict, the complete unsupported type-parameter bracket span, and separate unsupported-keyword / prohibited-lazy-future-statement spans. CPython's first compiler error does not establish exhaustive error count or these ranges.

The late/nested future-placement cause has the dedicated F404 namespace; unknown features have F407. Those complete facts are retained even when those codes are unselected. A prohibited lazy future statement is a separate generic cause; that case below needed an actual candidate fix and now matches Ruff, not a discrepancy waiver.

Primary basis: [Python3.10 declarations/future grammar](https://docs.python.org/3.10/reference/simple_stmts.html#future-statements), [PEP695: syntax introduced3.12](https://peps.python.org/pep-0695/), [PEP810: lazy syntax3.15 and future prohibition](https://peps.python.org/pep-0810/), and [typing expressions versus Python grammar](https://typing.python.org/en/latest/spec/annotations.html#type-and-annotation-expressions). The relative-future case is specifically the [older CPython bug118216](https://github.com/python/cpython/issues/118216), [fixed in3.13 by118267](https://github.com/python/cpython/pull/118267), not intended semantics to copy. Actual CPython3.10 reproduces that bug; it is not offered as normative support. For `.pyi`, grammar acceptance does not certify portable or well-typed stub expressions.

Independent original/compiler controls, changed positive counterparts and the static token/AST oracle are linked in [the reporting contract](../../profiles/F841/evidence/validity-reporting-contract.json). Forty source observations are examined across F811/F841. The oracle's Python3.13 parser supplies ranges only, not Python3.10 runtime evidence; target3.10 executions and the versioned language basis are separate. No upstream issue was created by this work.

### Reproduce one case

From this directory, replace the example case name below. The script writes the exact canonical file and prints both explicit expectations; it does not execute the program.

```sh
python3 - 'future-scope-relative-future-flag' <<'PYCASE'
import pathlib, sys, tomllib
case = next(c for c in tomllib.loads(pathlib.Path('cases.toml').read_text())['cases'] if c['name'] == sys.argv[1])
for name, source in case['files'].items():
    destination = pathlib.Path('repro') / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(source)
print('intended:', case['expected'])
print('pinned reference:', case.get('reference-discrepancy', {}).get('expected', case['expected']))
PYCASE
ruff check --isolated --no-cache --target-version py310 --select F841 --output-format json -- repro/case.py
```

Use the listed `.pyi` filename for stub rows. The observation below lists normalized UTF-8 byte ranges; raw CLI JSON and exact tool/settings/source hashes are preserved in the receipt archive and certificates.

| Case / exact reproducer | Observed pinned Ruff | Intended complete output | Basis |
|---|---|---|---|
| [cross-PLE0117-own-global-conflict-with-real-outer-binding](cases.toml#L272) | `[]` | `invalid-syntax@case.py:80..85` | Validity reporting contract |
| [nonlocal-policy-global-no-valid-nonlocal](cases.toml#L303) | `[]` | `invalid-syntax@case.py:33..38` | Validity reporting contract |
| [nonlocal-policy-nonlocal-then-global](cases.toml#L310) | `[]` | `invalid-syntax@case.py:43..48` | Validity reporting contract |
| [nonlocal-policy-skip-class-namespace](cases.toml#L359) | `[]` | `invalid-syntax@case.py:85..90` | Validity reporting contract |
| [nonlocal-policy-deleted-outer](cases.toml#L366) | `[]` | `invalid-syntax@case.py:69..74` | Validity reporting contract |
| [nonlocal-policy-annotation-outer](cases.toml#L373) | `[]` | `invalid-syntax@case.py:60..65` | Validity reporting contract |
| [nonlocal-policy-invalid-outer-nonlocal](cases.toml#L380) | `[]` | `invalid-syntax@case.py:65..70` | Validity reporting contract |
| [nonlocal-policy-default-real-binding](cases.toml#L387) | `[]` | `invalid-syntax@case.py:65..70` | Validity reporting contract |
| [nonlocal-policy-nearer-function-over-type-parameter](cases.toml#L406) | `invalid-syntax@case.py:9..12` | `invalid-syntax@case.py:9..12`<br>`invalid-syntax@case.py:76..77` | Validity reporting contract |
| [nonlocal-policy-class-type-parameter](cases.toml#L413) | `invalid-syntax@case.py:7..10` | `invalid-syntax@case.py:7..10`<br>`invalid-syntax@case.py:48..49` | Validity reporting contract |
| [nonlocal-policy-nested-function-type-parameter](cases.toml#L420) | `invalid-syntax@case.py:9..12` | `invalid-syntax@case.py:9..12`<br>`invalid-syntax@case.py:48..49` | Validity reporting contract |
| [nonlocal-policy-local-stub-class-header-is-late](cases.toml#L427) | `[]` | `invalid-syntax@case.pyi:83..88` | Syntax/typing distinction |
| [nonlocal-policy-early-annotation-before-late-future](cases.toml#L434) | `[]` | `invalid-syntax@case.py:50..55` | Validity reporting contract |
| [nonlocal-policy-future-annotation-not-visible](cases.toml#L441) | `invalid-syntax@case.py:44..57` | `invalid-syntax@case.py:44..57`<br>`invalid-syntax@case.py:85..90` | Validity reporting contract |
| [nonlocal-policy-type-scope-class-body-py](cases.toml#L448) | `[]` | `invalid-syntax@case.py:50..55` | Validity reporting contract |
| [nonlocal-policy-type-scope-class-body-pyi](cases.toml#L455) | `[]` | `invalid-syntax@case.pyi:50..55` | Syntax/typing distinction |
| [nonlocal-policy-type-scope-class-method-py](cases.toml#L462) | `[]` | `invalid-syntax@case.py:66..71` | Validity reporting contract |
| [nonlocal-policy-type-scope-class-method-pyi](cases.toml#L469) | `[]` | `invalid-syntax@case.pyi:66..71` | Syntax/typing distinction |
| [nonlocal-policy-type-scope-annotation-py](cases.toml#L476) | `[]` | `invalid-syntax@case.py:50..55` | Validity reporting contract |
| [nonlocal-policy-type-scope-annotation-pyi](cases.toml#L483) | `invalid-syntax@case.pyi:9..22` | `invalid-syntax@case.pyi:50..55` | Syntax/typing distinction |
| [nonlocal-policy-type-scope-parameter-py](cases.toml#L490) | `invalid-syntax@case.py:5..8` | `invalid-syntax@case.py:5..8`<br>`invalid-syntax@case.py:32..33` | Validity reporting contract |
| [nonlocal-policy-type-scope-parameter-pyi](cases.toml#L497) | `invalid-syntax@case.pyi:5..8` | `invalid-syntax@case.pyi:5..8`<br>`invalid-syntax@case.pyi:32..33` | Syntax/typing distinction |
| [nonlocal-policy-delayed-annotation-keeps-outer-value](cases.toml#L504) | `[]` | `invalid-syntax@case.py:73..78` | Validity reporting contract |
| [future-scope-later-definition](cases.toml#L517) | `invalid-syntax@case.py:83..96` | `[]` | Validity reporting contract |
| [future-scope-stub-remains-postponed](cases.toml#L530) | `invalid-syntax@case.pyi:9..22` | `[]` | Syntax/typing distinction |
| [future-scope-function-scoped-import](cases.toml#L543) | `invalid-syntax@case.py:63..76` | `[]` | Validity reporting contract |
| [future-scope-class-scoped-import](cases.toml#L550) | `invalid-syntax@case.py:55..68` | `[]` | Validity reporting contract |
| [future-scope-nested-future-blocks-type-scope-walrus](cases.toml#L605) | `invalid-syntax@case.py:59..72` | `invalid-syntax@case.py:102..107` | Validity reporting contract |
| [future-scope-relative-future-flag](cases.toml#L612) | `invalid-syntax@case.py:45..58` | `[]` | Known older-compiler bug |
| [future-scope-lazy-future-does-not-enable](cases.toml#L625) | `invalid-syntax@case.py:0..4`<br>`invalid-syntax@case.py:0..39` | `invalid-syntax@case.py:0..4`<br>`invalid-syntax@case.py:0..39` | Actual candidate cause-projection fix; no reference discrepancy |

## Historical checker policy and earlier receipts

The notes below describe earlier query-policy and worker snapshots; their old case counts are not a proof of the current profile.
The query follows Ruff 0.16.5 current scope bindings and checker analysis timing. Shared namespace/nonlocal fragments distinguish real outer bindings from an invalid own-scope nonlocal, preserve function defaults, and model type-parameter and annotation/class-header type scopes. Deferred reads prefer actual module declarations and materialized builtins; later annotations retain existing bindings. F841 counts actual declaration uses and ignores valid redirected locals. PLW0602 reports only the surviving Global binding, with later assignment/deletion/global/valid-nonlocal effects applied in order.

All 44 prior live-integration controls remain unchanged. The canonical profile has 81 controls, including final frozen cross-rule reproductions and 31 new source-order, nonlocal, builtin, directive-order, and type-scope controls. Complete frozen 310-rule arrays remain in `cross-profile-cases.toml`; full 310-rule arrays for new controls remain in `new-policy-full-cases.toml`. New reference oracles and raw outputs are sealed under `/tmp/nonlocal-policy-oracles-sol/`. Existing reference inputs are reused, not rerun.

Actual old frozen worker `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b` evaluates every case completely with zero unknown/unread/skipped pairs. All expected F841 findings match. One retained control (`nonlocal-policy-early-annotation-before-late-future`) still has an unrelated provider `invalid-syntax` extra at bytes 9..22 because that worker applies a later future import retroactively. The frontend owner is correcting that source-order validation and will replay these exact arrays with a fresh worker. Until that receipt exists this is a query-policy integration handoff, not a fully passing profile or parity certificate. Old-worker evidence: `/tmp/f841-candidate-only-sol/cases.jsonl`.

Scoped future follow-up: the canonical union now has 106 cases. `future-context-full-cases.toml` retains the full 310-rule reports for additional ordered/nested future controls, including the frontend owner's eight regression sources. Existing sources reuse sealed reference records; 25 genuinely new sources were checked once against the pinned full 310-rule reference selection. Shared helpers now follow statement/exception restoration, inherited definition snapshots, whole annotation-root deferral, runtime PEP 613/TypeVar boundaries, and skipped operands. Old-worker candidate-only evidence under `/tmp/future-scoped-policy-oldworker-sol/F841/` has complete coverage and zero differences for this rule; nine cases retain only the known frontend syntax-validation differences (early, nested, relative, or lazy future behavior). Final whole-profile pass status awaits the frontend owner's fresh-worker replay. Every new invocation persists its exact request, profile, module registry, source input, and raw response; historical missing request metadata was not reconstructed as if recorded.
