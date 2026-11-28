# domain/relations
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.5.

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). Read only through
`git -C /workspaces/idiomatic-public-benchmark show|grep|ls-tree|log|diff`. A quote is one line, two spaces, `@rev:path:line`.
Paths without a root are under `crates/ryo/crates/relations/src/` (`R/` below). Lines naming a `ryo_table` type are
UNPROVEN until `domain/table/spike.md` records a pass.

## 1. Owns
The catalog of tables: which relations exist, their columns, what each column holds, and what each relation declares
of where its rows come from. One thing. The draft takes out what is not the catalog (section 5 and decision 1): the
cell layout types (to `domain/table`), the run's words (to `domain/symbols`), and a group of tree and notes words that
today live here only because this crate depends on nothing (decision 5).

## 2. On main (f4ae3bd802)
- **Where.** The crate `crates/ryo/crates/relations` (package `idiomatic-sh-ryo-relations`, library `ryo_relations`).
  13,067 Rust lines with tests, 8,916 without (tests, `fixtures.rs` and `cases.rs` are 4,151). 77 relations, one
  folder each under `R/domain/relations/collections/relations/<name>/` (`item.rs`, `constants.rs`, `item_test.rs`).
  Its only Cargo dependency is `strum`. No use case, no port: free functions, one sealed trait, macros.
- **Public items: 686.** Counted as `pub struct|enum|trait|fn|const|type` outside tests and the `fixtures` feature
  (140), the per-relation `constants.rs` functions `name`, `summary`, `column_name` (231) and unit types (77), the
  row struct and column enum `relation_row!` generates per relation (154), and `#[macro_export]` macros (84: 77 per
  relation plus `each_relation`, `relation_names`, `optional_relations`, `baseline_relations`, `kept_relations`,
  `relation_row`, `kinds`). The unnameable `sealed::Sealed` is not counted.
- **The contract, verbatim.** The item trait, with `type Row; type Column: Copy; type Words: AsRef<[Bits]>;`, the
  consts `CLASS`, `SCOPE`, `NOTES` and the functions `relation`, `columns`, `kinds`, `kind`, `words`:
  ```
  pub trait RelationsItem: sealed::Sealed + 'static {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:21
  ```
  the one list every crate that handles each relation expands, and the lookups over it:
  ```
  macro_rules! each_relation {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/catalog/service.rs:10
  macro_rules! relation_row {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/catalog/service.rs:173
  pub fn all() -> &'static [Relation] {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/service.rs:195
          pub const fn capability_relation(  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/service.rs:181
  pub fn is_declaring(relation: Relation) -> RelationDeclaring {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/service.rs:201
  pub fn is_key(column: Column) -> ColumnKeyness {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/service.rs:353
  ```
  and the typed row writer a language is handed:
  ```
          pub trait RowWriter:  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:260
  pub trait Rows<Row> {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:276
  pub trait Term: Copy {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/terms/types.rs:54
  ```
  Five `pub trait` lines: `Sealed`, `RelationsItem`, `RowWriter`, `Rows`, `Term`.
  Beside them, in the same `types.rs`, the cell and layout types: `pub enum Value {`
  `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:176`, `pub struct FactColumn(pub Vec<u64>);`
  `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:357`, `pub struct ColumnsView<'a> {`
  `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:362`.
- **Who implements it.** Only the crate: `RelationsItem` is sealed, and each relation's `item.rs` implements it
  through `relation_row!`. A language that tries is refused at compile time by
  `impl RelationsItem for Mine {`  `@f4ae3bd802:crates/ryo/crates/language/tests/ui/fail_relation_defined_by_a_language.rs:9`.
- **Who calls it** (files naming `ryo_relations`, by crate): operators 140, language 123, execution 120,
  query_language 107, core 104, execution_domain 93, execution_runtime 73, derived 39, values 38, sdk 13, cli 6.
  The SDK re-exports 53 items to the language crates (`pub use ryo_relations` in `sdk/src/resources/languages.rs`).
- **List and describe** are not here: `core`'s `ListRelationsUseCase` and `DescribeRelationUseCase` (with acceptance
  features) read the catalog through `core`'s `LanguagePort::relations()`. Same at the tip (section 3).
- **Proof.** 82 `#[test]`: one `item_test.rs` per relation (77, 78 case rows: each checks `words(row)`) and five
  service tests (answers 1, catalog 3, kinds 4, relations 9, terms 8 case rows). About 21 doctests (42 fences). No
  acceptance feature. The seal is proved by the compile-fail test above (it lives in `language`).
- **Tastemaker entries** (`crates/ryo/tastemaker.toml` at main): line 30 `local-dependency-allowlist` (no sibling),
  31 `internal-package-allowlist`, 44 `no-default-trait-methods` (includes `crates/relations/src/**`), 77
  `catalog-lists-every-item`, 131 `crate-relations`, 192 `nu-hex-relations` (`exposed = []`, no item trait).

## 3. On the perf branch (2a207464c)
- **Where.** The same crate. 14,228 lines with tests, 9,822 without (4,406). 80 relations: 39 `baseline`, 41
  `optional` (main: 39 and 38).
- **Public items: 717** (counted as for main: 150 + 240 + 80 + 160 + 87).
- **The contract actually called.** The item trait gained five declarations, all required of every item:
  ```
  pub trait RelationsItem: sealed::Sealed + 'static {  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:22
      type Words: AsRef<[Bits]>;  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:28
      const SCOPE: RowScope;  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:36
      const FORM: FactForm;  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:44
      const NODE_COLUMNS: &'static [(Self::Column, NodeColumn)];  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:48
      const EXTENT: Option<NodeExtent>;  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:52
      const SIZE: Option<RelationSize>;  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:56
      const KEYS: &'static [Column];  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:60
      fn words(row: Self::Row) -> Self::Words;  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:75
  ```
  An item states them in its macro, for example:
  ```
          form Filled(Many(Reference)),  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/lookup_scope/item.rs:39
          size PerElement,  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/lookup_scope/item.rs:40
          keys [Reference],  @2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/lookup_scope/item.rs:41
  ```
  and the catalog answers them by value:
  ```
  macro_rules! each_relation {  @2a207464c:crates/ryo/crates/relations/src/domain/catalog/service.rs:11
          pub fn fact_form(  @2a207464c:crates/ryo/crates/relations/src/domain/catalog/service.rs:208
          pub fn relation_size(  @2a207464c:crates/ryo/crates/relations/src/domain/catalog/service.rs:253
          pub fn key_columns(  @2a207464c:crates/ryo/crates/relations/src/domain/catalog/service.rs:278
  pub fn effective_extent(  @2a207464c:crates/ryo/crates/relations/src/domain/catalog/service.rs:332
  pub fn covers_own_tree_for(  @2a207464c:crates/ryo/crates/relations/src/domain/catalog/service.rs:358
  macro_rules! relation_row {  @2a207464c:crates/ryo/crates/relations/src/domain/catalog/types.rs:77
  pub fn all() -> &'static [Relation] {  @2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:185
          pub trait RowWriter:  @2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:261
  pub trait Rows<Row> {  @2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:277
  pub trait Term: Copy {  @2a207464c:crates/ryo/crates/relations/src/domain/terms/types.rs:54
  ```
  Still five `pub trait` lines, the same five.
- **Who implements it.** The same: sealed, 80 items; the compile-fail test is unchanged
  (`impl RelationsItem for Mine {`  `@2a207464c:crates/ryo/crates/language/tests/ui/fail_relation_defined_by_a_language.rs:9`).
- **Who calls it** (files naming `ryo_relations`, by crate): language 95, query_language 53, values 30, executor 22,
  core 12, sdk 9, execution_domain 8, cli 6, execution 1. The SDK re-exports 56 items, among them
  `pub use ryo_relations::domain::relations::collections::relations::interface::RelationsItem as CatalogRelation;`
  `@2a207464c:crates/ryo/crates/sdk/src/resources/languages.rs:237` and
  `pub use ryo_relations::domain::relations::service::all;`  `@2a207464c:crates/ryo/crates/sdk/src/resources/languages.rs:335`.
  What each crate reaches for (by `git grep -o "ryo_relations::..."`):
  | Crate | Mainly | Through |
  |---|---|---|
  | language | `RelationsItem` generically (`Reader<R: RelationsItem>`, fillers), `Rows`, `each_relation!` (11 files) and the per-relation macros | static: by type and by tokens |
  | executor | `relation_named` (9 files), `name`, `fact_form`, `node_extent`, `covers_own_tree_for`, `CellKind`, `Missing`, terms | by value, often by relation **text** (section 9) |
  | query_language | `Relation`, `CellKind`, `Vocabulary`, `relation_named`, `columns`, `column_name`, `kind`, `is_key`, `scope_of`, column-name constants; `CatalogPort` returns `all()` | by value |
  | values | `CellKind`, `Relation`, `Column`, `Value`, `columns`, `summary`, `column_name` (`descriptions` builds the listings) | by value |
  | core | `all()` behind `LanguagePort::relations()`, `class_of`, `reason_name`, `RowWriter` | by value |
  | sdk, cli | re-exports; `location`, `NO_SPAN`, `Locatedness` | by value |
- **List and describe.** Unchanged in place: `pub trait ListRelationsUseCase: Send + Sync {`
  `@2a207464c:crates/ryo/crates/core/src/inbound/use_cases/list_relations/interface.rs:7` and
  `pub trait DescribeRelationUseCase: Send + Sync {`
  `@2a207464c:crates/ryo/crates/core/src/inbound/use_cases/describe_relation/interface.rs:7`, over
  `    fn relations(&self) -> Result<RelationList, LanguageError>;`  `@2a207464c:crates/ryo/crates/core/src/outbound/language/interface.rs:21`,
  whose adapter is `        Ok(RelationList(all().to_vec()))`
  `@2a207464c:crates/ryo/crates/core/src/outbound/language/adapters/use_case/adapter.rs:75`. A second port wraps the
  same call for RyoQL: `pub trait CatalogPort: Send + Sync {`
  `@2a207464c:crates/ryo/crates/query_language/src/outbound/catalog/interface.rs:7`, adapter
  `        Ok(Relations(all().to_vec()))`  `@2a207464c:crates/ryo/crates/query_language/src/outbound/catalog/adapters/language/adapter.rs:15`.
- **Proof.** 85 `#[test]`: 80 item tests (81 case rows) and the five service tests (answers 1, catalog 14,
  kinds 4, relations 8, terms 8 rows). About 34 doctests (68 fences). No acceptance feature.
- **Tastemaker entries** (at the tip): 27 `local-dependency-allowlist`, 28 `internal-package-allowlist`, 42
  `no-default-trait-methods`, 72 `catalog-lists-every-item`, 121 `crate-relations`, 182 `nu-hex-relations`.

## 4. What changed between them
`git diff --stat f4ae3bd802 2a207464c -- crates/ryo/crates/relations`: 100 files, +1,417 / -256. Twelve commits
(`git log f4ae3bd802..2a207464c -- crates/ryo/crates/relations`), 19 to 27 Nov.

**Added**
| What | Commit | Why, from the commit |
|---|---|---|
| relation `binding-resolution` (optional) | 57df0568c7 (11-19) | "spike: binding-resolution contract (S1, red)" |
| relation `lookup-scope` (optional); its size and key | 7f03f3068b, 523a48d800, 459b39d42f (11-20, 11-21) | "the namespaces a name is looked up in, filled by today's parse-side semantics" |
| `FactForm`, `FilledKey`, `NodeColumn`, `RelationsItem::FORM`, `catalog::service::fact_form` | 3097109506 (11-20) | "the engine contract beside today's engine" |
| `RelationSize`, `NodeExtent`, `RelationsItem::{NODE_COLUMNS, EXTENT, SIZE, KEYS}`, `node_extent`, `relation_size`, `key_columns`; every item's `form`, `size`, `keys` clauses (77 items, about 6 lines each); `relation_row!` moved from `catalog/service.rs` to `catalog/types.rs`; the item macros take extra arguments | 8abefc84de (11-20) | "a relation says how its rows grow with the file and which keys it promises a fill by ... No engine reads the new parts yet" (74 filled items: 8 `PerFile`, 66 `PerElement`; 12 declare keys) |
| `effective_extent` | 23cf0575d0 (11-26) | the engine's reads and the numbered adapter's projection "take one rule" |
| `covers_own_tree_for` | 6dd5a7db92 (11-26) | moved from the executor into the catalog |
| `declared_terms` | 234cbf3c2b (11-25) | closed vocabulary in its declared namespace |
| relation `annotation-evaluation` (optional) | baee62831b (11-26) | "a shared optional catalog relation (WIP, frozen files to re-record)" |
| catalog case rows 3 -> 14 | 8abefc84de and later | the new declarations are each held by a row |

**Removed**, in one commit, 070d4a1275 (11-27): "the never-run functions nothing live names are deleted with their
tests (21 functions)". In this crate: `capability_relation` (main `service.rs:181`) with its case row
`        name: CaseName("optional capability identities round trip without weakening baseline"),`
`@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/service_test.rs:115`, and `is_declaring` (main
`service.rs:201`). The macro that generated `capability_relation` is left empty:
`    ( $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )* ) => {};`
`@2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:178`, and the type `is_declaring` returned
is left with no user anywhere: `pub enum RelationDeclaring {`
`@2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:336`. On main `is_declaring` was read by the
language contract's row checks (`language/src/inbound/use_cases/parse/fixtures/row_checks.rs:20`).

**Lost that should not have been.** Both deletions are of designed contract on "nothing live names it" evidence,
which is what the owner stopped on 27 Nov (memory "Contracts are not dead code"). The round trip from an optional
capability to its relation is the only check that the 41 capabilities and the 41 optional relations stay one set.
The draft restores both (decision 4).

**Changed in reach, not in code.** Callers fell from 11 crates to 9 because the crates that called the catalog
(operators, execution_runtime, derived) were deleted; files naming `ryo_relations` fell from 862 to 249.

## 5. The drafted public interface
The listing is `domain/relations/api.txt` beside this sheet: **447 public items** (503 tagged lines counting
19 trait members and 37 variant lines), against 686 on main and 717 at the tip. By tag: 444 `[main]`, 40 `[perf]`,
19 `[changed]`, 0 `[new]`. Everything below explains a line that is changed, a `[perf]` line kept, or a removal.

**The claim it starts from.** Appendix A 3.3 says "exists, unchanged" and lists `RelationsItem` with `CLASS`,
`SCOPE`, `FORM`, `EXTENT`, `SIZE`, `KEYS`, then says the layout types leave when `table` lands and `CellKind` moves
down. Checked: the trait as written there is the **tip's**, not main's (main has no `FORM`, `EXTENT`, `SIZE`, `KEYS`,
section 4); it omits `NOTES` and `NODE_COLUMNS`; and "unchanged" does not hold once `CellKind`, `Bits`, `Count`,
`Position`, `Offset`, `KindBits`, `PropertyId`, `FieldId`, `Span`, `Symbol`, `NameText`, `TextHash`, `NO_SYMBOL` are
owned by the table and symbols listings. The trait's shape survives; ten names in and around it change owner.

### 5a. Three ways in, all kept (decision 3)
1. **By type**: `RelationsItem`, sealed, one implementation per relation, used generically by the language crate
   (fillers, readers, collectors). Kept with every const, the five tip declarations included.
2. **By value**: 30 free functions over `Relation` and `Column` (`all`, `name`, `columns`, `kind`, `scope_of`,
   `fact_form`, `key_columns`, ...). A plan, the describe use cases and RyoQL use these; each is a `match` over a closed
   enum, fixed at compile time.
3. **As tokens**: `each_relation!` and the 80 per-relation macros, which the language contract expands to generate a
   method, a fill arm and a fixture row per relation (for example `language/src/domain/fillers/types.rs:324`).

### 5b. Changed lines
- `RelationsItem::Words: AsRef<[ryo_table::CellWord]>`, `words(row) -> Self::Words`: today
  `    type Words: AsRef<[Bits]>;`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/interface.rs:28`
  with `pub struct Bits(pub u64);`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:51` ("the
  number a value is stored as"). The table listing's `CellWord(pub u64)` is the same concept; one word survives, the
  table's (rule: one definition per concept). The 80 item tests change only the type they compare.
- `kind`, `kinds`, `RelationsItem::kind(s)`, `kind_name`: `CellKind` is the table's
  (`pub enum CellKind {`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:220`, carried as
  `[main]` on the table listing). `number_of` returns the table's `Count`.
- `is_key(column) -> ColumnKeyness`: today a hand list of 13 columns,
  `pub fn is_key(column: Column) -> ColumnKeyness {`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:330`,
  read by RyoQL as `            unique: if is_key(column) == ColumnKeyness::Key {`
  `@2a207464c:crates/ryo/crates/query_language/src/domain/resolving/service.rs:47`. The items already declare which
  column is unique, for example `            form Filled(Unique(Id)),`
  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/collections/relations/function/item.rs:36`, and the
  hand list repeats `Column::Function(FunctionColumn::Id)`. Drafted: the same signature, answered from the
  declarations. Whether the two agree today for all 13 is not checked (section 11).
- `Value`, `to_bits`, `from_bits`, `node_id`, `node_position`, `location`, `NO_SPAN`: only the table's type names in
  their signatures change. They stay here **only until decision 2**: the table listing puts `Value` under NO LONGER
  PUBLIC, but `to_bits` / `from_bits` (`pub fn to_bits(value: Value) -> Bits {`
  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:253`,
  `pub fn from_bits(kind: CellKind, bits: Bits) -> Value {`
  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:267`) are called by `language` and by the
  python, rust and typescript crates to encode a typed cell, and `relation_row!` builds `words` from them.

### 5c. `[perf]` lines kept, and why (decision 2)
`FORM`, `NODE_COLUMNS`, `EXTENT`, `SIZE`, `KEYS`, `FactForm`, `FilledKey`, `NodeColumn`, `NodeExtent`, `RelationSize`,
`fact_form`, `node_extent`, `effective_extent`, `covers_own_tree_for`, `relation_size`, `key_columns`, the three new
relations, `declared_terms`. They are what a relation is in logical terms (derived from the tree, or filled by the
language; which ids; how many rows per file; which columns are keys), which is catalog knowledge and the input a plan
needs to pass demand down when rules load (rows-trace causes 4 and 6). Who reads them today:
- `fact_form`: the executor (check_session.rs:658, :1216; pairs.rs:93; reader_run/packet.rs:205, :299) and the growth
  harness.
- `node_extent`, `covers_own_tree_for`, `effective_extent`: the executor and the language's numbered walk.
- `relation_size`, `key_columns`: **no engine code**; only the language's growth harness, which holds every
  language's fillers to them in its contract test:
  `use ryo_relations::domain::catalog::service::{fact_form, key_columns, relation_size};`
  `@2a207464c:crates/ryo/crates/language/src/domain/fillers/fixtures/growth.rs:13`. So they are proved, not dead.
- `NODE_COLUMNS`: read by nothing outside the crate's own catalog test. Kept (a designed declaration is never dropped
  for having no caller); reported.

### 5d. Restored from main (decision 4)
`capability_relation(OptionalCapability) -> Relation` and `is_declaring(Relation) -> RelationDeclaring`, both
`[main]`, with the case row deleted at 070d4a1275.

### 5e. What stops being public, and what leaves
| Side | Public | Stays | Private | Leaves to table | Leaves to symbols | Leaves, no home yet |
|---|---|---|---|---|---|---|
| main | 686 | — | — | — | — | — |
| tip | 717 | 447 (incl. 2 restored) | 241 | 11 | 4 | 16 |

(717 + 2 restored = 719 = 447 + 241 + 11 + 4 + 16.) Private: the 240 per-relation `constants::{name, summary,
column_name}` (no caller outside the crate at either revision; every caller uses `name`, `summary`, `column_name`)
and `ALL` (`        pub const ALL: &[Relation] = &[`
`@2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:44`, a second name for `all()`; one reader
in `executor`). To the table: `CellKind`, `Bits` (as `CellWord`), `Count`, `KindBits`, `Offset`, `Position`,
`PropertyId`, `FieldId` (as `FieldCode`), `Span` (as `ByteSpan`), and the two layout types
`pub struct FactColumn(pub Vec<u64>);`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:358` and
`pub struct ColumnsView<'a> {`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:363`, which is
what the table sheet already says. To symbols: `Symbol`, `NameText`, `TextHash`, `NO_SYMBOL`
(`pub const NO_SYMBOL: Symbol = Symbol(u32::MAX);`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/constants.rs:112`).
No home: decision 5. Every item is named in api.txt's `# NO LONGER PUBLIC`.

**How it is built.** Nothing: static data and compile-time code. No constructor, no port, no `dyn`.

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| 80 item tests (81 rows): `words(row)` per relation | `R/domain/relations/collections/relations/*/item_test.rs` at the tip | carried; the compared type becomes `[CellWord; N]` |
| catalog service test, 14 rows (form, extent, size, keys, scope, notes) | `fn succeeds() {`  `@2a207464c:crates/ryo/crates/relations/src/domain/catalog/service_test.rs:275` | carried as is |
| relations service test, 8 rows | `R/domain/relations/service_test.rs` at the tip | carried |
| the capability round trip row | `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/service_test.rs:115` | **restored from main** |
| terms 8 rows, answers 1 row | `R/domain/{terms,answers}/service_test.rs` | carried |
| kinds 4 rows | `R/domain/kinds/service_test.rs` | leave with `kinds!` (decision 5) |
| the seal, compile-fail | `@2a207464c:crates/ryo/crates/language/tests/ui/fail_relation_defined_by_a_language.rs:9` | carried; it should move here, beside the trait it proves (it names only this crate) |
| about 34 doctests | the crate | carried; those naming moved types are restated |
| growth harness (size and keys held per language) | `language/src/domain/fillers/fixtures/growth.rs` | stays with the language contract; it is the proof of `SIZE`/`KEYS` |
| `is_key` agrees with what items declare | — | **MISSING**: one row asserting the drafted `is_key` equals the 13-column list it replaces |
| the engine's copies of catalog facts match the catalog | — | **MISSING** while the copies exist (section 9: `TOKEN_COLS`, relation names as text) |
| acceptance feature | — | none on either side; `describe`'s features (in `core` today) cover listing and describing |

## 7. Perf gains it carries
No gain of section 10.3 has code in this crate: the catalog is declarations. These live gains depend on a
declaration here, so the draft keeps each one (`perf-gains.tsv` does not exist yet; numbers are section 10.3's):
| # | Gain | Measured | What here it depends on |
|---|---|---|---|
| 12 | fill on first use | not measured | `FactForm`: a `NodeTable` relation is never filled, a `Filled` one is filled at first read (executor `check_session.rs:1216`) |
| 44, 39 | facts as columns; native reads as columns | Django 32.46 -> 28.08; 16.29 -> 13.93 | `FactForm::NodeTable` and `NodeColumn`: the tree relations are read in place, not filled (`reader_run/packet.rs:205`). Which commit made the executor rely on it: not checked |
| 7 | file-local rules leave the cross-file phase | dagster 26.13 -> 22.12; 1,245 -> 477 MiB | `SCOPE` / `scope_of` ("which the partition of a step that reads it follows", interface.rs:34) |
| 31 | binding rules read provider facts | nine rules 1.82 -> 0.87 s | the relations `binding-resolution` and `lookup-scope` (added 11-19, 11-20); the link from #31 to these two is by date only |
| 3 | Python-only relations replaced by general forms | fastapi +12%, dagster +16 to +27% (a regression, open) | `annotation-evaluation` (baee62831b, 11-26) is the catalog side; whether other relations took part: not checked |

Room it must keep for gains not yet had: rows-trace causes 4 and 6 (a plan that sees `keys` and `size` can swap a
derived table for a keyed fill and pass columns and keys down to the language). That is why `SIZE`, `KEYS` and
`is_key` stay, and why decision 2 asks for one definition of a key.

## 8. Benchmarks that cover it
Nothing benchmarks the catalog itself (its lookups are `match` arms). What reaches its names:
| Benchmark | What it names here | After the draft |
|---|---|---|
| python `fillers` (tip only; per relation `ns-per-element`, `growth-percent`, held by Asymptote) | `Relation` and `Column` through the SDK: `    self, CellWord, Column, Count, FactAnswer, FillExtent, FilledColumns, NodeTable, Relation,` `@2a207464c:crates/ryo/languages/python/benchmarks/provider/performance/fillers/bench.rs:13` | unchanged in what it measures; `Relation`/`Column` keep their names |
| the growth harness (a contract test, not a bench) | `fact_form`, `key_columns`, `relation_size`, `all`, `columns` | unchanged |
| the lint images, the Tastemaker check, `benchmark` crate | no `ryo_relations` name at either revision (the `benchmark` crate names none) | untouched |
The bars this crate answers for: none directly. It must not add a run-time cost: every lookup stays a compile-time
`match`, and the engine must stop looking relations up by text per file (section 9).

## 9. Checks
IMPORTS: table, symbols
CROSSED: per row
- **Imports.** Today none (Cargo: `strum` only; Tastemaker line 27 allows no sibling). Drafted: `table` and `symbols`,
  both below `relations` in the agreed order. The Tastemaker allowlist (tip line 27) must list them.
- **Layout words in signatures.** Removed from this crate: `FactColumn(pub Vec<u64>)`, `ColumnsView`, `Bits`.
  `FactForm::NodeTable` names the tree (a logical thing every file has), not a layout. `FilledKey`'s doc says "how
  the engine indexes" a relation; its variants are logical (unique, many, whole); the doc wording is decision 2.
- **How often it is crossed.** The lookups: per request, when rules load, in a plan. `Rows<Row>::write` is crossed
  **once per row, through `dyn`**: `    rows: &mut dyn Rows<<I::Relation as RelationsItem>::Row>,`
  `@2a207464c:crates/ryo/crates/language/src/domain/fillers/service.rs:54`. That breaks the owner's rule (once per
  file or set, generic); the fix is on the language sheet (make the fill generic over `impl Rows<Row>`, or hand over
  whole columns), not here: the trait's shape allows both.
- **Catalog facts read at run time by text, and copied.** The executor looks relations up by name while running a
  file: `                    let rel = relations::service::relation_named(NameRef(name))?;`
  `@2a207464c:crates/ryo/crates/executor/src/check_session.rs:1465` (inside `visit`, which starts at :864), and
  `    match relation_named(NameRef(rel)) {`  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:597`;
  `relation_named` is a scan of all 80 names (`pub fn relation_named(text: NameRef<'_>) -> Option<Relation> {`
  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:191`). It names relations as text:
  `        "syntax-tree" | "semantic-syntax" => Some(Native::Tree(extent_of(run, rel))),`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:606`, and copies a relation's columns:
  `pub const TOKEN_COLS: [(&str, Ty); 5] = [("source", Ty::Node), ("target", Ty::Node), ("link-kind", Ty::Sym), ("file", Ty::File), ("span", Ty::Span)];`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:591`. The catalog's own doc says "so the engine never names
  one" (`types.rs:389`). Owner's rule: decided when rules load, never looked up while running. The draft keeps
  `relation_named` for RyoQL (text enters once) and asks the plan, programs and execute sheets to hold `Relation`.
- **One definition per concept.** Three statements of "which columns are keys": `is_key`'s list, the items' `KEYS`
  and `FilledKey::Unique`, and the language's `    const KEYS: &'static [Column];`
  `@2a207464c:crates/ryo/crates/language/src/domain/fillers/types.rs:82`, held equal only by a doctest
  (`/// assert_eq!(<TokenLinkFill as KeyedFillItem>::KEYS, key_columns(Relation::TokenLink));`
  `@2a207464c:crates/ryo/crates/language/src/domain/fillers/types.rs:78`). Also two names for a column's name: this
  crate's `NameRef<'static>` and `pub struct ColumnName(pub Box<str>);`
  `@2a207464c:crates/ryo/crates/values/src/domain/descriptions/types.rs:218`, beside the table listing's own
  `ColumnName(pub Box<str>)` (decision 1).
- **Generics, not `dyn`.** Nothing in the drafted listing is a `dyn`; the two ports that wrap `all()` (core's
  `LanguagePort::relations`, RyoQL's `CatalogPort`) are `dyn` today and are the describe and ryoql sheets' to settle.

## 10. Decisions for the owner
**1. What leaves the catalog for the table and for symbols.**
Example: the row type of `lookup-scope` has fields `NodeId`, `Symbol`, `Integer`, `FileId`, `Span`; its words are
`Bits`; its kinds are `CellKind`. `CellKind`, `Bits`, `Span`, `Position` are table shape; `Symbol` is the run's word.
Why: three crates now define the same things, and the table and symbols listings already claim these names.
What we get: one definition each, and this crate imports `table` and `symbols`, both below it. Also one word for a
column's name: today `values::descriptions::ColumnName`, this crate's `NameRef`, and the table listing's `ColumnName`.
**My pick:** move the 11 and 4 items listed in 5e; `Rows<Row>` and `RowWriter` stay here (they name the catalog's row
types, and the table sheet agrees); the table listing's `ColumnName` and `values`' are made one, owned by the table.

**2. Storage hints are catalog knowledge; one definition of a key.**
Example: `lookup-scope` declares `form Filled(Many(Reference))`, `size PerElement`, `keys [Reference]`; `function`
declares `Filled(Unique(Id))` and `is_key` repeats `Function::Id` by hand. `relation_size` and `key_columns` are read
only by the growth harness; `NODE_COLUMNS` by nothing outside this crate.
Why: these are logical facts (derivation, extent, cardinality, keys), exactly what a plan needs to pass demand down
when rules load; the alternative (moving them to the store) would make the plan import an outbound crate.
What we get: the inputs for rows-trace causes 4 and 6 stay in the domain; one key definition instead of three.
**My pick:** keep all five declarations here; derive `is_key` from them; make the language's `KeyedFillItem::KEYS`
read `key_columns` instead of restating it; reword `FilledKey`'s doc from "how the engine indexes" to "which column
identifies a row". Also settle **the typed cells**: the table sheet lists `Value` as no longer public, yet `to_bits` /
`from_bits` / `NodeId` / `FileId` / `Integer` / `Flag` are what five crates encode rows with. My pick: they move to
the table beside `CellKind` and `CellWord` (a change to the table listing); until then they stay here, marked.

**3. One sealed trait plus `all()`: does it serve every caller?**
Example: the language crate is generic over `R: RelationsItem`; the plan, describe and RyoQL call `columns(relation)`
and friends by value; the language contract expands `each_relation!` and the per-relation macros; the executor calls
`relation_named` by text inside `visit`.
Why: the three static ways cost nothing at run time; the fourth (by text, at run time) is the one that does, and it is
outside this crate. Two ports (`LanguagePort::relations`, `CatalogPort::relations`) exist only to return `all()`.
What we get: the catalog stays a domain crate with no use case and no port (a trait with one implementation would be
an abstraction with one user).
**My pick:** yes, keep the three ways; write them as the contract (5a); the engine holds `Relation` values resolved
once at load; recommend to the describe and ryoql sheets that they call `ryo_relations::all()` directly (domain may
import domain) and drop the two wrapper ports.

**4. Restore what 070d4a1275 deleted.**
Example: `capability_relation` and its round-trip row; `is_declaring`, whose type `RelationDeclaring` is still
public with no user.
Why: deleted on "nothing live names it", the evidence the owner ruled out on 27 Nov.
What we get: the check that capabilities and optional relations are one set; no orphan public type.
**My pick:** restore both from main (`[main]` in api.txt) with the row. If the owner wants `is_declaring` gone, the
type goes with it, by his word.

**5. Sixteen items that are not the catalog and have no home on any sheet.**
Example: `Flags`, `TOKEN`, `IMPLIED`, `NO_FIELD`, `KIND_LIMIT`, `Extent` and the five `elements` types (a file's tree
and its embedded segments); `kinds!` (a language's kind numbering over `strum` enums,
`macro_rules! kinds {`  `@2a207464c:crates/ryo/crates/relations/src/domain/kinds/service.rs:13`); `Bytes`, `Words`,
`Half`, `NameNote` (notes and buffers). Also the cross-file name cell `Name` with `NamePlace`, `PlaceNumber`,
`Origin`, `WorkerNumber`, `CROSS_FILE`, `CROSS_PLACE`, kept here for now.
Why: they sit here because this crate depended on nothing, so everything could import it; "we do not colocate by
mechanism". What we get: a catalog that owns one thing.
**My pick:** tree words to the table's tree half (beside `KindBits`, `FieldCode`), or to `outbound/language` if the
owner splits the tree out of `table`; `kinds!`, `Bytes`, `NameNote` to `outbound/language`; `Words` to
`outbound/store`; `Half` to the table; the cross-file name to `symbols`. Each needs a line on that sheet; until then
each is a "no home" row for `ledger.tsv` and `INDEX.md`.

## 11. Not checked
- Whether `is_key`'s 13 columns equal what the items declare (`Unique` keys and node ids). The drafted `is_key` may
  answer differently for some column; RyoQL's `unique` flag would change. Needs a one-off comparison.
- Per-language: which of the nine crates use `Value`, `to_bits`, `from_bits` through the SDK and how often per file
  (I saw python, rust, typescript, toml, go by file name only).
- The 249 tip files naming `ryo_relations` were counted by crate, not read; "mainly reaches for" in section 3 is from
  `git grep -o` frequencies, not from call sites. Main's 862 were not broken down by item.
- How often `Rows::write` is called per file at the tip, and what the `dyn` costs; no profile was read.
- Which call sites of `relation_named` in the executor's 9 files run per file and which per request; only
  `check_session.rs:1465` was placed (inside `visit`).
- The link between perf gains 31, 44, 39, 3 and specific commits here is by date and subject, not by reading
  `engine/STATE.md`.
- Whether the SDK's re-export of `NodeExtent`, `RelationSize`, `FactColumn` (tip only) has a user in the language
  crates.
- Build 133: not read for this crate (CONTRACT-SHEETS.md says the crate is the same there).
- Doctest counts are fence counts halved, not a run of `cargo test --doc`.
- No `tm check` on a skeleton of the drafted listing (the scoper's step); no check script exists yet.
- The path form: api.txt writes flat paths (`ryo_relations::Relation`); whether the crate re-exports at its root or
  keeps today's `domain::...` paths is not decided.
- The Tastemaker `collection-members` and `item-fixtures` entries (tip lines 59, 62) mention relation items in the
  language crates; not read for what they require of this crate.
