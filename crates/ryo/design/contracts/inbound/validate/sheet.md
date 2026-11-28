# inbound/validate
STATUS: drafted (28 Nov; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.19.

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). Read only through
`git -C /workspaces/idiomatic-public-benchmark show|grep|ls-tree|log`. Every quote is one line, two spaces,
`@rev:path:line`. Short paths below: `core/` = `crates/ryo/crates/core/src/`, `C/` = `crates/ryo/crates/`.

## 1. Owns
The verdict on one RyoQL query: whether it is valid against the relations ryo owns, the relations the host declares
and the kinds of the linked languages, and, when it is, what it uses (relations read, operators used, steps of the
plan that runs). One thing: "the report on a query". It does not read text (ryoql), lower or judge the plan (plan),
or know a language (the kinds are handed in as a value).

## 2. On main (f4ae3bd802)
**Where.** `core/inbound/use_cases/validate/`: `interface.rs` 18, `types.rs` 15, `errors.rs` 29, `use_case.rs` 83 lines
(145 without tests); tests `use_case_test.rs` 378, `acceptance.rs` 79, `acceptance.feature` 26. The report lives in
`values`: `C/values/src/domain/descriptions/types.rs:173-185`.

**Public items: 7** (+1 trait method, 3 error variants): `ValidateUseCase`, `ValidateRequest`, `ValidateError`,
`ValidateUseCaseImpl`, `ValidateUseCaseImpl::new`, and in values `ValidateReport`, `StepCount`.

**The contract, word for word.**
```
pub trait ValidateUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/interface.rs:8
fn validate(&self, request: ValidateRequest) -> Result<ValidateReport, ValidateError>;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/interface.rs:17
pub struct ValidateRequest {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/types.rs:8
pub enum ValidateError {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/errors.rs:8
Uncompilable {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/errors.rs:17
pub struct ValidateReport {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:178
pub struct StepCount(pub usize);  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:174
```
`ValidateRequest { observations, text: QueryBundle, hosted: Vec<HostedRelation> }`; `ValidateError::{Refused { error:
RefusedQuery }, Uncompilable { error: RefusedQuery }, UnknownKind { reason: String, origin: Option<QueryOrigin> }}`;
`ValidateReport { relations: Vec<RelationName>, operators: Vec<OperatorName>, steps: StepCount }`.

**Implementor.** One, `ValidateUseCaseImpl`, over two ports:
```
pub struct ValidateUseCaseImpl {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:17
pub fn new(  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:34
```
taking `Arc<dyn QueryLanguagePort>` (execution's port over ryoql's compile and typecheck) and `Arc<dyn LanguagePort>`
(core's language port, of which it calls one method):
```
fn kind_vocabularies(&self) -> KindVocabularies;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/language/interface.rs:29
```
It validates the text, and when a language is linked compiles it a second time and runs main's kind check:
```
.compile(&request.text, &request.hosted, &request.observations)  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:61
QueryLanguageError::Refused { error } => ValidateError::Uncompilable { error },  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:63
&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new(),  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:69
```

**Callers (files per crate).** cli 2 (`resources/query/validate/{command,composition}.rs`); sdk 3
(`resources/query/validate/{entry,composition,schemas}.rs`), whose public free function is
```
pub fn validate(  @f4ae3bd802:crates/ryo/crates/sdk/src/resources/query/validate/entry.rs:17
```
and that function's callers: Tastemaker's analysis adapter (`match ryo::query::validate(` at
`crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:72`), the Ruff benchmark's
`authored_queries_test.rs` (2 calls), the command line's replay test `cli/tests/replay/sdk.rs:220`.

**Proof.** 3 test functions (`succeeds`, `refuses`, `fails`) over 24 case rows (20 succeed: a valid query, no language
linked, a hosted relation, and the example of each of 17 operators; 3 refuse: syntax, undeclared host relation, unknown
kind; 1 fails: a correct query the compiler refuses is `Uncompilable`), and 2 scenario outlines with 5 example rows:
```
fn succeeds() {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case_test.rs:366
Feature: validate  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/acceptance.feature:1
```
Edge tests: sdk `entry_test.rs` 2, `schemas_test.rs` 2; cli `command_test.rs` 2, `schemas_test.rs` 2.

**Tastemaker entries.** None names the use case. The `idiomatic.arch/sdk` entry in `crates/ryo/tastemaker.toml:134`
lists the SDK's re-exports `validate`, `ValidateError`, `ValidateRequest`, `ValidateReport`; two Asymptote lock files
carry `no-bare-text` entries on `crates/sdk/src/resources/query/validate/types.rs`.

## 3. On the perf branch (2a207464c)
**Where.** Same folder: `interface.rs` 18, `types.rs` 15, `errors.rs` 29, `use_case.rs` 57 (119 without tests). **No
test file**: the three test files went in fd6122c42a. Report in values, `descriptions/types.rs:202-214`.

**Public items: 7** (+1, 3 variants), the same names. `interface.rs`, `types.rs`, `errors.rs` are byte-identical to main:
```
pub trait ValidateUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/interface.rs:8
fn validate(&self, request: ValidateRequest) -> Result<ValidateReport, ValidateError>;  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/interface.rs:17
pub struct ValidateRequest {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/types.rs:8
pub enum ValidateError {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/errors.rs:8
Uncompilable {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/errors.rs:17
pub struct ValidateReport {  @2a207464c:crates/ryo/crates/values/src/domain/descriptions/types.rs:207
pub struct StepCount(pub usize);  @2a207464c:crates/ryo/crates/values/src/domain/descriptions/types.rs:203
```

**Implementor.** One, over ryoql's typecheck use case and the language port:
```
pub struct ValidateUseCaseImpl {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:18
pub fn new(typecheck: Arc<dyn TypecheckUseCase>, language: Arc<dyn LanguagePort>) -> Self {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:35
.typecheck_kinds(asked, &self.language.kind_vocabularies())  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:51
other => ValidateError::Refused { error: refused(other) },  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:54
fn kind_vocabularies(&self) -> KindVocabularies;  @2a207464c:crates/ryo/crates/core/src/outbound/language/interface.rs:27
```
All the work is in ryoql's `typecheck_kinds` (checks, then `keeper::lowered_plan` through the running plan's lowering,
then the kind check, then the report from the written program). A lowering refusal reaches validate as `Invalid` at
byte 0 and leaves as `Refused`:
```
.map_err(|message| TypecheckError::Invalid { span: nowhere, message })?;  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:144
return Err(format!("unplanned query {}", refused.reason.0));  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/keeper.rs:46
```
So **`ValidateError::Uncompilable` is never built on the tip**, while the SDK and the command line still map it:
```
CoreError::Uncompilable { error } => {  @2a207464c:crates/ryo/crates/sdk/src/resources/query/validate/schemas.rs:229
ValidateError::Uncompilable { .. } => Next::Fix("correct the query text where the message says".to_owned()),  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:65
```

**Callers.** cli 2 (as main) plus the performance bench `cli/benches/performance/query/validate/bench.rs`; sdk 3 (as main).
The SDK's `query::validate` builds the whole composition on every call (language factory, typecheck, syntax, catalog):
```
super::composition::build(languages.0.clone())  @2a207464c:crates/ryo/crates/sdk/src/resources/query/validate/entry.rs:22
```
Its callers: the lint session, to word a rule the executor refused (an edge calling an edge function today),
```
match validate(linked, ValidateRequest { query, hosted, observations: Vec::new() }) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:200
```
Tastemaker's adapter, once per definition whose proof is not current (`tm validate`, and `tm check` through its
`proved` step),
```
match ryo::query::validate(  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:89
let verdict = self.analysis.validate(query)?;  @2a207464c:crates/tastemaker/crates/core/src/inbound/use_cases/validate/use_case.rs:225
```
the Ruff benchmark's `authored_queries_test.rs` (2 calls), `cli/tests/replay/sdk.rs:217`, and the new contract test.

**Proof.** No test in core. At the SDK seam, new on the tip (6e8b89c94e, e21cc5c651): `C/sdk/tests/validate_contract.rs`,
2 tests, one over 31 oracle rows (29 typed refusals, 2 accepted reports with their steps), generated by
`build/ports/stage2/validate-oracle/gen_contract_test.py` in the evidence repository:
```
fn query_validate_answers_the_contract_the_oracle_records() {  @2a207464c:crates/ryo/crates/sdk/tests/validate_contract.rs:142
fn a_kind_no_linked_language_has_is_refused_where_the_comparison_is_written() {  @2a207464c:crates/ryo/crates/sdk/tests/validate_contract.rs:155
```
Edge tests as main (sdk 2 + 2, cli 2 + 2). Tastemaker entries: as main (`tastemaker.toml:124`).

## 4. What changed between them
`git log f4a32acb56..2a207464c` on the folder: 4 commits; main has none after the split.
- **5ea9b6801c** (20 Nov): an import path in the test (values' `Plan` moved to `query_plans`).
- **fd6122c42a** (27 Nov): ryoql's compile use case and execution's `QueryLanguagePort` deleted; validate is handed
  `TypecheckUseCase` instead; **the 3 test files deleted (483 lines: 3 tests, 24 rows, 5 scenario rows)**.
- **f0db2d155a** (27 Nov): "the kind check once, over the plan that runs; validate and the executor both ask it":
  validate's own second compile and main's kind check (`MetadataUseCase::kind_refusal`, deleted with execution at
  c46607e439) replaced by one call, `typecheck_kinds` (-37 lines).
- **6e8b89c94e** (27 Nov): the unknown kind is refused where the comparison is written (`origin` from `located`).

**Lost that should not have been.** (1) The seam's proof: 24 rows and 5 scenarios over fakes, replaced only by the SDK's
oracle test, which runs the real query language and so cannot plant a bug in validate alone. (2) `Uncompilable`'s
producer: a designed variant kept in the type and mapped by two edges, built by nothing. (3) The steps are no longer
main's optimizer's count but the running plan's (oracle row "a computed column, refused by the old checks, is valid",
`StepCount(284)`): a stated, measured change, not a loss.

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
The listing is `api.txt` beside this sheet: 6 public items plus 1 method, 3 variants and 3 impls. The claim it starts
from, plan Appendix A 3.17 ("`ValidateUseCase` [exists] ... composes ryoql's `check`, plan's `lower` and
`unknown_kind`, with the languages' kind vocabularies handed in"), holds for the trait; two corrections: it composes
plan's **`judge`** (the plan sheet folds lower-then-kind-check into that one call, its decision 3), and ryoql's
**`locate`** for the place of an unknown kind.

### 5a. The call (main, unchanged)
`ValidateUseCase::validate(&self, ValidateRequest) -> Result<ValidateReport, ValidateError>` is main's and the tip's,
word for word. Kept because it already is the smallest surface: one call, per request.

### 5b. The types
- `ValidateRequest` keeps its three fields. Their types move with their owners: `QueryBundle` and
  `BoundObservationInput` to ryoql, `HostedRelation` to relations (as ryoql's `QueryRequest` names them). It is not
  ryoql's `QueryRequest`: that carries `modules`, which a caller of validate never sets (the tip sets
  `ParsedModules::default()`, use case line 45).
- `ValidateReport` moves here from values with its fields verbatim; the plan sheet sends it here (its decision 3:
  "the report ... is assembled by inbound/validate"). `steps` is plan's `StepCount`, which leaves values for plan.
- `ValidateError` keeps main's three variants. `Refused` is filled from ryoql's `CheckError` through ryoql's drafted
  `From<CheckError> for RefusedQuery` (today's free fn `refused`). `UnknownKind { reason, origin }`: `reason` is the
  Display of plan's `UnknownKind` (today `unknown_message`), `origin` is ryoql's `locate`. `Uncompilable` is filled
  again, from plan's `PlanError::Unplanned` and `MissingEmit` (decision 2).

### 5c. What the implementation does (one path; nothing else lowers or checks)
```
check.check(QueryRequest { text, hosted, observations, modules: default })   -> CheckedQuery | Refused
lower.judge(&checked, &kinds)                                                -> StepCount | PlanError
  PlanError::UnknownKind(u)          -> UnknownKind { reason: u.to_string(), origin: check.locate(&checked, u.word) }
  PlanError::Unplanned | MissingEmit -> Uncompilable (decision 2)
Ok(ValidateReport { relations: checked.reads, operators: checked.operators, steps })
```
The empty-vocabulary rule ("with no language linked ... the words are not asked") is plan's `judge` given an empty
`KindVocabularies`, as `typecheck_kinds` is today (typecheck `use_case.rs:147`, `if !kinds.0.is_empty()`). Observations: `judge` answers the
checks' own count, as today (plan sheet 5a). So validate holds no rule of its own beyond mapping three answers.

### 5d. How it is built (changed)
`ValidateUseCaseImpl<C: CheckUseCase, L: LowerUseCase>::new(check: C, lower: L, kinds: KindVocabularies)`  `@new`.
Generic, as plan's `LowerUseCaseImpl<C: CompareUseCase>` is, so no `dyn` crosses (the call is per request, so this is for
uniformity, not speed). The language port goes: validate called one of its methods, once per call; the edge, which
already holds the linked languages, asks `kind_vocabularies()` once and hands the value in (decision 1). The impl holds
two use cases and one value: nothing to tear down (perf row 52's "forget the composed services" stays possible).

### 5e. What stops being public, and the counts
Listed in api.txt `# NO LONGER PUBLIC`: both `new` signatures, values' `ValidateReport` (moved) and `StepCount` (to
plan), and what validate called that its neighbours' sheets retire (`typecheck_kinds`, `refused`, main's
`QueryLanguagePort`, main's `kind_refusal`).

| Side | Public items (+ methods, variants) | Where |
|---|---|---|
| main | 7 (+1, 3) | core 5, values 2 |
| tip | 7 (+1, 3; `Uncompilable` never built) | core 5, values 2 |
| drafted | 6 (+1, 3) | this crate; `StepCount` to plan |

## 6. Proof it brings
- **Restated (not carried):** main's `use_case_test.rs` (3 tests, 24 rows) and `acceptance.{feature,rs}` (2 outlines, 5
  rows), deleted at fd6122c42a, come back as seam tests over fakes of `CheckUseCase` and `LowerUseCase` (main faked
  `QueryLanguagePort` and the language port; those ports are gone). The 17 "example of <operator>" rows need the real
  ryoql and plan and belong at the SDK seam beside the oracle test, not here. Planted bugs each catches: a check refusal
  mapped to the wrong variant; a judge refusal swallowed; the unknown kind's origin dropped; `reads`/`operators` taken
  from the wrong program; `steps` not the judge's.
- **Carried as they are:** the SDK's `validate_contract.rs` (2 tests, 31 rows) and the edges' 8 tests stay where they are
  (sdk and cli sheets); they are the byte-identity gate for this build (review criterion 7).
- **Missing today and added:** a row for a plan refusal (`unplanned query ..`), which no row of the 31 has (decision 2).

## 7. Perf gains it carries
`perf-gains.tsv` (54 rows): **no row names inbound/validate** in its crate column. Row by row, the ones that touch
what validate composes:
- **1, 8, 40** (the lowering moved in process, once per pack, in Rust; crates plan, programs, ryoql): validate runs
  plan's `judge` once per request over a one-rule pack, so it neither needs nor blocks "once per pack"; the gains live
  in plan and programs and the draft changes nothing there.
- **17** (Tastemaker submits its lint calls in rounds; inbound/lint, edges/sdk): lint only; validate is not on it.
- **52** (teardown skipped by forgetting composed services; edges): kept, because `ValidateUseCaseImpl` owns no store,
  cache or pool, only two use cases and a value.
- **Room it makes** (not a recorded gain): the SDK builds validate's whole composition on every call
  (`entry.rs:22`), and Tastemaker's cold `tm check` calls it once per unproven definition. With the kinds a value and
  the use cases generic, an edge can build one `ValidateUseCaseImpl` and reuse it; whether that is worth doing is the
  SDK sheet's call and unmeasured here.
**Aim 3, from the history.** The four commits on validate since the split (section 4) were all structure, none perf.
Under this draft: f0db2d155a (the kind check over the plan that runs) would have landed in plan alone (`judge`), with no
line in validate; 6e8b89c94e (where an unknown kind is written) in ryoql's `locate` plus one line here; fd6122c42a
(ryoql's compile deleted) in ryoql and plan only, since validate names neither compile nor lowering. No perf change in
the 48-row history crossed into validate.

## 8. Benchmarks that cover it
| Benchmark | What it calls today | After the draft |
|---|---|---|
| Ruff linter benchmark: `crates/ryo/languages/python/benchmarks/linters/ruff/engine_test/authored_queries_test.rs` | `ryo::query::validate` on every authored query, and expects RUF100 without its observation refused | unchanged: the SDK's free function stays; it composes this crate |
| Clippy, Oxlint linter benchmarks | nothing in validate (grep at both revisions: no `query::validate` under `languages/{rust,typescript}`) | nothing |
| The Tastemaker check benchmark (bar: 5 s) | `tm check` proves every definition (`proved`), one `ryo::query::validate` per unproven definition (adapter.rs:89) | unchanged through the SDK |
| Command-line performance bench `query/validate` (tip only; main deleted the bench in f4ae3bd802 but kept its ratchet) | `ryo_cli::resources::query::validate::command::run`, which builds `ValidateUseCaseImpl` | the command line builds the new impl; the bench is untouched |
| Asymptote ratchet, same benchmark | the line quoted below, at both revisions | unchanged |
| The validate oracle (`sdk/tests/validate_contract.rs`, evidence `build/ports/stage2/validate-oracle/`) | `ryo::query::validate` | unchanged; the build's byte gate for this crate |
```
"query/validate",  @2a207464c:crates/ryo/cli/benches/performance/query/validate/bench.rs:6
{ tool = "performance", benchmark = "query/validate", measure = "cpu-ms", best = 0.022 },  @2a207464c:crates/ryo/.asymptote/operations/bf77391cafe13abc33089896880bbefe47a32fa08ae5c587cf7e93da3d1d3c80.lock:7
{ tool = "performance", benchmark = "query/validate", measure = "cpu-ms", best = 0.022 },  @f4ae3bd802:crates/ryo/.asymptote/operations/bf77391cafe13abc33089896880bbefe47a32fa08ae5c587cf7e93da3d1d3c80.lock:7
let answer = ryo::query::validate(&super::prepared_engine_test::python_310(), request);  @2a207464c:crates/ryo/languages/python/benchmarks/linters/ruff/engine_test/authored_queries_test.rs:99
```
No benchmark reaches an internal name of validate; all go through the SDK's free function or the command line.

## 9. Checks
IMPORTS: relations, operators, ryoql, plan
CROSSED: per request
- Directions: inbound may import domain; all four are domain. No outbound import (the language port is gone, decision
  1), no application import, no other inbound crate. Outside crates: `thiserror` (as today).
- No layout word in any signature: no table, column, row or cell type appears.
- Crossing: `validate` once per request; inside it, `check`, `judge` and at most one `locate`, each once per request.
  Never per file, step, row or cell. No `dyn` (generic impl).
- Review criteria applied: 2 (one path: validate lowers nothing; the plan judges), 3 (6 items), 6 (one `ValidateReport`
  in the core; the SDK's second family is decision 5), 7 (byte identity: decision 2 states its one change), 12
  (`From` for the refusal conversion, `std::error::Error` on the error).

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **The kinds handed in as a value, not the language port.** Example: validate calls one of `LanguagePort`'s ten
   methods, `self.language.kind_vocabularies()` (tip use case line 51), on every call. Why: the smallest surface its
   caller needs; validate would otherwise import `outbound/language` for one method. What we get: no outbound import;
   a test hands a `KindVocabularies`, no fake port. **Recommend: `new(check, lower, kinds: KindVocabularies)`.**
2. **What `Uncompilable` means now.** Example: main builds it when the compiler refuses a correct query (use case
   line 63); the tip never builds it, a lowering refusal leaves as `Refused` with a 0..0 span (typecheck use case
   line 144), yet the SDK (`schemas.rs:229`) and the command line (`remedy.rs:65`) still map it. Why: a designed variant
   with no producer is hollow; dropping it is the owner's call. What we get by filling it from plan's
   `Unplanned`/`MissingEmit`: main's meaning back; one stated byte change at the SDK, the span of such a refusal becomes
   the whole text (`fn invalid`, "it is wrong as a whole, so the span is the text"), proved by a new oracle row.
   **Recommend: fill it, with the row.** (The other way: keep the tip's bytes and delete the variant from three crates.)
3. **Lint's refused-rule wording stays at the SDK edge.** Example: the lint session calls validate to word a rule the
   executor refused (`entry.rs:200`). Why: once the session is `inbound/lint`, it may not import `inbound/validate`;
   calling ryoql and plan itself would be a second validate. What we get: lint returns the refusal it has
   (`Unrunnable { id, reason }`); the SDK, which composes both, asks validate for the spanned refusal, as
   review-landing.md item 2 already proposed. **Recommend: the edge maps it; for the lint sheet to confirm.**
4. **The report's operator words.** Example: the oracle's accepted SIM201 row reports `OperatorName("join")`, but
   operators' drafted `OperatorName` has "`Join` gone (the plan crate lowers a join into `Select` sources)". Why:
   `ValidateReport.operators` lists what the query writes (ryoql's `operators_used` over the written program), and
   without a written `join` the report changes. What we get: byte identity on the oracle. **Recommend: operators keeps a
   `Join` name in `OperatorName` (a written operator `OperatorCall::name` never returns); for the operators and ryoql
   sheets to settle.**
5. **The SDK's second family of validate types.** Example: the SDK defines its own `StepCount` and `ValidateReport`
   (`sdk/src/resources/query/validate/types.rs:10,15`), `ValidateRequest`, a six-variant `ValidateError`, and 318 lines
   of `schemas.rs` mapping core's to its own. Why: one definition per concept (criterion 6) against the SDK's public
   promise (criterion 13; the SDK's types use `RyoQlQuery`, `HostDeclaration`, its own names). What we get by keeping
   both for this build: no change to the SDK's public surface. **Recommend: keep both now; this crate's types are what
   edges compose with; record the duplicate for the SDK sheet.**

## 11. Not checked
- The relations listing does not define `HostedRelation`, `RelationName` or `KindVocabularies`, which this listing,
  ryoql's, plan's and language's name as `ryo_relations::..`; programs names `ryo_plan::KindVocabulary`. `check-types.py`
  will fail on them until the relations sheet takes them. I did not redefine them. (Run on 28 Nov, it reports none of
  them; it does report the variant names `Refused` and `UnknownKind` as "defined" in several sheets, this one included:
  they are enum variants, not types, and the script reads them as types.)
- Whether ryoql's `CheckedQuery.reads`/`.operators` equal the tip's `relations_read`/`operators_used` over the linked
  program in order and duplicates (the report says "each once"); taken from the ryoql sheet, not re-read.
- That `QueryView` can be made from plan's `KindWord` for `locate` (both text newtypes; not checked in code).
- The SDK's `schemas.rs` mapping line by line (318 lines), and whether decision 2 changes any of the 31 oracle rows (none
  of them is a plan refusal by its text; not run, no Cargo).
- The cost of a validate call and of rebuilding its composition per SDK call: not measured.
- Tastemaker specs or templates that might name `ryo_core::inbound::use_cases::validate` paths outside
  `crates/ryo/tastemaker.toml`: not searched.
- Build 133: not read for this crate (main and the tip carry the same contract).
