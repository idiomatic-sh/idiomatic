# F811 checker binding policy

## Current discrepancy review (2026-11-03)

The complete 120-case profile runs on sealed worker `82341f24076f38c30dd593b0d5084665470bf8d6f63b8596ed9841f71b8ee107`: 111 raw-exact cases and 9 exact reference discrepancies. All source bytes, full upstream fixtures and selected `F811` findings are retained. No binding/query change or `F811` finding is waived. Source-bound proof: [summary](../../results/ci-80be-full-inventory-2026-11-03/f811-f841-target-supplement/summary.json), [raw receipts](../../results/ci-80be-full-inventory-2026-11-03/f811-f841-target-supplement/raw-receipts.tar.gz). The original exact80be failures remain in that archive.

These differences concern frontend validity and reporting, not an assertion that every output on an invalid program is an upstream bug. A valid AST can retain `validation` / `preserved` diagnostics and complete facts. The reporting policy chooses one original name-token span per global/nonlocal conflict, the complete unsupported type-parameter bracket span, and separate unsupported-keyword / prohibited-lazy-future-statement spans. CPython's first compiler error does not establish exhaustive error count or these ranges.

The late/nested future-placement cause has the dedicated F404 namespace; unknown features have F407. Those complete facts are retained even when those codes are unselected. A prohibited lazy future statement is a separate generic cause; that case below needed an actual candidate fix and now matches Ruff, not a discrepancy waiver.

Primary basis: [Python3.10 declarations/future grammar](https://docs.python.org/3.10/reference/simple_stmts.html#future-statements), [PEP695: syntax introduced3.12](https://peps.python.org/pep-0695/), [PEP810: lazy syntax3.15 and future prohibition](https://peps.python.org/pep-0810/), and [typing expressions versus Python grammar](https://typing.python.org/en/latest/spec/annotations.html#type-and-annotation-expressions). The relative-future case is specifically the [older CPython bug118216](https://github.com/python/cpython/issues/118216), [fixed in3.13 by118267](https://github.com/python/cpython/pull/118267), not intended semantics to copy. Actual CPython3.10 reproduces that bug; it is not offered as normative support. For `.pyi`, grammar acceptance does not certify portable or well-typed stub expressions.

Independent original/compiler controls, changed positive counterparts and the static token/AST oracle are linked in [the reporting contract](../../profiles/F811/evidence/validity-reporting-contract.json). Forty source observations are examined across F811/F841. The oracle's Python3.13 parser supplies ranges only, not Python3.10 runtime evidence; target3.10 executions and the versioned language basis are separate. No upstream issue was created by this work.

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
ruff check --isolated --no-cache --target-version py310 --select F811 --output-format json -- repro/case.py
```

Use the listed `.pyi` filename for stub rows. The observation below lists normalized UTF-8 byte ranges; raw CLI JSON and exact tool/settings/source hashes are preserved in the receipt archive and certificates.

| Case / exact reproducer | Observed pinned Ruff | Intended complete output | Basis |
|---|---|---|---|
| [cross-PYI034-metaclass-function-annotation-typescope](cases.toml#L242) | `invalid-syntax@case.pyi:74..84` | `[]` | Syntax/typing distinction |
| [cross-PYI019-metaclass-function-annotation-typescope](cases.toml#L249) | `invalid-syntax@case.pyi:74..84` | `[]` | Syntax/typing distinction |
| [future-scope-early-definition](cases.toml#L568) | `[]` | `invalid-syntax@case.py:50..55` | Validity reporting contract |
| [future-scope-later-definition](cases.toml#L581) | `invalid-syntax@case.py:83..96` | `[]` | Validity reporting contract |
| [future-scope-stub-remains-postponed](cases.toml#L594) | `invalid-syntax@case.pyi:9..22` | `[]` | Syntax/typing distinction |
| [future-scope-function-scoped-import](cases.toml#L607) | `invalid-syntax@case.py:63..76` | `[]` | Validity reporting contract |
| [future-scope-class-scoped-import](cases.toml#L614) | `invalid-syntax@case.py:55..68` | `[]` | Validity reporting contract |
| [future-scope-nested-future-blocks-type-scope-walrus](cases.toml#L669) | `invalid-syntax@case.py:59..72` | `invalid-syntax@case.py:102..107` | Validity reporting contract |
| [future-scope-relative-future-flag](cases.toml#L676) | `invalid-syntax@case.py:45..58` | `[]` | Known older-compiler bug |
| [future-scope-lazy-future-does-not-enable](cases.toml#L689) | `invalid-syntax@case.py:0..4`<br>`invalid-syntax@case.py:0..39` | `invalid-syntax@case.py:0..4`<br>`invalid-syntax@case.py:0..39` | Actual candidate cause-projection fix; no reference discrepancy |

## Historical checker policy and earlier receipts

The notes below describe earlier query-policy and worker snapshots; their old case counts are not a proof of the current profile.
The query follows Ruff 0.16.5 `redefined_while_unused`, `Binding::redefines`, `SemanticModel::shadowed_bindings`, and checker binding creation. Function methods are included as function definitions while keeping their concrete scope and analysis phase. Same-namespace comparisons follow immediate predecessor bindings; used references copy forward only when the use predates the new binding, and deletion/exception cleanup breaks that inheritance.

Stub class headers and other deferred type reads resolve at their actual final checker context through `deferred-read-events`, so a later rebinding can be used instead of the earlier class/import. Class-header and function-annotation walrus bindings remain transient TypeScope events. Function defaults and decorators retain ordinary evaluation behavior. Distinct elif/else clause identities are distinct branch arms.

The policy retains overload, dummy-name, property-use, cross-class, and explicit-export behavior. Dotted unaliased submodule imports redefine only the same qualified import, and their diagnostic uses the real full alias source range. Future imports and loop-variable bindings are excluded where `Binding::redefines` requires; they still participate in predecessor blocking. Source scope/phase constraints are preserved.

All 32 original live-integration cases remain unchanged. The scoped profile now contains 94 cases: final frozen cross-rule sources plus 37 added positive/negative controls. Candidate-only replay against retained complete reference records passes 94/94, with 188 complete records and zero unknown/unread/skipped pairs, on actual frozen worker SHA `65b5a3933be11bb735c9bfb805451ee378581bec1831daf245d3d37e92e5d94b`. Evidence: `f811-candidate-only-sol/cases.jsonl`; each reused reference identifies its original actual contestant receipt. New controls additionally have actual benchmark reference/candidate records under `f811-new-control-sol/` and `f811-new-kind-control-sol/`.

A full 310-query candidate-only replay covers all 25 distinct final affected sources, paired with the sealed original reference records: 50 complete records, zero unknown/unread/skipped pairs, zero F811 differences. Twenty-three full reports match exactly. The remaining two reports retain unrelated frozen PLE0118 extras and F841/PYI020 missing findings; these are not hidden by the scoped profile. Full arrays remain in `cross-profile-cases.toml`; evidence is `f811-final-full310-sol/{cases.jsonl,result.json}`. This is scoped rule evidence, not a 413-rule parity certificate.

Scoped future follow-up: the canonical union now has 120 cases. `future-context-full-cases.toml` retains the full 310-rule reports for additional ordered/nested future controls, including the frontend owner's eight regression sources. Existing sources reuse sealed reference records; 25 genuinely new sources were checked once against the pinned full 310-rule reference selection. Shared helpers now follow statement/exception restoration, inherited definition snapshots, whole annotation-root deferral, runtime PEP 613/TypeVar boundaries, and skipped operands. Old-worker candidate-only evidence under `future-scoped-policy-oldworker-sol/F811/` has complete coverage and zero differences for this rule; nine cases retain only the known frontend syntax-validation differences (early, nested, relative, or lazy future behavior). Final whole-profile pass status awaits the frontend owner's fresh-worker replay. Every new invocation persists its exact request, profile, module registry, source input, and raw response; historical missing request metadata was not reconstructed as if recorded.
