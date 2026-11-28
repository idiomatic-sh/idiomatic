# domain/ryoql
STATUS: drafted (28 Nov 2026; bead ryo-fmx3.1.13.23.4.7; not reviewed, not agreed)

## 1. Owns
RyoQL text in, checked query out: its syntax, its parsing, and its typed checks (the refusals a person reads, with
their place in the text). One thing. It does not own lowering to the plan, the step count of the plan that runs, or
the check of a kind word against the linked languages; at the tip it holds all three, and that is where it is drawn
wrong (section 4, decisions 1 and 2).

Revisions: MAIN = f4ae3bd802, TIP = 2a207464c, B133 = f0219b3b3. Q = `crates/ryo/crates/query_language/src`.
Lines "without tests" exclude `*_test.rs`, `*_test/`, `tests/`, `fixtures`, `cases.rs`; counted with `git show | wc -l`.

## 2. On main (f4ae3bd802)
**Where.** The crate `query_language` (package `idiomatic-sh-ryo-query_language`, library `ryo_query_language`):
282 `.rs` files, 31,409 lines with tests, 15,553 without. 112 `#[test]`, 5 scenarios in 3 features
(compile 2, describe 1, typecheck 2), 438 `CaseName(` rows (198 of them in `compile`'s seam suite). Its Cargo
description says what it was: `description = "RyoQL parsing and planning for source queries"  @f4ae3bd802:crates/ryo/crates/query_language/Cargo.toml:9`.
On main this crate **was the planner**: there is no `finish_plans` module and no `plan_lowering` anywhere on main
(`git ls-tree` finds neither). The lowering was this crate's `domain/assembling` plus 13 optimizer passes
(capture_pushing, cutting, distinctness, fusing, inlining, lineage, narrowing, pruning, pushing, regrouping,
selection_pushing, walking, widths: 8,503 lines with tests, 4,814 without), behind `CompileUseCase`, producing
`pub struct Plan {  @f4ae3bd802:crates/ryo/crates/values/src/domain/plans/types.rs:23`.

**Public items: 21, plus 8 trait methods** (`lib.rs` keeps the domain private: `mod domain;  @f4ae3bd802:crates/ryo/crates/query_language/src/lib.rs:7`).
The five traits, verbatim:
```
pub trait CompileUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/interface.rs:8
fn compile(&self, request: QueryRequest) -> Result<CompileReport, CompileError>;  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/interface.rs:14
pub trait TypecheckUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/interface.rs:8
fn typecheck(&self, request: QueryRequest) -> Result<ValidateReport, TypecheckError>;  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/interface.rs:15
pub trait DescribeUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/describe/interface.rs:7
fn describe(&self) -> Result<DescribeReport, DescribeError>;  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/describe/interface.rs:13
pub trait SyntaxPort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/syntax/interface.rs:9
fn read(&self, text: QueryView<'_>) -> Result<WrittenProgram, SyntaxError>;  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/syntax/interface.rs:15
fn read_bundle(&self, bundle: &QueryBundle) -> Result<ParsedQueryBundle, SyntaxError>;  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/syntax/interface.rs:22
fn read_root(&self, bundle: &QueryBundle) -> Result<WrittenProgram, SyntaxError>;  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/syntax/interface.rs:30
fn grammar(&self) -> Grammar;  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/syntax/interface.rs:33
pub trait CatalogPort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/catalog/interface.rs:7
fn relations(&self) -> Result<Relations, CatalogError>;  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/catalog/interface.rs:13
```
The other 13: `CompileReport` (`pub plan: Plan,  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/types.rs:12`),
`CompileError`, `TypecheckError` (`pub enum TypecheckError {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/errors.rs:7`,
seven variants), `DescribeReport`, `DescribeError`, `Relations`, `CatalogError`, `SyntaxError`, three `*UseCaseImpl`
with their `new`, and the two factories
`pub fn winnow() -> impl crate::outbound::syntax::interface::SyntaxPort {  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/syntax/factory.rs:8` and
`pub fn language() -> impl crate::outbound::catalog::interface::CatalogPort {  @f4ae3bd802:crates/ryo/crates/query_language/src/outbound/catalog/factory.rs:8`.
Plus 24 items behind the `fixtures` feature (test support, "not a public API" by its Cargo comment).

**Implementors.** One each: `CompileUseCaseImpl`, `TypecheckUseCaseImpl`, `DescribeUseCaseImpl`, the winnow adapter,
the catalog adapter (fakes behind `fixtures`). Both checks read the same front: `read_bundle` (or `read_root` when
the session's modules are given: `let (root, modules) = if request.modules.0.is_empty() {  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/compile/use_case.rs:43`),
links, validates, then lowers; `typecheck` counted the steps of the very plan that ran:
`steps: StepCount(plan.steps.len()),  @f4ae3bd802:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:115`.

**Callers** (files naming `ryo_query_language::` outside the crate). Production: sdk 6 (`lint/composition.rs`,
`query/{run,validate,describe}/composition.rs`, `describe/{entry,schemas}.rs`), execution 3 (its
`outbound/query_language` adapter, types, factory), cli 2 (`query/{run,validate}/composition.rs`). Tests: execution 9,
core 4, cli 2. The planner was reached through execution's port:
`pub trait QueryLanguagePort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/execution/src/outbound/query_language/interface.rs:11`
(`validate`, `compile`, `compile_parsed`), and the kind check lived in `execution`'s metadata use case, over the old
plan, called from core's validate after a second compile:
`.compile(&request.text, &request.hosted, &request.observations)  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:61`.

**Proof.** 112 `#[test]`: compile 44 (its seam suite: 22 sub-suites, 198 case rows), the 13 optimizer passes 23,
typecheck 3, describe 1, syntax 3, catalog 3, the rest one case-table harness per domain module. 5 scenarios.
`benches/performance.rs` is an empty divan main (8 lines).

**Tastemaker** (`crates/ryo/tastemaker.toml` at main): line 23 (internal-package allowlist: language, relations,
values), 78 (`grammar-lists-every-parser` over the winnow adapter), 79 (`relation-columns-by-name`), 129
(`crate-query-language`), 190 (`nu-hex-query-language`, nothing exposed).

## 3. On the perf branch (2a207464c)
**Where.** Same crate, 197 `.rs` files, 20,838 lines with tests, 14,188 without (the plan's 14,184 differs by my
filter). 51 `#[test]`, 3 scenarios (describe 1, typecheck 2), 180 case rows. The Cargo description now says
"RyoQL parsing, checking and lowering to the plans the executor runs". Three different things live here:

| Part | Modules (without tests) | Lines |
|---|---|---|
| the front: read, link, validate | `outbound/syntax` 1,634, `outbound/catalog` 79, `domain/{expanding, binding, checking, linking, remapping, rooting, validation, observation_inputs, syntax}` 2,991 | 4,704 |
| **the older lowering** (main's lowering without its optimizer) | `domain/{assembling, calling, combining, comparing, idioms, iterating, joining, lowering, planning, refusing, rendering, reshaping, resolving, rewriting, selecting, spanning, substituting, traversing}` (18) | 5,625 |
| the plan's lowering (from build 133's `engine`) and the kind check | `domain/plan_lowering` 2,951, `domain/kind_check` 197 | 3,148 |
| the use cases | `typecheck` 538 (with `keeper.rs` 63), `describe` 115 | 653 |

**Public items: 41, plus 10 trait methods.** The domain was opened: `pub mod domain;  @2a207464c:crates/ryo/crates/query_language/src/lib.rs:7`,
`pub mod kind_check;  @2a207464c:crates/ryo/crates/query_language/src/domain.rs:15`,
`pub mod plan_lowering;  @2a207464c:crates/ryo/crates/query_language/src/domain.rs:16` (17 items in
plan_lowering, 6 in kind_check). The four traits:
```
pub trait TypecheckUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/interface.rs:9
fn typecheck(&self, request: QueryRequest) -> Result<ValidateReport, TypecheckError>;  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/interface.rs:16
fn typecheck_kinds(&self, request: QueryRequest, kinds: &KindVocabularies) -> Result<ValidateReport, TypecheckError>;  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/interface.rs:23
fn check(&self, request: QueryRequest) -> Result<Checked, TypecheckError>;  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/interface.rs:30
pub trait DescribeUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/describe/interface.rs:7
pub trait SyntaxPort: Send + Sync {  @2a207464c:crates/ryo/crates/query_language/src/outbound/syntax/interface.rs:9
fn read_fragment(  @2a207464c:crates/ryo/crates/query_language/src/outbound/syntax/interface.rs:29
pub trait CatalogPort: Send + Sync {  @2a207464c:crates/ryo/crates/query_language/src/outbound/catalog/interface.rs:7
```
(`read`, `read_bundle`, `read_root`, `grammar`, `relations`, `describe` unchanged from main, at `syntax/interface.rs:15,22,41,44`.)
Also public: `pub enum Checked {  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/types.rs:7`
and `pub fn refused(mut error: TypecheckError) -> ryo_values::domain::program::types::RefusedQuery {  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/service.rs:89`.

**What each method really does** (this decides the cut):
- `typecheck` is `typecheck_kinds` with no languages: `self.typecheck_kinds(request, &KindVocabularies(Vec::new()))  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:136`.
- `typecheck_kinds` runs the checks, then **lowers through the plan that runs** to count its steps and refuse what
  it refuses, then runs the kind check over that plan:
  `let (plan, steps) = keeper::lowered_plan(root, modules)  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:143`,
  `&& let Some(word) = unknown_word(&plan, kinds)  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:148`,
  with `use crate::domain::plan_lowering::service::{lowered, mark_through};  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/keeper.rs:4`.
  So both validate-report methods need the plan, not only the second (Appendix A 3.5 says only the second).
- `check` runs only the checks and stops: `self.checked(&request).map(|(checked, ..)| checked)  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:176`.
  `checked` builds four things and `check` keeps one:
  `) -> Result<(Checked, WrittenProgram, ParsedModules, WrittenProgram), TypecheckError> {  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:67`.
  Its outcome comes from the older lowering:
  `let checked = match lower_with_observations(  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:120`, and
  `Err(error) if computed(&error) => Checked::Deferred(unresolved(error)),  @2a207464c:crates/ryo/crates/query_language/src/inbound/use_cases/typecheck/use_case.rs:127`
  (a computed column the older lowering cannot follow; the plan judges it).

**Callers** (production only; no test file outside the crate names it at the tip). executor 7:
`lower.rs` (parses with the winnow factory, lowers with `lowered`/`mark_through`/`passed`),
`check_session/query.rs` (parses fragments), `check_session/kinds.rs`
(`use ryo_query_language::domain::kind_check::service::{asked, names_kind};  @2a207464c:crates/ryo/crates/executor/src/check_session/kinds.rs:5`),
`check_session.rs`, `check_session/shared.rs`, `replay/batch.rs`, `request_input.rs` (plan_lowering types).
sdk 5: `query/describe/{composition,entry,schemas}.rs`, `query/validate/composition.rs`, and `query/run/schemas.rs`,
the one caller of `check`, for the headings of `query run`:
`.check(ryo_values::domain::query_requests::types::QueryRequest {  @2a207464c:crates/ryo/crates/sdk/src/resources/query/run/schemas.rs:26`.
core 1: validate holds the use case and asks the kinds:
`.typecheck_kinds(asked, &self.language.kind_vocabularies())  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/validate/use_case.rs:51`.
cli 1: `query/validate/composition.rs`. The lint session also reaches validate, through the SDK, to word a rule the
executor refused: `match validate(linked, ValidateRequest { query, hosted, observations: Vec::new() }) {  @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/entry.rs:200`.
So the spanned refusal a `lint` user reads comes from the older lowering, via validate.

**The lint path parses and lowers and never runs the checks.** `pub fn parsed(texts: &PackTexts, context: RuleContext) -> PackPrograms {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:26`
calls the factory itself (`let syntax = winnow();  @2a207464c:crates/ryo/crates/executor/src/lower.rs:27`, again
at `check_session/query.rs:87`) and drops a fragment it cannot read with a print:
`Err(e) => println!("unread fragment {stem}: {e}"),  @2a207464c:crates/ryo/crates/executor/src/lower.rs:35`.

**Three plan types** at the tip: the older lowering's `pub struct PlanStep {  @2a207464c:crates/ryo/crates/values/src/domain/plans/types.rs:21`
(with `pub enum Operation {  @2a207464c:crates/ryo/crates/values/src/domain/plans/types.rs:30`; main's `Plan` struct is
gone and nothing outside `query_language` reads `values::plans` at the tip), the plan lowering's
`pub struct FinishPlan {  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/types.rs:33`, and the
executor's own `pub struct Plan {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:490`.

**Proof.** 51 `#[test]`: typecheck 3 (14 case rows, all through `.typecheck`, none through `.check`), describe 1,
syntax 3 (28 rows), catalog 3, plan_lowering 6 (7 rows), the front and the older lowering one harness each
(refusing 3). 3 scenarios. `benches/performance.rs` still empty.

**Tastemaker** (tip `crates/ryo/tastemaker.toml`): line 20 (allowlist now language, matching, relations, values),
70 (`fact-reads-through-lookup` over `plan_lowering/**`), 73 (`grammar-lists-every-parser`), 74
(`relation-columns-by-name`), 119 (`crate-query-language`), 180 (`nu-hex-query-language`).

## 4. What changed between them
Main has had no commit on this crate since the branch point `f4a32acb56`; everything below is the perf branch.
- **Added `read_fragment`** to `SyntaxPort` (bdd9c5fc54, 23 Nov, "perf(lowering): reuse step dependencies and read
  pack fragments directly"): each fragment parsed once for the pack.
- **Moved in the plan lowering** from build 133's `engine` (04f8f4ec1b, 27 Nov, +3,350 lines in this crate):
  `pub fn lowered(pack: &PackPrograms, stage: PlanningStage) -> FinishPlan {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/service.rs:25`.
- **Deleted the planner** (fd6122c42a, 27 Nov, "one front end: the checks, then the executor's plan lowering; the
  optimizer, the compile use case, compile_rules, rule_compilation and the query-language port are deleted"; 149
  files, -16,108 in this crate). That is the ~12,000 lines gone since build 133 (33,580 -> 20,838, net -12,742):
  the 13 optimizer passes (9,229 with tests at build 133, 23 tests), the compile use case and its seam suite
  (inbound/use_cases 8,110 -> 1,071; 44 tests, 198 rows, 2 scenarios), trims to `assembling` (-121) and `planning`
  (-82); against +3,419 `plan_lowering` and +197 `kind_check`. Start-up measured 341.3 -> 317.9 ms across builds
  138-139 (plan section 10.3 row 1).
- **Added `check`, `Checked` and `keeper.rs`** (fd6122c42a), **`typecheck_kinds` and `kind_check`** (f0db2d155a,
  27 Nov: "the kind check once, over the plan that runs; validate and the executor both ask it"), **`located`**
  (6e8b89c94e), and **headings from the checks** (551980284c).
- **Dependencies added**: `ryo_matching` (for `names_kind`) and `hashbrown`. `ryo_language` was already there on main,
  for one import: `use ryo_language::domain::semantics::types::RenderContext;  @2a207464c:crates/ryo/crates/query_language/src/domain/rendering/service.rs:21`.
- **Lost that should not have been.** (1) The compile seam's proof: 44 tests, 198 case rows and 2 scenarios that
  pinned how each RyoQL construct lowers, deleted with `CompileUseCase`, while the lowering that now runs has 6 tests
  and 7 rows. (2) The closed domain: main exposed 21 items; the tip opens `domain` and exposes the plan's lowering as
  free functions the executor calls into (6 executor files name `ryo_query_language::domain`). (3) `read_root` and `QueryRequest.modules` lost their only reader
  (the compile use case): the "parse the modules once per session" path is dead code at the tip.
- **A recorded finding this contradicts**: CRATES-AND-CONTRACTS.md line 352 says `validate` uses neither `plan` nor
  `execute`. At the tip validate lowers through the plan's lowering on every call (keeper.rs).

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
The listing is `api.txt` beside this sheet: 43 public items plus 10 trait methods. The starting claim was plan
Appendix A 3.5 (`ParseUseCase`, `CheckUseCase`, `DescribeUseCase`, `SyntaxPort`, `CatalogPort`, with typecheck and
typecheck_kinds moving to `inbound/validate`). Checked against the code, three corrections: both validate-report
methods need the plan, not one (section 3); they cannot go to `inbound/validate`, because the lint path reaches
validate (`entry.rs:200`) and inbound crates may not import each other, and they cannot stay in ryoql, because ryoql
sits below plan; so they go to `domain/plan` (decision 2). And `parse` cannot take the plan's `RuleContext` or return
the plan's `PackPrograms` (both are lowering inputs), and today it returns no error at all (5b).

### 5a. `CheckUseCase` (changed)
```
pub trait CheckUseCase: Send + Sync {  @new
fn check(&self, request: QueryRequest) -> Result<CheckedQuery, CheckError>;  @new
fn locate(&self, query: &CheckedQuery, word: QueryView<'_>) -> Option<QueryOrigin>;  @new
```
The trait is the tip's `TypecheckUseCase` renamed for the one method it keeps.
- `check` is the tip's `check` (line 30). It returns `CheckedQuery` instead of `Checked` so that plan's validate can
  lower and judge **without parsing again**: the tip already computes `root`, `modules` and the linked program (use
  case line 67) and throws three away; `reads` and `operators` are what `typecheck_kinds` computes today from the linked
  program with ryoql's own private `relations_read`/`operators_used`. `outcome` is the tip's `Checked`, unchanged.
- `locate` is `pub fn located(program: &WrittenProgram, word: &str) -> Option<(QuerySpan, Placed)> {  @2a207464c:crates/ryo/crates/query_language/src/domain/kind_check/location.rs:34`:
  where a query writes a word, which only the written program knows. The rest of the kind check reads the plan and
  leaves (decision 2). The word is a `QueryView`, the crate's existing word for a piece of query text.
- `CheckError` is main's `TypecheckError` verbatim, seven variants. The tip's eighth, `UnknownKind`, is produced only by
  `typecheck_kinds` and leaves with it.
- `From<CheckError> for RefusedQuery` replaces the free function `refused` (tip `service.rs:89`, called by core's
  validate and the SDK's `query run`): a conversion is `From` (review criterion 12).

### 5b. `ParseUseCase` (new)
```
pub trait ParseUseCase: Send + Sync {  @new
fn parse(&self, pack: &PackTexts) -> ParsedPack;  @new
```
The lint path's own front end, today in the executor (`lower.rs:26`, and the fragment loop of `query.rs:87-92`).
It takes the whole pack, so it is crossed once per request (gain 8). It does not take `RuleContext` (parameters,
context tables, observations: lowering inputs, the plan's) and does not know rule ids (`RuleId` is the request's):
rules are answered in the order given, and plan pairs them. A rule that does not parse is
`RuleProgram::Unread`, as today (`plan_lowering/types.rs:144`, moved here because it is a parse result); a fragment
that does not parse is returned in `unread` instead of printed (`lower.rs:35`). `RuleProgram::Parsed` holds an
`Arc` instead of a `Box` (`Parsed(Box<WrittenProgram>),  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/types.rs:146`)
so one parse of a text can serve both the pack's lowering and a rule's re-lowering (section 7, the RUF100 cost).
`PackTexts` is the executor's `pub struct PackTexts {  @2a207464c:crates/ryo/crates/executor/src/lower.rs:21`
(tuples of `String`) with named types; `RuleText`, `FragmentText`, `UnreadFragment` are new.

### 5c. `DescribeUseCase`, `SyntaxPort`, `CatalogPort` (main)
All carried as on main, plus the tip's `read_fragment`. `read_root` has no production caller at the tip (only main's
compile called it); it is kept, not dropped, because it is half of the designed "modules parsed once per session"
path with `QueryRequest.modules` (decision 4). `CatalogPort` has one adapter that returns
`ryo_relations::...::all()`; kept, a designed seam with a fake behind it. The factory name `language()` is main's;
it reads ryo's own catalog, not a language, and could be renamed (not proposed: no caller reads the name).

### 5d. How it is built
Edges call `syntax::winnow()`, `catalog::language()`, then `CheckUseCaseImpl::new(syntax, catalog)`,
`ParseUseCaseImpl::new(syntax)`, `DescribeUseCaseImpl::new(syntax)`, as they do today for the first and last. The
executor stops calling the factory (`lower.rs:27`, `query.rs:87`): it is handed a `ParseUseCase` ("only edges call
factories"). Constructors keep main's `Arc<dyn ..>`: every call is per request (section 9), none per file or row.

### 5e. The types that cross it
ryoql becomes the home of the RyoQL vocabulary now in `values`: `program` (28 public items at the tip),
`query_syntax` (22), `query_tokens` (27), `query_bindings` (6), `query_requests` (1), the grammar's four types from
`descriptions` (`Grammar`, `Production`, `ProductionName`, `ProductionForm`) and `QueryText` from `plan_parts`
(`pub struct QueryText(pub Box<str>);  @2a207464c:crates/ryo/crates/values/src/domain/plan_parts/types.rs:7`), which
`query_tokens` imports (`use crate::domain::plan_parts::types::QueryText;  @2a207464c:crates/ryo/crates/values/src/domain/query_tokens/types.rs:3`).
api.txt names the 13 its own signatures use. Imported, not owned: `Relation`, `RelationName`, `HostedRelation` from
relations; `OperatorName`, the rest of `plan_parts` and `query_operators` from operators (both below ryoql; the
architecture review's B1 sends `plan_parts` and `query_operators` to operators). `ValidateReport`, `StepCount`,
`KindVocabularies` leave ryoql's signatures entirely (plan's).

### 5f. What stops being public
Named in api.txt `# NO LONGER PUBLIC`: 22 of the tip's 41 items and 2 of its 10 methods (all of `plan_lowering`
but `RuleProgram`/`UnreadQuery`, all of `kind_check`, `refused`, `typecheck`, `typecheck_kinds`), and `domain`
closes again as on main. Main's four compile items are not restored (decision 1).

### 5g. The counts
| Side | Public items (+ trait methods) | Traits |
|---|---|---|
| main `f4ae3bd802` | 21 (+8) | 5: Compile, Typecheck, Describe, Syntax, Catalog |
| tip `2a207464c` | 41 (+10), of which 23 are the plan's lowering and the kind check | 4: Typecheck, Describe, Syntax, Catalog |
| drafted | 43 (+10), of which 13 are moved vocabulary | 5: Check, Parse, Describe, Syntax, Catalog |
Counted as `pub fn|struct|enum|trait|type` reachable from the crate root, without `pub mod` lines and without the
`fixtures` feature (main 24, tip 8). The drafted count rises only by vocabulary that moves in from `values`.

## 6. Proof it brings
- **Carried as they are:** syntax 3 tests / 28 rows and the winnow adapter's tests; catalog 3 / 3; describe 1 and its
  scenario; the front's harnesses (binding, checking 2, expanding 2, linking 2, remapping 2, rooting, validation 2,
  observation_inputs 2, syntax); and, if decision 1 keeps it, the older lowering's 18 harnesses (refusing 3, 33 rows).
- **Restated:** typecheck's 3 tests and 2 scenarios. Its 14 rows all call `.typecheck` and compare a `ValidateReport`;
  the 10 refusal rows (syntax span, unknown relation, type mismatch, undefined step, six observation rules) and the
  failing-catalog row restate against `check` with the same errors; the 3 success rows compare `steps`, which become
  plan's, so they restate on the plan sheet.
- **Moves with the plan:** plan_lowering's 6 tests / 7 rows; the kind check has no test of its own at the tip.
- **Missing:** a `ParseUseCase` contract (the executor's `parsed` has no test; planted bugs to catch: a dropped
  fragment, rule order not kept, an unread rule not reported); a `locate` row table (one place, two places, a
  module's place); a seam test that `check` hands on the program `parse` would have read (no double parse).
- **Lost, for the owner:** the compile seam suite (44 tests, 198 rows) at fd6122c42a. Its rows assert old-plan steps,
  so nothing of it carries to ryoql; the refusal rows among them could be restated against `check`; how many is not
  counted. The rest is the plan sheet's question.

## 7. Perf gains it carries
perf-gains.tsv does not exist yet; from plan section 10.3:
- **Row 40** (11-22), Rust lowering replaces the Python chain, 8.0 -> 0.093 s: the parse half is this crate's winnow
  adapter. Depends on: parsing in process into typed programs, no text plan in between (`SyntaxPort::read`).
- **Row 8** (11-25), the pack lowered once per request, fastapi lowering 888 -> 586 M instructions: `parsed` runs once
  over every rule. Depends on: `ParseUseCase::parse` taking the whole pack, crossed once per request, never per rule.
- **Row 1** (11-27), one lowering, start-up 341.3 -> 317.9 ms: holds only while the lint path calls `parse` and never
  `check` (the old fused line measured compile at 62% of a one-file run, row 48). The draft keeps `check` off the lint
  path.
- **Not in 10.3:** bdd9c5fc54, fragments read once each per pack (`read_fragment`, `ParsedModules` shared by `Arc`);
  no figure found. Depends on `read_fragment` and on `ParsedModules` staying an `Arc` map.
- **Room to make (lost or never taken):** Python start-up spends 64 ms re-lowering one rule (RUF100) because the
  executor re-parses all 410 producer texts it observes (`texts.rules.push((alias.clone(), Some(source.text.to_string())));  @2a207464c:crates/ryo/crates/executor/src/lower.rs:101`,
  then `let mut programs = parsed(&texts, context);  @2a207464c:crates/ryo/crates/executor/src/lower.rs:110`;
  build/ports/stage2/trial-1/summary.txt line 3). The parse share of the 64 ms is not measured. Depends on:
  `RuleProgram::Parsed(Arc<..>)` and `read_root` + `QueryRequest.modules` (decision 4), so a text is parsed once
  per request.
- Rows 16, 20, 25 (pushdown, the shared plan, binding from the typed plan) are code in `plan_lowering` and the
  executor: the plan and programs sheets', not this one's.

## 8. Benchmarks that cover it
- `query_language/benches/performance.rs`: an empty divan main on both sides; it measures nothing. Proposed: a parse
  bench over the Ruff pack (413 rules) would give the parse share of start-up its own number; not written.
- The lint images (`vec`, `vec_ts`, `vec_rust`, `tm`) and `build/tm-int/scripts/window-nine.sh` measure start-up
  (Python 296 ms) through the SDK only; no ryoql name is reached, so the draft cannot break them.
- `build/one-engine/scripts/price-root-split.py` line 47 (evidence repo): its "lower" bucket matches
  `ryo_query_language|plan_lowering|assembling|cutting|...`, parse and lowering together. After the regroup it must
  match `ryo_ryoql` and `ryo_plan` as two buckets, or the start-up split goes blind.
- Asymptote ratchets: 7 lock files under `crates/ryo/.asymptote/operations/` hold path-keyed entries for
  `crates/query_language/**` (file-length, no-bare-bool), some for files deleted at fd6122c42a
  (`domain/inlining/service.rs`, `domain/pushing/service.rs`). They must be re-keyed to the new path.
- Tastemaker entries that name paths and must follow the move: `grammar-lists-every-parser` (winnow adapter paths),
  `crate-query-language`, `nu-hex-query-language`, the internal-package allowlist; `fact-reads-through-lookup`
  follows `plan_lowering` to the plan.

## 9. Checks
IMPORTS: relations, operators
CROSSED: per request
- Directions: relations and operators are below ryoql in the agreed order. Today's wrong-way edges and their fix:
  ryoql -> plan through `kind_check/service.rs` (FinishPlan), `typecheck/keeper.rs` and `typecheck/use_case.rs`
  (gone: they move to plan); ryoql -> the `matching` crate (broken apart by the owner) through `names_kind` only (moves to
  operators); ryoql -> `ryo_language` (outbound) through `RenderContext` in `domain/rendering/service.rs:21` and
  `values::plans` (`context: ryo_language::domain::semantics::types::RenderContext,  @2a207464c:crates/ryo/crates/values/src/domain/plans/types.rs:374`):
  `RenderContext` is the render-syntax operator's argument and must move to operators. ryoql -> `runs` (`RuleId`)
  only in keeper and plan_lowering (both leave). plan -> ryoql edges (plan_lowering reads `program`, `query_syntax`,
  `query_tokens`; `finish_plans` reads `AliasName`:
  `use crate::domain::query_tokens::types::AliasName;  @2a207464c:crates/ryo/crates/values/src/domain/finish_plans/expressions/types.rs:5`)
  point the allowed way. `query_tokens` and `query_bindings` import `plan_parts`, which goes to operators: allowed.
- Smallest cut that leaves ryoql importing nothing above it: move out `domain/plan_lowering` (2,951), 
  `domain/kind_check/service.rs`, `typecheck/keeper.rs`, the two report methods and `UnknownKind`; keep
  `kind_check/location.rs` (as `locate`); move `RenderContext` down. Nothing else in ryoql names `finish_plans` or
  `plan_lowering` (checked by grep of each module's imports at the tip).
- No layout word in any signature (no rows, columns, cells).
- How often: `check`, `locate`, `describe` once per request (validate, `query run` headings, a refused lint rule);
  `parse` once per request over the pack; inside it, `SyntaxPort` once per rule and per fragment (413 + fragments
  for Python), through `dyn`; at load, not per file. No call is per file, step, row or cell.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **The older lowering: keep it as ryoql's private typed checks, or delete it.** Example: `check` gets its outcome
   from `lower_with_observations` (use case line 120): 18 modules, 5,625 lines, its own step type `PlanStep`/
   `Operation` in `values::plans` that nothing else reads; it is main's planner minus its optimizer. Why it matters:
   it is a second lowering beside the plan's ("one path"), but it is also the only source of spanned type refusals:
   `pub(crate) enum LoweringError {  @2a207464c:crates/ryo/crates/query_language/src/domain/lowering/errors.rs:8`
   has 33 variants with a span or origin; the plan's
   `pub enum PlanLoweringError {  @2a207464c:crates/ryo/crates/query_language/src/domain/plan_lowering/errors.rs:17`
   has 8 and none carries a span; and `query run`'s headings come from it. Deleting it changes refusal text that
   `lint` users read (via validate) and needs spans and headings added to the plan's lowering first: a rewrite, not a
   move. What we get by keeping: the build stays a move with identical refusals; `values::plans`, `plan_orders`,
   `written_rows`, `lineage` become private to ryoql. **Recommend: keep it, private, for this build; one later bead
   gives the plan's lowering spans and headings, proves refusal identity on the validate and typecheck suites, then
   deletes the 5,625 lines.**
2. **Where the validate report and the kind check go.** Example: `typecheck` and `typecheck_kinds` lower through the
   running plan (use case lines 136, 143) and run `unknown_word` over the `FinishPlan` (line 148); `names_kind`
   compares with glob, prefix, suffix and regex through `ryo_matching`. Why: in ryoql they make a cycle with plan;
   in `inbound/validate` the lint path (`entry.rs:200`) would import another inbound crate. What we get: ryoql below
   plan with no exception; the report computed in one place both validate and lint call. **Recommend: both methods
   with `keeper.rs` and `asked`/`unknown_word` go to `domain/plan` as one use case taking a `CheckedQuery` and the
   kind vocabularies; `names_kind` goes to operators (the comparison's own meaning, as the owner said of the kind
   check); `located` stays as `CheckUseCase::locate`.**
3. **`parse` as ryoql's own use case.** Example: the executor parses for itself with the factory
   (`lower.rs:26-53`, `query.rs:87-92`) and prints and drops an unreadable fragment (`lower.rs:35`). Why: the lint
   path needs parsing without the checks; a domain crate calling a factory breaks "only edges call factories"; a
   dropped fragment surfaces later as an unrelated "no fragment" refusal. What we get: one front end for the lint
   path, handed in, testable; fragment refusals reported. **Recommend: `ParseUseCase::parse(&PackTexts) ->
   ParsedPack` as drafted, with `RuleProgram::Parsed(Arc<..>)`.**
4. **`read_root` and `QueryRequest.modules`: dead at the tip.** Example: main's compile read only the root when the
   session's modules were given (main `compile/use_case.rs:43`, `:55`); at the tip no production code calls
   `read_root` and nothing in the crate reads `modules`. Why: a designed contract is not dropped for having no caller;
   and this pair is exactly the room for the RUF100 re-parse (section 7). What we get: modules parsed once per
   session, a measurable start-up cut. **Recommend: keep both; `check` honours `modules` again as main's compile did;
   the programs sheet decides whether the lint path uses it.**
5. **Which crate owns the request's declared inputs.** Example: `QueryRequest` carries `hosted:
   Vec<HostedRelation>` and `observations: Vec<BoundObservationInput>`; `HostedRelation` is a relation the host
   declares (a catalog entry), an observation is written in RyoQL (`observe <step> as <output>`) and read by the plan's
   lowering. Why: `values` is dissolving and "the lowest user" is not a placement rule (architecture review B11).
   What we get: each type with its concept. **Recommend: `HostedRelation` to relations; `BoundObservationInput` and
   `ObservationName` to ryoql (plan reads them from above); `QueryRequest` and the RyoQL syntax vocabulary to ryoql.
   Not fully checked: who else names the observation types (section 11).**

## 11. Not checked
- Every reader of the moved vocabulary outside ryoql (program, query_syntax, query_tokens, query_bindings,
  observations) was not listed file by file; the mapping reader's module counts (executor, sdk, core) were not
  re-verified. The item counts in 5e include test-only items if any sit in those directories.
- Whether `plan_lowering/expansion` and ryoql's `expanding` (import linking, twice: once on the lint path, once in
  the checks) agree on every pack is not checked; it is a second implementation of fragment linking.
- Whether the plan's `FinishPlan` can give `query run` its headings (needed before decision 1's deletion) is not
  checked.
- How many of the 198 deleted compile rows are refusal rows that could be restated against `check` is not counted.
- The parse share of Python start-up (inside the 95-98 ms batch lowering and the 64 ms RUF100 re-lowering) is not
  measured; no figure for bdd9c5fc54 was found.
- `HostedRelation`'s and `BoundObservationInput`'s other users, and whether `ObservationName` belongs to ryoql or
  the request (decision 5), are not checked.
- `query_bindings` (parameter declarations and bindings) is moved whole on the strength of its users inside ryoql;
  its users in the executor and SDK were not read.
- No `tm check` was run on the drafted names (nominal boundary types: `PackTexts` holds `Vec`s of named types;
  `CheckedQuery` holds `Vec<RelationName>`); no compile of the draft.
- Build 133 was read only for line counts and the commit range; its `CompileUseCase` and execution's
  `QueryLanguagePort::written` were not compared.
- The Tastemaker benchmark project and the per-language benches were not searched for ryoql names.
