# inbound/describe
STATUS: drafted (28 Nov 2026; bead ryo-fmx3.1.13.23.4.20). Not reviewed, not agreed.

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). Build 133 (f0219b3b3)
was not needed: the concept is the same on main and the tip, file for file. Every path below is in
`/workspaces/idiomatic-public-benchmark`, read with `git -C ... show "REV:PATH"`. A quote is one line, two spaces,
`@REV:PATH:LINE`. The listing is `api.txt` beside this sheet.

## 1. Owns
Ryo's catalog as a reader sees it: each relation (with what every linked language says of it) or operator, listed
whole or described one at a time.

One thing: the reader's view of the catalog. The catalog itself is domain/relations'; what an operator says of itself
is domain/operators'; the RyoQL grammar is domain/ryoql's `DescribeUseCase` (decision 3). This crate turns those into
the four answers `tm explain`, `tm list` and `ryo relation|operator list|describe` print. It runs no rule and reads no
file, so it is off every path the benchmarks time.

## 2. On main (f4ae3bd802)

**Where.** Four use cases in `core`, one domain helper in `core`, one application service in `core`, and the relation
description types in `values`:

| Place | Files (without tests) | Lines |
|---|---|---|
| `crates/ryo/crates/core/src/inbound/use_cases/{list_relations, describe_relation, list_operators, describe_operator}/` | `interface.rs`, `types.rs`, `errors.rs`, `use_case.rs` each (16) | 373 |
| `crates/ryo/crates/core/src/domain/relation_lookup/` | `service.rs`, `errors.rs` | 37 |
| `crates/ryo/crates/core/src/application/metadata/service.rs` | the `kernel_metadata()` bridge, plus main's `RunDecisions` bridge (not this concept) | 56 |
| `crates/ryo/crates/values/src/domain/descriptions/` | `types.rs` (189), `service.rs` (52), `constants.rs` (36): the relation half is this concept's, the operator half the operators', the grammar half ryoql's | 277 |

**The contract, word for word.** Four use-case traits:
```
pub trait ListRelationsUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_relations/interface.rs:7
fn list_relations(&self) -> Result<ListRelationsReport, ListRelationsError>;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_relations/interface.rs:13
pub trait DescribeRelationUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/interface.rs:7
fn describe_relation(  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/interface.rs:13
pub trait ListOperatorsUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_operators/interface.rs:7
fn list_operators(&self) -> Result<ListOperatorsReport, ListOperatorsError>;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_operators/interface.rs:13
pub trait DescribeOperatorUseCase: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/interface.rs:7
fn describe_operator(  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/interface.rs:13
```
(`describe_relation(&self, request: DescribeRelationRequest) -> Result<DescribeRelationReport, DescribeRelationError>`
and `describe_operator(&self, request: DescribeOperatorRequest) -> Result<DescribeOperatorReport, DescribeOperatorError>`
run over lines 13 to 16 of their files.)

Their types and errors:
```
pub struct ListRelationsReport {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_relations/types.rs:7
pub struct DescribeRelationRequest {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/types.rs:8
pub struct DescribeRelationReport {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/types.rs:15
pub struct ListOperatorsReport {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_operators/types.rs:7
pub struct OperatorSummary {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_operators/types.rs:14
pub struct DescribeOperatorRequest {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/types.rs:8
pub struct DescribeOperatorReport {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/types.rs:15
pub enum ListRelationsError {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_relations/errors.rs:7
Language(#[from] LanguageError),  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_relations/errors.rs:10
pub enum DescribeRelationError {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/errors.rs:5
pub enum ListOperatorsError {}  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_operators/errors.rs:5
pub enum DescribeOperatorError {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/errors.rs:5
```
`DescribeRelationError` has `UnknownRelation { name: String }` and `Language { reason: String }`;
`DescribeOperatorError` has `UnknownOperator { name: String }`. Note the two relation errors carry the same failure two
ways: `ListRelationsError::Language` wraps core's `LanguageError`, `DescribeRelationError::Language` its text.

The relation description types the reports carry, from `values`:
```
pub struct RelationSummary {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:62
pub struct RelationDescription {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:25
pub struct LanguageDescription {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:38
pub struct ColumnDescription {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:51
pub enum RelationReadability {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:16
pub struct AbsenceReason(pub Box<str>);  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:8
pub struct ColumnKind(pub Box<str>);  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:12
pub fn summarize(relation: Relation) -> RelationSummary {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/service.rs:23
pub fn describe(relation: Relation) -> RelationDescription {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/service.rs:32
```
and the text names the requests carry:
```
pub struct RelationName(pub Box<str>);  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:130
pub struct OperatorName(pub Box<str>);  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:71
pub struct OperatorDescription {  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:100
pub struct Purpose(pub Box<str>);  @f4ae3bd802:crates/ryo/crates/values/src/domain/descriptions/types.rs:75
```

**Public items: 27** in core (per use case: trait, method, report, error, `*UseCaseImpl`, `new`; plus the two requests
and `OperatorSummary`), plus 9 in `values::descriptions` that only this concept reads (the 7 relation types above and
`summarize`, `describe`). The modules are public: `pub mod describe_operator;` and the three others in
`core/src/inbound/use_cases.rs`, under `pub mod inbound;` in `core/src/lib.rs`.

**Who implements it.** One implementation each, in `core`:
```
pub fn new(language: Arc<dyn LanguagePort>) -> Self {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_relations/use_case.rs:27
pub fn new(language: Arc<dyn LanguagePort>) -> Self {  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/use_case.rs:36
pub struct ListOperatorsUseCaseImpl;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/list_operators/use_case.rs:11
pub struct DescribeOperatorUseCaseImpl;  @f4ae3bd802:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/use_case.rs:11
```
The relation pair asks core's language port for the catalog and, for describe, each linked language's answer:
```
pub trait LanguagePort: Send + Sync {  @f4ae3bd802:crates/ryo/crates/core/src/outbound/language/interface.rs:17
fn relations(&self) -> Result<RelationList, LanguageError>;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/language/interface.rs:23
fn supports(&self, relation: Relation) -> Answers;  @f4ae3bd802:crates/ryo/crates/core/src/outbound/language/interface.rs:35
Ok(RelationList(all().to_vec()))  @f4ae3bd802:crates/ryo/crates/core/src/outbound/language/adapters/use_case/adapter.rs:110
```
so the real adapter's `relations` never fails: it is the relations crate's `all()`. The name is resolved by
```
pub(crate) fn resolve(  @f4ae3bd802:crates/ryo/crates/core/src/domain/relation_lookup/service.rs:16
```
over `relation_named`. The operator pair asks the kernel through a crate-private bridge:
```
pub(crate) fn kernel_metadata() -> impl MetadataUseCase {  @f4ae3bd802:crates/ryo/crates/core/src/application/metadata/service.rs:20
fn operators(&self) -> &'static [QueryOperator];  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/metadata/interface.rs:132
fn describe(&self, operator: QueryOperator) -> OperatorDescription;  @f4ae3bd802:crates/ryo/crates/execution/src/inbound/use_cases/metadata/interface.rs:134
pub fn describe(operator: QueryOperator) -> OperatorDescription {  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/operator/constants.rs:47
```
(`MetadataUseCase` has 23 methods on main; these two are the only ones this concept calls.)

**Who calls it.** Two edges, each composing the same use cases itself:
- the SDK, `crates/ryo/crates/sdk/src/resources/{relation/{list,describe}, operator/{list,describe}}/` (composition,
  entry, schemas; 3 files each, 12 files), behind four of the five public functions:
```
pub fn ryo::operator::describe(ryo::operator::DescribeRequest) -> core::result::Result<ryo::operator::DescribeReport, ryo::operator::DescribeError>  @f4ae3bd802:crates/ryo/crates/sdk/api.txt:2510
pub fn ryo::operator::list() -> core::result::Result<ryo::operator::ListReport, ryo::operator::ListError>  @f4ae3bd802:crates/ryo/crates/sdk/api.txt:2511
pub fn ryo::query::describe() -> core::result::Result<ryo::query::DescribeReport, ryo::query::DescribeError>  @f4ae3bd802:crates/ryo/crates/sdk/api.txt:3283
pub fn ryo::relation::describe(ryo::linter::Languages, ryo::relation::DescribeRequest) -> core::result::Result<ryo::relation::DescribeReport, ryo::relation::DescribeError>  @f4ae3bd802:crates/ryo/crates/sdk/api.txt:3668
pub fn ryo::relation::list() -> core::result::Result<ryo::relation::ListReport, ryo::relation::ListError>  @f4ae3bd802:crates/ryo/crates/sdk/api.txt:3669
```
  The fifth, `query::describe`, composes ryoql's `DescribeUseCase` directly, not core.
- the command line, `crates/ryo/cli/src/resources/{relation,operator}/{list,describe}/` (command, composition,
  schemas; 12 files) and `crates/ryo/cli/src/outcome/remedy.rs` (one `Remedy` per error). It does NOT go through the
  SDK for these: the same use case has two composition paths today. The command line has no `query describe`.
- **The one product caller** of the SDK functions is Tastemaker's analysis adapter, behind `AnalysisPort::{relations,
  relation, operators, operator, grammar}` for `tm explain` and `tm list`. On main it sits at lines 132 to 191:
```
let listed = ryo::relation::list().map_err(engine)?;  @f4ae3bd802:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:132
let described = ryo::relation::describe(  @f4ae3bd802:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:150
let listed = ryo::operator::list().map_err(engine)?;  @f4ae3bd802:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:165
let described = ryo::operator::describe(ryo::operator::DescribeRequest {  @f4ae3bd802:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:179
let described = ryo::query::describe().map_err(engine)?;  @f4ae3bd802:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:191
```
  The plan (8.2, 11) gives `adapter.rs:161-220` for both revisions; that range is the tip's only. Correction recorded.
- No benchmark, language crate, Asymptote or tool names any of it (`git grep` over `crates/ryo/crates/benchmark`,
  `benchmark_contracts`, `languages`, `tools`, `crates/asymptote` at both revisions: no hit).

**Proof on main.**

| Where | `#[test]` | case rows | scenarios (feature rows) |
|---|---|---|---|
| core `list_relations/use_case_test.rs` | 2 | 2 (`lists_every_relation_ryo_owns` pinned at 77, `fails_when_language_port_fails`) | 2 (succeeds 1, refuses 1) |
| core `describe_relation/use_case_test.rs` | 3 | 5 (`describes_one_relation`, `says_which_linked_language_does_not_read_it`, `a_relation_the_engine_gives_is_read_by_no_language`, `unknown_relation_is_refused`, `fails_when_language_port_fails`) | 2 |
| core `list_operators/use_case_test.rs` | 1 | 1 (`lists_every_operator_in_documentation_order`, pinned at 26) | 1 |
| core `describe_operator/use_case_test.rs` | 2 | 2 (`describes_one_operator`, `unknown_operator_is_refused`) | 2 |
| core `domain/relation_lookup/service_test.rs` | 2 | 3 | 0 |
| core `application/metadata/service_test.rs` | 1 | 1 | 0 |
| values `descriptions/service_test.rs` | 1 | 3 (all three about relation descriptions) | 0 |
| **concept total** | **12** | **17** | **7** (4 features, run by `core/tests/bdd.rs`) |
| SDK `resources/{relation,operator}/*/{entry,schemas}_test.rs` (8 files) | 12 | 13 | 0 |
| CLI `resources/{relation,operator}/*/{command,schemas}_test.rs` (8 files) | 12 | 16 | 0 |

The SDK and command-line tests are those edges' own (their sheets), counted here because they are what proves the
calls end to end.

**Tastemaker entries that name it.** None by name. It is covered by `crate-core` and `nu-hex-core`
(`crates/ryo/tastemaker.toml`, all of `crates/core`), `nu-hex-values`, `crate-layers` and `edges-apart`; and, from the
Tastemaker side, by the rule that pins which SDK names Tastemaker may use, which lists all five functions and their
request, report and error types (see section 3).

## 3. On the perf branch (2a207464c)

**Unchanged in shape.** The same files at the same paths; `git diff --stat f4ae3bd802 2a207464c` over the four use
case folders, `relation_lookup`, the SDK's `relation`, `operator` and `query/describe` resources and ryoql's `describe`
touches only test files (section 4). Every quote of section 2 under `core/src/inbound/use_cases/` holds at the tip at
the same line:
```
pub trait ListRelationsUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_relations/interface.rs:7
fn list_relations(&self) -> Result<ListRelationsReport, ListRelationsError>;  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_relations/interface.rs:13
pub trait DescribeRelationUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/interface.rs:7
pub trait ListOperatorsUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_operators/interface.rs:7
pub trait DescribeOperatorUseCase: Send + Sync {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/interface.rs:7
pub struct ListRelationsReport {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_relations/types.rs:7
pub struct DescribeRelationRequest {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/types.rs:8
pub struct DescribeRelationReport {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/types.rs:15
pub struct ListOperatorsReport {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_operators/types.rs:7
pub struct OperatorSummary {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_operators/types.rs:14
pub struct DescribeOperatorRequest {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/types.rs:8
pub struct DescribeOperatorReport {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/types.rs:15
pub enum ListRelationsError {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_relations/errors.rs:7
pub enum DescribeRelationError {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/errors.rs:5
pub enum ListOperatorsError {}  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_operators/errors.rs:5
pub enum DescribeOperatorError {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/errors.rs:5
pub fn new(language: Arc<dyn LanguagePort>) -> Self {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_relations/use_case.rs:27
pub fn new(language: Arc<dyn LanguagePort>) -> Self {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/use_case.rs:36
pub fn new() -> Self {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_operators/use_case.rs:16
pub fn new() -> Self {  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/use_case.rs:16
```
**Public items: 27**, the same 27. Lines: the same 373 in the four use cases and 37 in `relation_lookup`; the metadata
service is 11 lines (section 4).

**The interface actually called.** The operator pair still goes through the bridge, now over a 2-method trait:
```
pub(crate) fn kernel_metadata() -> impl MetadataUseCase {  @2a207464c:crates/ryo/crates/core/src/application/metadata/service.rs:9
let operators = kernel_metadata()  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_operators/use_case.rs:23
kernel_metadata()  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_operator/use_case.rs:26
pub trait MetadataUseCase {  @2a207464c:crates/ryo/crates/execution/src/inbound/use_cases/metadata/interface.rs:11
fn operators(&self) -> &'static [QueryOperator];  @2a207464c:crates/ryo/crates/execution/src/inbound/use_cases/metadata/interface.rs:13
fn describe(&self, operator: QueryOperator) -> OperatorDescription;  @2a207464c:crates/ryo/crates/execution/src/inbound/use_cases/metadata/interface.rs:15
pub fn describe(operator: QueryOperator) -> OperatorDescription {  @2a207464c:crates/ryo/crates/execution_domain/src/domain/operator/constants.rs:17
```
The relation pair calls the port's two methods, and **they are that port pair's only callers on the tip** (`git grep
'\.relations()'` and `'\.supports('` over `crates/ryo/*.rs` without tests: the two use cases, plus ryoql's own
`CatalogPort::relations`, a different trait):
```
.relations()?  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_relations/use_case.rs:36
.supports(relation)  @2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/use_case.rs:66
fn relations(&self) -> Result<RelationList, LanguageError>;  @2a207464c:crates/ryo/crates/core/src/outbound/language/interface.rs:21
fn supports(&self, relation: Relation) -> Answers;  @2a207464c:crates/ryo/crates/core/src/outbound/language/interface.rs:39
Ok(RelationList(all().to_vec()))  @2a207464c:crates/ryo/crates/core/src/outbound/language/adapters/use_case/adapter.rs:75
```

**Callers.** The same two edges. The SDK builds a language port with no language to list relations:
```
Arc::new(language::factory::use_case(Vec::new())),  @2a207464c:crates/ryo/crates/sdk/src/resources/relation/list/composition.rs:30
pub fn list() -> Result<ListReport, ListError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/relation/list/entry.rs:14
pub fn describe(  @2a207464c:crates/ryo/crates/sdk/src/resources/relation/describe/entry.rs:15
pub fn list() -> Result<ListReport, ListError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/operator/list/entry.rs:14
pub fn describe(request: DescribeRequest) -> Result<DescribeReport, DescribeError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/operator/describe/entry.rs:14
pub fn describe() -> Result<DescribeReport, DescribeError> {  @2a207464c:crates/ryo/crates/sdk/src/resources/query/describe/entry.rs:15
let syntax = Arc::new(ryo_query_language::outbound::syntax::factory::winnow());  @2a207464c:crates/ryo/crates/sdk/src/resources/query/describe/composition.rs:27
```
The command line does the same, and names the operator use case as a unit-struct literal:
```
service: ListRelationsUseCaseImpl::new(Arc::new(language::factory::use_case(Vec::new()))),  @2a207464c:crates/ryo/cli/src/resources/relation/list/composition.rs:28
service: DescribeRelationUseCaseImpl::new(Arc::new(crate::languages::linked())),  @2a207464c:crates/ryo/cli/src/resources/relation/describe/composition.rs:27
service: ListOperatorsUseCaseImpl,  @2a207464c:crates/ryo/cli/src/resources/operator/list/composition.rs:24
impl Remedy for ryo_core::inbound::use_cases::list_relations::errors::ListRelationsError {  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:101
impl Remedy for ryo_core::inbound::use_cases::describe_relation::errors::DescribeRelationError {  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:109
impl Remedy for ryo_core::inbound::use_cases::list_operators::errors::ListOperatorsError {  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:118
impl Remedy for ryo_core::inbound::use_cases::describe_operator::errors::DescribeOperatorError {  @2a207464c:crates/ryo/cli/src/outcome/remedy.rs:124
```
The SDK's five functions, unchanged from main:
```
pub fn ryo::operator::describe(ryo::operator::DescribeRequest) -> core::result::Result<ryo::operator::DescribeReport, ryo::operator::DescribeError>  @2a207464c:crates/ryo/crates/sdk/api.txt:2477
pub fn ryo::operator::list() -> core::result::Result<ryo::operator::ListReport, ryo::operator::ListError>  @2a207464c:crates/ryo/crates/sdk/api.txt:2478
pub fn ryo::query::describe() -> core::result::Result<ryo::query::DescribeReport, ryo::query::DescribeError>  @2a207464c:crates/ryo/crates/sdk/api.txt:3252
pub fn ryo::relation::describe(ryo::linter::Languages, ryo::relation::DescribeRequest) -> core::result::Result<ryo::relation::DescribeReport, ryo::relation::DescribeError>  @2a207464c:crates/ryo/crates/sdk/api.txt:3637
pub fn ryo::relation::list() -> core::result::Result<ryo::relation::ListReport, ryo::relation::ListError>  @2a207464c:crates/ryo/crates/sdk/api.txt:3638
```
and Tastemaker's adapter, the one product caller:
```
let listed = ryo::relation::list().map_err(engine)?;  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:161
let described = ryo::relation::describe(  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:179
let listed = ryo::operator::list().map_err(engine)?;  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:194
let described = ryo::operator::describe(ryo::operator::DescribeRequest {  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:208
let described = ryo::query::describe().map_err(engine)?;  @2a207464c:crates/tastemaker/crates/core/src/outbound/analysis/adapters/ryo/adapter.rs:220
```
with Tastemaker's own pin on the SDK names it may use (all five functions, `DescribeRequest`, `DescribeError`,
`RelationDescription`, `Reading`, `OperatorDescription`, `Requirement`, `DescribeReport`, `RelationName`,
`OperatorName`):
```
{ rule = "idiomatic.arch/library-surface", include = ["**"],  @2a207464c:crates/tastemaker/tastemaker.toml:10
```

**Proof at the tip.** The same 12 `#[test]`, 17 case rows and 7 scenarios as main; the pins read 80 relations and 34
operators. The four features still run from `core/tests/bdd.rs` (`mod describe_operator;` at `:4`, `mod list_relations;`
at `:13`). SDK 12 / 13 and CLI 12 / 16, as main.

**Tastemaker entries.** As main: `crate-core`, `nu-hex-core`, `nu-hex-values`, `crate-layers`, `edges-apart`, and
Tastemaker's `library-surface` pin above.

## 4. What changed between them

**Added, removed, changed in the concept: nothing in a signature.** `git diff --stat` over every path of section 2:

| File | Change | Commit (date) | Why |
|---|---|---|---|
| `core/.../list_relations/{use_case_test.rs, acceptance.feature}`, SDK `relation/list/entry_test.rs` | the pinned relation count 77 -> 79 -> 80 | `9253e01271` "the pinned catalog size is 79: binding-resolution and lookup-scope were added without it", `baee62831b`, `d9b560788f` "the relation count is pinned at 80", merged by `3e68ba623e` (all 11-26) | relations were added to the catalog |
| `core/.../list_operators/{use_case_test.rs, acceptance.feature}` | the pinned operator count 26 -> 34 and the documented order (`as-of`, `group-min`, `group-max`, `group-ordinal`, `follow`, `order`, `prefixes`, `subtree-class`) | `aa01a7d03b` / `a660a24c08` (11-20), `e21cc5c651` (11-27) | operators were added to RyoQL |
| `core/src/application/metadata/service.rs` | 56 -> 11 lines: main's `impl RunDecisions for Metadata` bridge (8 methods, the old engine's run decisions) deleted; `kernel_metadata()` kept | `6768fafa46` (11-20) added to it, `ff4533b99a` "the old engine arms are deleted" (11-26) removed it | the old engine went; the bridge is not this concept's (the execute and programs sheets carry run decisions) |
| `execution/.../metadata/interface.rs` | `MetadataUseCase` 23 methods -> 2 (`operators`, `describe`) | stage 1 deletions | the 21 others were the old engine's; the two this concept calls survive |
| `values/.../descriptions/types.rs` | +30 lines: `FieldWord`, `FixedName`, two fields on `KindVocabulary` | (executor's kind check) | not this concept: the kind vocabulary is the plan's and language's |

**Lost that should not have been: nothing of this concept.** Its 12 tests, 17 rows and 7 scenarios are all at the tip.
What the stage-1 deletions took from `MetadataUseCase` belongs to the operators, execute and programs sheets.

## 5. The drafted public interface

The listing is `api.txt` beside this sheet: **34 public items plus 6 enum variants**. 18 lines are `[main]`, 22 are
`[changed]` (9 of them the 7 types moved from `values`, unchanged inside), none is `[new]`. Nothing here is agreed.

**The claim it starts from.** Appendix A 3.18 (`PLAN.md:1446-1456`) lists the four traits, "plus ryoql's
`DescribeUseCase` for the grammar", with operator descriptions from `OperatorsItem::describe`. Checked against the
code: the four traits are right, word for word on both revisions. Two corrections: (a) the operators sheet drafted
`CatalogUseCase { operators, describe }` (the tip's `MetadataUseCase` moved to its owner), not `OperatorsItem::describe`,
so this crate is handed that; (b) the grammar is NOT re-offered here (decision 3).

### 5a. The calls it offers (all `[main]`)

The four traits and their four methods exactly as quoted in section 2, with `Send + Sync` kept. The request and report
shapes are main's. One pair of errors keeps main's inconsistency (`ListRelationsError::Language(LanguageError)` against
`DescribeRelationError::Language { reason: String }`): both are matched by the command line's `remedy.rs`, and making
them one is not worth a change to two edges; noted in section 11.

### 5b. What is handed in

No port of its own. The relation pair is handed outbound/language's `LanguagePort` (that listing keeps
`relations` and `supports` as `[main]`); the operator pair is handed domain/operators' `CatalogUseCase`.

### 5c. How it is built (the 8 `[changed]` constructor lines)

```
pub struct ListRelationsUseCaseImpl<L: LanguagePort>         @new
pub fn new(language: L) -> Self                              @new
pub struct DescribeRelationUseCaseImpl<L: LanguagePort>      @new
pub fn new(language: L) -> Self                              @new
pub struct ListOperatorsUseCaseImpl<C: CatalogUseCase>       @new
pub fn new(catalog: C) -> Self                               @new
pub struct DescribeOperatorUseCaseImpl<C: CatalogUseCase>    @new
pub fn new(catalog: C) -> Self                               @new
```
- The relation pair: main's `new(language: Arc<dyn LanguagePort>)` (quoted in section 2) becomes generic over the
  port, as inbound/validate's draft made its use case generic over its two. Crossed once per request, so this is for
  one rule ("generics, not `dyn`") and costs nothing; an edge passes `Languages` (the language listing's type that
  implements the port) instead of `Arc::new(...)`.
- The operator pair: main's `new()` held nothing and called core's crate-private `kernel_metadata()`, which names
  execution's `MetadataUseCaseImpl` inside an application service. With `core` and `execution` gone, the catalog is
  handed in by the edge that composes (`ryo_operators::CatalogUseCaseImpl::new()`, on the operators listing), so this
  crate builds nothing of another crate's (decision 5).

### 5d. The types (7 moved from `values`; the names inside them)

- `RelationSummary`, `RelationDescription`, `LanguageDescription`, `ColumnDescription`, `ColumnKind`,
  `RelationReadability`, `AbsenceReason` move from `values::descriptions` with their fields unchanged (quoted in section
  2). Their only users at the tip are this concept's use cases, the SDK and the command line (`git grep -l` per name at
  `2a207464c`: core, sdk, cli and values only), so they belong with the concept, not in a shared vocabulary.
- Inside them, the names other sheets own: `RelationName` and `KindWord` become `ryo_relations::` (as the ryoql, plan,
  programs and validate listings write them; neither is on the relations listing yet: section 11); `Holds` and
  `UnknownName` are the relations listing's; `ColumnName` is the table listing's (that line is UNPROVEN until the
  table spike passes); `LanguageName` and `LanguageError` are the language listing's.
- `DescribeOperatorReport { operator: ryo_operators::OperatorDescription }`: `OperatorDescription` is on the operators
  listing; `OperatorSummary.summary` is its `Purpose`, which is not on that listing yet.
- `DescribeOperatorRequest.name` and `OperatorSummary.name` are `Box<str>`: main's type was values'
  `pub struct OperatorName(pub Box<str>);` (quoted in section 2), but the operators listing has taken the word
  `OperatorName` for the closed enum of 34 operators (the tip's `QueryOperator`). A reader's text is not yet an operator,
  so it cannot be the enum. Decision 2.
- The two `pub(crate)` helpers stay crate-private here: `resolve` (core's `relation_lookup`) and `summarize` /
  `describe` (from `values`, public there today, private here: their one caller each is in this crate).

### 5e. What stops being public

Nothing of the 27 that are public today: they move whole. `values::descriptions::{summarize, describe}` stop being
public; values' text `OperatorName` and `RelationName` go; the operator description types go to domain/operators. The
full list is api.txt's `# NO LONGER PUBLIC`.

### 5f. The counts

| Side | Public items for the concept | Traits |
|---|---|---|
| main `f4ae3bd802` | 27 in core, plus 9 in `values::descriptions` read only by it | 4 |
| tip `2a207464c` | 27 in core, plus the same 9 | 4 |
| drafted | 34 (27 carried, 7 moved in; 2 functions made private) | 4 |

## 6. Proof it brings

| Proof | Source (both revisions) | Fate |
|---|---|---|
| `lists_every_relation_ryo_owns` (pin 80), `fails_when_language_port_fails` | `core/.../list_relations/use_case_test.rs` | carried; the fake port is outbound/language's fixture (`LanguageFake`, `core/src/outbound/language/fixtures/fake.rs:26` today) |
| the five rows of `describe_relation` | `core/.../describe_relation/use_case_test.rs` | carried as they are |
| `lists_every_operator_in_documentation_order` (pin 34, the documented order) | `core/.../list_operators/use_case_test.rs` | restated only in its constructor (`ListOperatorsUseCaseImpl::new(CatalogUseCaseImpl::new())`); the pin and the order unchanged (decision 4) |
| `describes_one_operator`, `unknown_operator_is_refused` | `core/.../describe_operator/use_case_test.rs` | as above |
| the 3 rows of `relation_lookup` | `core/src/domain/relation_lookup/service_test.rs` | carried with `resolve` |
| the 3 rows of `descriptions` | `values/src/domain/descriptions/service_test.rs` | carried with `summarize` and `describe` |
| `the_decisions_it_hands_out_know_the_operators` | `core/src/application/metadata/service_test.rs` | restated against `CatalogUseCase` on the operators sheet (it tests the bridge, which goes) |
| the 4 features, 7 scenarios | `core/src/inbound/use_cases/*/acceptance.{feature,rs}`, run by `core/tests/bdd.rs` | carried; the crate gets its own `tests/bdd.rs` |
| SDK 12 tests, CLI 12 tests | the edges' `*_test.rs` | the edges' sheets; their expected values do not change |

**Missing:** nothing proves that the command line and the SDK give the same answer for the same question (they compose
separately today). One check would do it; it belongs to the edges' sheets, raised there.

## 7. Perf gains it carries

**None.** `perf-gains.tsv` has 54 rows; no row's `crate` column names `inbound/describe`, and no row's
`files_on_the_tip` names any file of this concept (`grep` for `metadata`, `descriptions`, `use_cases/list`,
`use_cases/describe`, `relation_lookup`, `resources/relation`, `resources/operator`, `query/describe`: no hit). Row by
row that is the whole check.

Why none can touch it: every gain in the table is on `Linter::lint` (loading rules, screening, running files, the
cross-file phase, the kept store, the workers). This crate is reached only by `tm explain`, `tm list` and the four
`ryo relation|operator` commands, once per request, and loads no rule, reads no file and starts no worker. Row 17 (the
Tastemaker check's rounds) and row 2 (bench composition in the SDK) are Tastemaker's `tm check` and the lint edge, not
these calls.

**Aim 3, the history.** No performance change in section 10.3 would have landed in this crate, and none crossed into
it. The changes that DID cross into it were not performance changes: five commits edited its tests only to move a
catalog-size pin when a relation or an operator was added (section 4). Under the draft they still would (decision 4).

## 8. Benchmarks that cover it

**None, today or after.** No benchmark calls it: the Ruff, Oxlint and Clippy linter benchmarks (`crates/ryo/languages/
{python,typescript,rust}/benchmarks/linters/...`) run rule packs through the lint path; the `benchmark` crate,
`benchmark-collector`, `benchmark_contracts`, `profile_tools`, the lint images (`executor/benches/lint_image`), the
provider benches and the retention matrix name nothing here (`git grep` at both revisions: no hit). The Tastemaker
benchmark project times `tm check`, which uses `query::validate` and `Linter::lint`, never the catalog calls.

What they call in this crate today: nothing. Afterwards: nothing. No benchmark changes because of this sheet.

## 9. Checks
IMPORTS: table, relations, operators, language
CROSSED: per request

- **Imports.** `table` (`ColumnName`, UNPROVEN), `relations` (`all`, `relation_named`, `class_of`, `reason_name`,
  `name`, `summary`, `columns`, `column_name`, `kind`, `vocabulary_of`, `words_of`, and the names in 5d), `operators`
  (`CatalogUseCase`, `OperatorDescription`, `Purpose`), `language` (`LanguagePort`, `Answers`, `RelationAnswer`,
  `Absence`, `LanguageName`, `LanguageError`). All allowed: inbound imports domain and outbound, and no other inbound
  crate. Not `ryoql` (decision 3), not `symbols` (`reason_name` answers a `NameText` read through its field only; a
  direct dependency is not needed, not checked by a build), not `validate` or `lint`.
- **No layout word** in any signature: no table, column store, row slot or cell type crosses; `ColumnName` and
  `ColumnKind` are a column's written name and written kind.
- **How often it is crossed.** Once per request: one `list_*` or `describe_*` call per question. Inside
  `describe_relation`, the language port is crossed twice (`relations` once, `supports` once, which loops over the
  linked languages). Never per file, per row or per cell.
- **No `dyn`** left in the crate (5c).

## 10. Decisions for the owner

**1. Keep `LanguagePort` handed to the relation pair, though `relations` only returns the catalog.**
Example: the real adapter's `relations` is `Ok(RelationList(all().to_vec()))` (section 2), and both edges build a port
with no language (`language::factory::use_case(Vec::new())`) just to list relations. The relations sheet (its decision
3) recommends calling `ryo_relations::all()` directly and dropping the wrapper. But these two use cases are the
port's only callers of `relations` and `supports` on the tip: dropping the call here would leave a designed port method
with no caller, and would drop the tested failure path (`fails_when_language_port_fails`, the `catalog_down` scenario).
What we get by keeping: no contract dropped, every test carried, the five SDK calls unchanged. Cost: one empty
`Languages` at two edges, once per request.
**My pick:** keep the port, as drafted. Revisit with the relations sheet's decision 3 as one choice, not two.

**2. The written name of an operator has no type.**
Example: main's `DescribeOperatorRequest { name: OperatorName }` used values' `pub struct OperatorName(pub Box<str>);`.
The operators listing now uses `OperatorName` for the enum of operators (`pub enum ryo_operators::OperatorName`), and
validate's `ValidateReport.operators: Vec<ryo_operators::OperatorName>` and ryoql's `CheckedQuery.operators` follow it.
A reader asking `ryo operator describe sort` hands text that names no operator; it must be refused
(`UnknownOperator`), so it cannot already be the enum.
Why: "one definition per concept": the written name appears in this crate's request and summary, in
`OperatorDescription.name`, and in the SDK's `ryo::query::OperatorName`.
What we get: the draft uses `Box<str>` for now, so nothing is defined twice.
**My pick:** the operators sheet adds the written name to its listing (a text type, and `OperatorName::named(&str) ->
Option<OperatorName>` with its inverse), and this crate's two `Box<str>` fields take it. Then `describe_operator` looks up
by name in one call instead of describing all 34 operators to find one (the tip's loop, harmless at 34).

**3. The grammar stays ryoql's; the edge composes it.**
Example: the SDK's `query::describe` builds `DescribeUseCaseImpl::new(winnow())` from ryoql (section 3); the command
line has no `query describe`. Appendix A put ryoql's `DescribeUseCase` "plus" on this sheet.
Why: re-offering it here would be a second definition of a one-method use case, and this crate would import `ryoql`
for nothing else.
What we get: `query::describe` unchanged; one definition; this crate's imports stay four.
**My pick:** leave `DescribeUseCase` on the ryoql sheet (`[main]` there); the SDK keeps composing it. This needs the
edges' rule to allow an edge to compose a domain use case directly (Appendix A 2: "plus the factories of the crates they
compose"), which it already does for `CatalogUseCaseImpl`.

**4. The catalog-size pins live in four places.**
Example: the relation count is written as a literal in `list_relations/use_case_test.rs`, its feature, the SDK's
`relation/list/entry_test.rs` and the command line's `relation/list/command_test.rs`; it moved 77 -> 79 -> 80 in four
commits on 26 Nov, one of them because two relations were added "without it" (`9253e01271`). The operator count moved
26 -> 34 in three commits.
Why: aim 3. Adding a relation is a relations change; today it edits this crate and both edges too.
What we get with my pick: a relation or operator lands in its own crate alone; the tripwire is kept where the catalog
is defined.
**My pick:** keep ONE pinned count each, on the relations and operators sheets (where the list is), and have this
crate's cases assert that the listing has `ryo_relations::all().len()` relations and `CatalogUseCase::operators().len()`
operators, in the documented order. The edges' pins follow the same way on their sheets.

**5. The operator pair is handed `CatalogUseCase` (generic), not built inside.**
Example: today `ListOperatorsUseCaseImpl::new()` builds nothing and calls core's `kernel_metadata()`, which names
`execution`'s `MetadataUseCaseImpl` (section 3); the command line writes `service: ListOperatorsUseCaseImpl,` as a
unit literal.
Why: "the edge owned by the sdk composes"; an inbound crate building a domain crate's implementation is the bridge that
existed only because `core` sat between them.
What we get: this crate builds nothing; a test can hand a fake catalog; each edge writes one more argument.
**My pick:** `new(catalog: C)` with `C: CatalogUseCase`, as drafted.

## 11. Not checked

- **The SDK's and command line's `schemas.rs`** (the translation from these types to `ryo::relation::*`,
  `ryo::operator::*` and the command line's output) were not read line by line; only their imports were grepped. Each
  moved field name above must be re-read against them when the edges' sheets are written.
- **`acceptance.rs` step files** were counted, not read; whether their steps name `core` internals beyond the use case
  is not checked.
- **`ryo_relations::RelationName` and `ryo_relations::KindWord`** are named by this listing and four others but are on
  no listing yet; `check-types` cannot find their owner. Same for `ryo_operators::Purpose` (and the other fields of
  `OperatorDescription`: `FieldDescription`, `FieldName`, `FieldRequirement`, `OptionLine`, `Example`; `Example` is also
  the field type of ryoql's `Production`).
- **Whether `ListRelationsError::Language` and `DescribeRelationError::Language` should become one shape** was not
  weighed beyond 5a.
- **The `symbols` import**: whether reading `reason_name(..).0` without naming `NameText` needs `symbols` in
  `Cargo.toml` was not checked by a build (no Cargo, by rule).
- **Build 133** was not read for this concept (it exists unchanged on both sides, so the brief's rule did not call for
  it); whether 133 had a different shape for these four use cases is not checked.
- **`crates/ryo/docs/README.md`** names these use cases at both revisions; its text was not read.
- **Line counts** are `wc -l` of whole files (comments and doc lines included, tests excluded), not Rust lines by a
  tool.
- **Check scripts**: `check-quotes.py`, `check-types.py` and `check-graph.py` results are stated in the final report
  of this run, not here.
