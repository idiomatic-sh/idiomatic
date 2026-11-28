# domain/table — contract sheet (coordinator draft, verified against git)

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139).
Read-only: `git show <rev>:<path>`. No checkout. Quotes are `@rev:path:line`.

## 1. Owns
What a table is, in logical words only: its columns, their kinds, its rows, and its cells.
(One thing. It does not own how cells are laid out; that is the store's job.)

## 2. On MAIN
The row table is the concept's current home.
- `crates/ryo/crates/values/src/domain/rows/types.rs:10-16` — `pub struct Table { kinds, cells, count }`
  — "Flat row values; ownership/publication policy belongs to the caller."
  — `@f4ae3bd802:crates/ryo/crates/values/src/domain/rows/types.rs:10`
- Row indexing types: `values/src/domain/rows/types.rs:4` (`pub struct RowIndex(pub usize)`),
  `:7` (`pub struct RowCount(pub usize)`).
- Writing rows: `relations/src/domain/relations/types.rs:276` —
  `pub trait Rows<Row> { fn write(&mut self, row: Row); ... }`
  — `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:276`
- Writer bundle: `relations/src/domain/relations/types.rs:260` — `pub trait RowWriter` (macro-generated).
- Implementers and callers: not yet mapped (see Not checked).
- Tests: not yet mapped.

## 3. On the TIP
The computed table is a column table, and it is in the proof-of-concept crate.
- `crates/ryo/crates/executor/src/finish.rs:147-153` —
  `pub struct Table { pub names: Names, pub tys: Tys, pub cols: Vec<Vec<u64>>, pub n: usize }`
  — "A column table." — `@2a207464c:crates/ryo/crates/executor/src/finish.rs:148`
- Constructor: `finish.rs:217-221` — `pub fn retained(names: Names, tys: Tys, rows: usize) -> Self`
  — `@2a207464c:crates/ryo/crates/executor/src/finish.rs:217`
- Cell kinds: `finish.rs` `pub enum Ty` (line ~101) and `pub struct V(Ty2, u64)` (line ~123).
  Exact lines to be confirmed in the next pass.
- Callers: `.cols` is touched on 179 lines in 25 files (from the plan; not re-counted here).

## 4. What changed
- MAIN has a row table (`kinds`, `cells`, `count`), the TIP has a column table (`names`, `tys`, `cols`, `n`).
- The TIP's table has public fields; MAIN's has public fields too.
- Move from row to column layout: builds 92 to 125 (per plan §10.3, rows 5 and 22; not independently re-verified).
- Lost that should not have been: none identified yet.

## 5. Drafted public interface
The listing is `domain/table/api.txt` beside this sheet (121 lines; 48 drafted public items plus the
`# NO LONGER PUBLIC` section). This part explains every line of it that is changed or new. Nothing here is
agreed, and nothing here is proved: the whole part is UNPROVEN until `domain/table/spike.md` records a pass
(see decision 3 and Not checked).

**The claim it starts from.** The plan's Appendix A §3.1 (`next/build/ports/stage2/contracts/PLAN.md:1061-1091`)
drafts `TablePort`, `TableRead`, `TableWrite`, `TreeRead`, `TreeWrite`. I checked each name against both
revisions: `pub trait TablePort`, `pub trait TableRead` and `pub trait TableWrite` appear in **no** `.rs` file at
either revision (`git grep -l` over `crates/ryo/*.rs` at both: no hit). So the three traits are `@new`, as the
first draft said. `pub struct ColumnNumber` is likewise absent at both revisions: `@new` as a name, though it is
main's `ColumnIndex` renamed. `pub struct Shape` exists at both revisions but means something else — a
language's expression-shape index, `@f4ae3bd802:crates/ryo/crates/language/src/domain/shapes/types.rs:21`
(`pub struct ShapeIndex(pub(super) RefCell<RetainedKeyIndex<Position, ShapeWord>>);`) — so the drafted `Shape`
is `@new` and the word collides; see Not checked.

### 5a. What is handed in: the store's contract

```
pub trait TablePort {                                                  @new
    type Table;                                      // opaque to every caller
    type Reader<'t>: TableRead where Self: 't;
    type Writer: TableWrite<Table = Self::Table>;
    fn read<'t>(&'t self, table: &'t Self::Table) -> Self::Reader<'t>;
    fn write(&mut self, shape: &Shape) -> Self::Writer;
    fn release(&mut self, table: Self::Table);
}
```
`release` is the one method with a live counterpart: on the tip a table's columns go back to the worker's
thread-local pool in `Drop`, and the pooling is `Table::release`:
```
pub fn release(&mut self) {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:258
impl Drop for Table {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:209
```
with the pool itself at `pub const POOL_CELLS: usize = 1 << 22;`
`@2a207464c:crates/ryo/crates/executor/src/finish.rs:163`, `pub const CLASSES: usize = 20;`
`@2a207464c:crates/ryo/crates/executor/src/finish.rs:186` and
`pub fn take_col(p: &mut ([Vec<Vec<u64>>; CLASSES], usize), n: usize) -> Vec<u64> {`
`@2a207464c:crates/ryo/crates/executor/src/finish.rs:192`. **None of that is in the drafted interface**: the
pool is the store's, and `release` is the one word the contract needs for it. That is perf gain 37 (pools for
columns and tables, -14 to -17%) kept behind one call — see section 7.

### 5b. Reading: `TableRead`

```
pub trait TableRead {                                                  @new
    fn shape(&self) -> &Shape;
    fn rows(&self) -> RowCount;
    fn cell(&self, row: RowNumber, column: ColumnNumber) -> CellWord;
    fn column_into(&self, column: ColumnNumber, into: &mut Vec<CellWord>);
    fn matching(&self, column: ColumnNumber, key: CellWord, into: &mut Vec<RowNumber>);
}
```
Each line against what exists:
- `shape()` replaces two public fields of the tip's table: `pub names: Names,`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:149` and `pub tys: Tys,`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:150`, whose types are
  `pub type Names = Rc<Vec<String>>;` `@2a207464c:crates/ryo/crates/executor/src/finish.rs:143` and
  `pub type Tys = Rc<Vec<Ty>>;` `@2a207464c:crates/ryo/crates/executor/src/finish.rs:144`. The `Rc` is the
  store's sharing decision (gain 22: columns and indexes set up once per file version) and does not cross.
- `rows()` replaces `pub n: usize,` `@2a207464c:crates/ryo/crates/executor/src/finish.rs:152`, typed as main's
  `pub struct RowCount(pub usize);` `@f4ae3bd802:crates/ryo/crates/values/src/domain/rows/types.rs:7`.
- `cell()` is main's
  `pub fn cell(row: &[Value], index: ColumnIndex) -> Result<Value, RowsError> {`
  `@f4ae3bd802:crates/ryo/crates/values/src/domain/rows/service.rs:14` **without the `Result`**, because the
  shape is checked once at `write(&Shape)` and not per cell; and without `Value`, the 16-byte tagged cell
  (`pub enum Value {` `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:176`, with
  `const _: () = assert!(size_of::<Value>() == 16, "a cell is sixteen bytes");` at
  `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:194`). A caller reads a `CellWord` and
  the column's `CellKind` from the shape. Rationale: the tip already works in words, not tagged cells
  (`pub cols: Vec<Vec<u64>>,` `@2a207464c:crates/ryo/crates/executor/src/finish.rs:151`), and AWB measured
  0.401 s per run for tagged reads and 0.165 s for widening (sheet 7 of CONTRACT-SHEETS.md, alternative C).
- `column_into()` is the live per-set read, behind the port:
  `pub fn extend_into(self, out: &mut Vec<u64>, n: usize) {`
  `@2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:46` and
  `pub fn translated(self) -> Vec<u64> {`
  `@2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:59`, over
  `pub enum Column<'a> {` `@2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:21`. This is the call
  that keeps gain 39 (native reads as columns, Django 16.29 -> 13.93): one typed pass per column instead of
  `pub fn at(self, row: usize) -> u64 {`
  `@2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:30`, which is per cell.
- `matching()` is `@new`. It is the one call the drafted set adds that no live signature matches: the rows of a
  column with one key, so select and the joins never ask per row. Its live equivalent is inside the executor's
  index machinery, not a public call.

### 5c. Writing: `TableWrite`

```
pub trait TableWrite {                                                 @new
    type Table;
    fn row(&mut self, cells: &[CellWord]);
    fn column(&mut self, cells: &[CellWord]);
    fn gather(&mut self, from: &impl TableRead, rows: &[RowNumber], columns: &[ColumnNumber]);
    fn append(&mut self, from: &impl TableRead);
    fn finish(self) -> Self::Table;
}
```
- `row()` is both sides' row append, as a trait method: main's
  `pub fn push_row(table: &mut Table, row: &[Value]) {`
  `@f4ae3bd802:crates/ryo/crates/values/src/domain/rows/service.rs:38` (unchanged at the tip,
  `pub fn push_row(table: &mut Table, row: &[Value]) {`
  `@2a207464c:crates/ryo/crates/values/src/domain/rows/service.rs:12`) and the tip executor's
  `pub fn push_row(&mut self, row: &[u64]) {`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:279`.
- `column()` generalises `pub fn push_const_col(&mut self, x: u64, n: usize) {`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:252` from one repeated word to a given column.
- `gather()` and `append()` are `@new`. They are the reason the contract can be crossed per set: select,
  project and the joins are one `gather`, union is one `append`.
- `finish()` replaces the tip's three constructors:
  `pub fn retained(names: Names, tys: Tys, rows: usize) -> Self {`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:218`,
  `pub fn with(names: impl Into<Names>, tys: impl Into<Tys>) -> Self {`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:223` and
  `pub fn with_rows(names: impl Into<Names>, tys: impl Into<Tys>, n: usize) -> Self {`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:227`. The row-count hint those carry (gain 37's
  "columns taken with that room") survives as `Shape` plus the store's own choice at `write`; whether a hint
  must be in `write`'s signature is open — see Not checked.

`TableFill` (`row`, `column`) is the object-safe part of `TableWrite`, as the plan states
(`next/build/ports/stage2/contracts/PLAN.md:1338`). `@new`.

### 5d. The tree: `TreeRead`, `TreeWrite`

These two are the only drafted traits whose every method is a live call, so they are tagged `[perf]` in
api.txt, not `[new]`. Today they are free functions over a nominal type:
`pub struct NodeTable(pub(crate) NodeTableEngine);`
`@2a207464c:crates/ryo/crates/language/src/domain/node_table/types.rs:31` and
`pub struct NodeTableBuilder<'a>(pub(crate) BuilderState<'a>);`
`@2a207464c:crates/ryo/crates/language/src/domain/node_table/types.rs:287`. Reading, each becoming a
`TreeRead` method (the `table: &NodeTable` first parameter becomes `&self`):
```
pub fn kind_of(table: &NodeTable, id: Position) -> KindBits {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:945
pub fn parent_of(table: &NodeTable, id: Position) -> Position {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:965
pub fn span_of(table: &NodeTable, id: Position) -> ByteSpan {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:989
pub fn field_of(table: &NodeTable, id: Position) -> FieldCode {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:1021
pub fn subtree_end_of(table: &NodeTable, id: Position) -> Position {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:1070
pub fn children_of(table: &NodeTable, id: Position) -> &[Position] {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:1095
pub fn ids_of_kind(table: &NodeTable, kind: KindBits) -> &[Position] {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:1125
pub fn ids_of_field(table: &NodeTable, field: FieldCode) -> &[Position] {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:1155
pub fn node_count(table: &NodeTable) -> Count {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:1306
pub fn element_count(table: &NodeTable) -> Count {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:1327
```
Writing, each becoming a `TreeWrite` method:
```
pub fn enter_node(  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:295
pub fn leave_node(builder: &mut NodeTableBuilder<'_>, end: Offset) {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:381
pub fn token(  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:496
pub fn node_property(  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:619
pub fn diagnose(  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:705
```
and the pair that opens and closes a build, which the drafted traits do NOT carry (an open question):
```
pub fn builder(table: &mut NodeTable, kinds: Count) -> NodeTableBuilder<'_> {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:43
pub fn finished<'b>(builder: &'b mut NodeTableBuilder<'_>) -> &'b NodeTable {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:862
```
**Finding:** the tip's `node_table` service is 1,578 lines with 42 `pub fn`, and the draft names 15 of them.
The other 27 (`ordinal_of`, `kind_codes`, `field_codes`, `parent_ids`, `starts`, `ends`, `kinds_present`,
`decoded_of`, `property_of`, `implied_of`, `acceptance_of`, `syntax_error_count`, `own_end`, `is_token`,
`embedded_segments`, `classify_fields`, `classify_written_fields`, `embed`, `enter_implied_node`,
`enter_unlocated_node`, `decoded_token`, `node_property_fmt`, `diagnose_with_metadata`,
`set_syntax_acceptance`, `structured`, `text_of` and `builder`/`finished` above) have no line in the draft.
Several are plainly layout (`kind_codes`, `parent_ids`, `starts`, `ends` hand out `&[u16]` / `&[u32]`
directly). Whether the tree belongs on this sheet at all, or is a second concept, is for the owner; this
sheet keeps the plan's claim and records the gap. Note also that `node_table` does not exist on main:
`crates/ryo/crates/language/src/domain/node_table/service.rs` is absent at `f4ae3bd802` (`git show` fails:
`fatal: path ... does not exist in 'f4ae3bd802'`), so every `TreeRead`/`TreeWrite` line is a tip-only quote.

### 5e. The types that cross it

`CellKind` is carried over unchanged, six variants, from
`pub enum CellKind {` `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:219`
(`Node, Symbol, File, Span, Number, Flag`; the same at the tip,
`pub enum CellKind {` `@2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:220`). It moves
down out of `relations`, as the plan says (Appendix A §3.3). The cell word is the tip's
`pub struct CellWord(pub u64);` `@2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:289`,
which exists on the tip only (`git grep -l "pub struct CellWord"` at `f4ae3bd802`: no hit).
`RowCount` is main's verbatim (above); `RowNumber` is main's
`pub struct RowIndex(pub usize);` `@f4ae3bd802:crates/ryo/crates/values/src/domain/rows/types.rs:4` renamed;
`ColumnNumber` is main's `pub struct ColumnIndex(pub u32);`
`@f4ae3bd802:crates/ryo/crates/values/src/domain/plan_parts/types.rs:54` moved out of `values::plan_parts`
and renamed. `Count`, `KindBits`, `Offset`, `Position`, `PropertyId` are main's verbatim at
`@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:58,42,24,28,377`. `FieldCode` is
main's `pub struct FieldId(pub u16);`
`@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:372` renamed to the word the tip's
tree service already uses. `ByteSpan` is main's
`pub struct Span {` `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:167` renamed, so
that `Span` the cell kind and `ByteSpan` the tree's stretch are not one word. `Shape` and `ColumnName` and
`NameText` are `@new`. The one error type is main's
`pub enum RowsError {` `@f4ae3bd802:crates/ryo/crates/values/src/domain/rows/errors.rs:5` renamed
`TableError`, one variant (`NoSuchColumn { index: u32 }`), kept because the shape check at `write` can still
refuse a column.

### 5f. How it is built

Nothing. The crate has no implementation and no constructor. `outbound/store` implements `TablePort`; an edge
builds the store and hands it in; callers are generic over `S: TablePort`, never `dyn`. That is the whole
reason the crate is in `domain` (plan §3.1: operators, in `domain`, cannot import `outbound/store`).

### 5g. What stops being public, and the counts

| Side | Public items in the concept today | Stop being public / move |
|---|---|---|
| main `f4ae3bd802`, `values::domain::rows` (`types.rs`, `service.rs`, `errors.rs`) | **10** (3 types, 6 functions, 1 error) | 7 |
| main `f4ae3bd802`, the layout types in `relations` the plan moves down | 3 (`FactColumn`, `ColumnsView`, `Value`) | 3 |
| perf tip `2a207464c`, the table concept in `executor/src/finish.rs:90-300` | **26** (counted by `^pub (struct|enum|fn|trait|type|const)` and `^    pub fn ` in that range) | 17 |
| drafted | **48** public items in api.txt | — |

Named, with why, in api.txt's `# NO LONGER PUBLIC` section. The three that matter most:
`pub cols: Vec<Vec<u64>>,` `@2a207464c:crates/ryo/crates/executor/src/finish.rs:151` (the layout word the
contract must not expose — check 2 of section 9), `pub struct V(Ty2, u64);`
`@2a207464c:crates/ryo/crates/executor/src/finish.rs:123` with `pub enum Ty {`
`@2a207464c:crates/ryo/crates/executor/src/finish.rs:101` and
`pub fn ty_of(k: CellKind) -> Ty {` `@2a207464c:crates/ryo/crates/executor/src/finish.rs:110` (a second kind
type and the translation between it and `CellKind`: one kind word survives, so `ty_of` has nothing to do), and
`pub struct ColumnsView<'a> {` `@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:362`
with `pub struct FactColumn(pub Vec<u64>);`
`@f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:357` (replaced by
`TablePort::Reader<'t>`, which is the store's own type and not a borrowed `&[FactColumn]`).

## 6. Proof it brings
Short answer: **almost none carries, and the gap is the biggest risk on this sheet.** The concept's own tests
are 2 case rows on main and 0 at the tip; everything that really exercises a table is a test of another
crate, written against that crate's table type.

### 6a. What the concept has on main (f4ae3bd802)
One test file, two `#[test]` entries over a 2-row case table:
```
fn succeeds() {  @f4ae3bd802:crates/ryo/crates/values/src/domain/rows/service_test.rs:38
fn refuses() {  @f4ae3bd802:crates/ryo/crates/values/src/domain/rows/service_test.rs:42
```
with the two rows
```
        name: CaseName("the_cell_at_a_column_is_the_value_there"),  @f4ae3bd802:crates/ryo/crates/values/src/domain/rows/service_test.rs:18
        name: CaseName("a_column_past_the_row_is_refused"),  @f4ae3bd802:crates/ryo/crates/values/src/domain/rows/service_test.rs:24
```
Both test `cell()` only. No acceptance feature, no scenario, no allocation test, no doctest on the type.
Counted across the whole `values` crate: 30 `#[test]`; of those, the table-ish concepts are `rows` 2,
`compact_table` 3 (`service_test.rs` 2 + `service_test/access_test.rs` 1), `table_words` 3
(`service_test.rs` 2 + `service_test/packing_test.rs` 1).

### 6b. What the concept has at the tip (2a207464c)
**Zero.** `crates/ryo/crates/values/src/domain/rows/service_test.rs` does not exist at `2a207464c` (`git
cat-file -e` fails: `fatal: path 'crates/ryo/crates/values/src/domain/rows/service_test.rs' exists on disk,
but not in '2a207464c'`), and `crates/ryo/crates/values/src/domain/rows.rs` at the tip no longer declares the
module (main declares `#[cfg(test)] mod service_test;`, the tip does not). `git diff f4ae3bd802 2a207464c --
crates/ryo/crates/values/src/domain/rows/` is `service.rs | 35 +----` and `service_test.rs | 44 ----`:
**the two tests of the row table were deleted, not moved.** That is a loss section 4's "lost that should not
have been: none identified yet" did not have: it should now read "the two `rows` case rows". The same diff
shows `row_in`, `reset`, `positions` and `cell` deleted from `service.rs`, leaving only `column_in` and
`push_row`.
The whole `values` crate falls from 30 `#[test]` on main to 18 at the tip; `compact_table` and `table_words`
have 0 at the tip (`git grep -c '#\[test\]'` over those three directories at `2a207464c` returns nothing).
The tip's executor has 49 `#[test]` in 21 files, and the two that are actually about a table's cells are
```
    fn reserved_cells_are_not_file_dictionary_offsets() {  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:181
    fn file_columns_match_owned_cells_in_the_executor_namespace() {  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:193
```
The second is the one test on either side that states the property `TableRead` needs: a lent column and an
owned column answer the same cell for every row. It must be carried, restated against `TableRead::cell` and
`column_into`.

### 6c. The proof that is really about tables, and lives elsewhere (main)
main's operators crate is the only place a table is tested as a table, and every one of those tests names
`ryo_values::domain::rows::types::Table`:
- 166 `#[test]` in the crate; 35 of them under `inbound/use_cases/run/use_case_test*` (the per-operator seam
  tests: `axis`, `closure`, `graphs`, `held_rows`, `kinds`, `ordered_match`, `relational`, `render`,
  `shared_index`, `streaming`, `text_map`, `tree_rows`, `fed`, `cross_kind`, `worklist`).
- 2 acceptance features, 2 scenarios each (`plan/acceptance.feature`, `run/acceptance.feature`).
- One allocation test, run alone in its process with `dhat` as the global allocator, whose single case is
  `        "allocation::operators::allocations_do_not_grow_with_rows_or_the_graph",`
  `@f4ae3bd802:crates/ryo/crates/operators/tests/allocation.rs:19`. **This is the test that proves the pooling
  the drafted `release` has to keep**, and its subject is the buffers that hold the tables:
  `    pub(crate) nodes: Vec<Option<ryo_values::domain::rows::types::Table>>,`
  `@f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/file_buffers/value.rs:33` and
  `    pub(crate) pool: KeptPool<ryo_values::domain::rows::types::Table>,`
  `@f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/file_buffers/value.rs:45`, with the
  caps `pub(crate) const KEPT_ROOM: usize = 1 << 12;`
  `@f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/constants.rs:7` and
  `pub(crate) const POOL_TABLES: usize = 256;`
  `@f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/constants.rs:15`.
- 88 files in the operators crate name `domain::rows::types` (by `git grep -l`, counted by crate: operators
  88, execution 31, derived 31, execution_domain 29, execution_runtime 24, core 18, values 8). None of those
  crates exists at the tip except `execution`, `execution_domain`, `core` and `values`.

### 6d. Carried, restated, missing

| Proof | Source | Fate |
|---|---|---|
| `the_cell_at_a_column_is_the_value_there` | `@f4ae3bd802:.../rows/service_test.rs:18` | **carried**, restated against `TableRead::cell` (no `Result`: the "refused" twin moves to `write`) |
| `a_column_past_the_row_is_refused` | `@f4ae3bd802:.../rows/service_test.rs:24` | **restated** as a `write(&Shape)`-time refusal (`TableError::NoSuchColumn`), since `cell` no longer returns a `Result` |
| `file_columns_match_owned_cells_in_the_executor_namespace` | `@2a207464c:.../finish/fact_view.rs:193` | **carried**, restated: a lent reader and an owned reader answer the same `cell` and the same `column_into` for every row |
| `reserved_cells_are_not_file_dictionary_offsets` | `@2a207464c:.../finish/fact_view.rs:181` | stays with the store (it is about the word namespace, not the table contract) |
| `allocations_do_not_grow_with_rows_or_the_graph` | `@f4ae3bd802:crates/ryo/crates/operators/tests/allocation.rs:19` | **restated** against `TablePort::release` + `write`: a port contract test (`fixtures/contract.rs`) every store adapter runs, with `dhat` alone in its process. Without it, gain 37 has no gate |
| the 35 operator seam tests | main `operators/.../run/use_case_test*` | belong to the operators sheet, but each must be rewritten generic over `S: TablePort` — they construct `Table { kinds, cells, count }` literals today |
| a `gather`/`append`/`matching` case table | — | **MISSING**. Three of the five hot calls are `@new` and nothing on either side tests them |
| a `TreeRead`/`TreeWrite` contract test | — | **MISSING**. The tip's `node_table` has tests nowhere in the 1,578-line service file (no `#[test]` in it) |
| an acceptance feature / scenario | — | **MISSING** on both sides for the row table; main's two operator features test the operator, not the table |
| a `compile_fail` doctest that the layout cannot be reached | — | **MISSING**, and it is the check that makes "no layout word" (section 9) hold by machine rather than by review. Precedent: `pub struct ShapeIndex(pub(super) ...)` is held that way (`compile_fail,E0616` at `@f4ae3bd802:crates/ryo/crates/language/src/domain/shapes/types.rs:13`) |

**What this means for the owner.** The table is the one concept every other sheet writes its interface
against, and it is the concept with the least proof of its own on either side: 2 case rows on main, 0 at the
tip. The 166 tests that do exercise tables are in a crate that was deleted (`operators` is absent at
`2a207464c`), written against a type the draft removes. So the contract's proof has to be built, not moved:
one port contract test (`fixtures/contract.rs`) that every store adapter runs, covering `cell`,
`column_into`, `matching`, `row`, `column`, `gather`, `append`, `finish`, `release`, plus the carried
allocation row. Section 11 lists this as open.

## 7. Perf gains it carries
From plan §10.3: row 22 (columns and indexes set up once per file version), row 37 (pools for columns, tables),
row 39 (native reads as columns). Each still needs its file list checked.

## 8. Benchmarks that cover it
Nothing benchmarks the table contract by name today. What exercises it is (a) three Rust benches that
construct a table type directly, (b) the four whole-run images whose instruction counts are the spike's and
the regroup's real gate, and (c) the per-language provider bench that fills columns. All three groups are
read-only facts here; the sheet-22 inventory (`contracts/benchmarks/sheet.md`) is the authority.

### 8a. What reaches a table type today, by name

| Benchmark | Where | What it names today | After the draft |
|---|---|---|---|
| operators `kernels` | `crates/ryo/crates/operators/benches/performance/kernels/bench.rs` (main only) | `use ryo_values::domain::rows::types::{RowCount, Table};` `@f4ae3bd802:crates/ryo/crates/operators/benches/performance/kernels/bench.rs:30`, building row tables in `fn defines(rows: usize, names: &[String], texts: &TextResources) -> Table {` `@f4ae3bd802:crates/ryo/crates/operators/benches/performance/kernels/bench.rs:39` | builds its rows through `TableWrite::row`/`column` on a handed-in store; the workload (selects keyed on one name column, a projection, a union, an antijoin, an extension, a join, a counted grouping, three probes, repeated-row drops, two text kinds) is unchanged |
| operators `graphs` | `crates/ryo/crates/operators/benches/performance/graphs/bench.rs` (main only) | the same import at `@f4ae3bd802:crates/ryo/crates/operators/benches/performance/graphs/bench.rs:21`, building edges in `fn edge_rows(pairs: impl Iterator<Item = (u32, u32, i64)>, texts: &TextResources) -> Table {` `@f4ae3bd802:crates/ryo/crates/operators/benches/performance/graphs/bench.rs:30` | the same, through `TableWrite` |
| python `fillers` | `crates/ryo/languages/python/benchmarks/provider/performance/fillers/bench.rs` (tip) | fills columns, not tables: `    self, CellWord, Column, Count, FactAnswer, FillExtent, FilledColumns, NodeTable, Relation,` `@2a207464c:crates/ryo/languages/python/benchmarks/provider/performance/fillers/bench.rs:13`, over `    let walk = ryo_python::walk();` `@2a207464c:crates/ryo/languages/python/benchmarks/provider/performance/fillers/bench.rs:271` | unchanged in what it measures (ns-per-element and growth-percent per relation, held by Asymptote); `NodeTable` becomes `impl TreeWrite`, `FilledColumns` becomes a `TableFill` sink. Note this is also the only caller of `walk()` outside a doctest, which is sheet 3's open item |

**Both operators benches are gone at the tip.** `crates/ryo/crates/operators` does not exist at
`2a207464c` (the crate list at the tip is `benchmark, benchmark_contracts, core, engine, execution,
execution_domain, executor, language, matching, query_language, relations, sdk, values`), and no bench under
`crates/ryo/crates` at the tip replaces them: the tip's benches are `benchmark/benches/performance{,/engine,
/evidence,/execution,/linter}`, `core/benches/performance{,/corpus,/scheduler}`,
`query_language/benches/performance.rs`, `sdk/benches/performance.rs` and the `executor/benches/lint_image`
image package. **So the two benches that measure the kernels over tables exist only on main, and nothing at
the tip measures a kernel over a table at all.** That is a finding for `perf-gains.tsv`: gains 42 (union and
antijoin kernels, Django 32.68 -> 24.82), 43 (two select kernels, 17.30 -> 16.73) and 30 (equijoin bound at
plan load) have no bench on the side they are live on.

### 8b. The images: what actually gates the draft
The spike's and the regroup's measurement is the whole-run image, not a divan bench. The image package is
`crates/ryo/crates/executor/benches/lint_image`, built with the release profile (`lto = "fat"`,
`codegen-units = 1`), three binaries `vec`, `vec_ts`, `vec_rust` over one entry
`pub fn run(` `@2a207464c:crates/ryo/crates/executor/benches/lint_image/main.rs:39`. A fourth image, `tm`, is
named beside them in the test that keeps their inputs honest:
`const IMAGES: [&str; 4] = ["tm", "vec", "vec_ts", "vec_rust"];`
`@2a207464c:crates/ryo/crates/executor/tests/image_sources.rs:5` — a test asserting the linked-image
dependency files hold the actual inputs of all local crates, which is itself a check the regroup must keep
passing as files move between crates.
These images produce the gate cells recorded in `build/ports/stage2/gates.tsv` (build 139 row, head
`2a207464c`): fastapi 2.20/2.19 CPU-s, dagster 20.47/19.89, django 9.16/8.89, pandas 10.47/10.93, tm one
worker 22.885/22.618, start-up 341.3/317.9 ms, peak 267.8/267.3 MiB (fastapi). The spike's bar is **+0.2%
instructions or less on fastapi and dagster** against the kept build-139 programs. The build-139 row also
records `pairs_quiet: NO except tm ten workers and start-up`, so the CPU cells of that row were not taken in
a quiet window; the spike needs its own quiet window.

### 8c. How each keeps measuring the same thing through the regroup

| Group | How it survives |
|---|---|
| the four images (`tm`, `vec`, `vec_ts`, `vec_rust`) | they call the SDK, not a table type: `ryo = { package = "idiomatic-sh-ryo-sdk", features = ["bench-hooks"] }`. Nothing in `api.txt` is named by them, so the draft cannot break them. They are the measure of "not lost" |
| `executor/tests/image_sources.rs` | must be updated as files move, and is the check that no new crate smuggles an input into an image |
| operators `kernels`, `graphs` | must be **restored from main** with the operators crate and rewritten generic over `S: TablePort`. Until then the kernels have no bench |
| python `fillers`, `manifest_reading` | unchanged measures; the type names in their imports change |
| `benchmark/benches/performance/{engine,evidence,execution,linter}` | measure ports of the benchmark crate (its engine, evidence, execution and linter adapters), not tables: untouched by this sheet |
| `core/benches/performance/{corpus,scheduler}` | the corpus and scheduler ports: untouched |
| `query_language/benches/performance.rs`, `sdk/benches/performance.rs` | both are empty divan mains at the tip (no `#[divan::bench]` registered): nothing to keep |
| the retention matrix `crates/ryo/benchmarks/retention/edit_matrix.py` | a process-level comparison of a retaining run against ordinary evaluation on isolated corpus copies; names no Rust type, so untouched |
| Asymptote ratchets | hold the python `fillers` measures to their best; they must keep the same measure names (`ns-per-element`, `ns-per-element-4n`, `growth-percent`) |

## 9. Checks
- IMPORTS: domain only (per the directions in ryo-contract-scope.md).
- Layout words in signatures: the TIP's `cols: Vec<Vec<u64>>` is a layout word. A drafted interface must not expose it.
- Crossing frequency: per set, not per cell (plan §3.1 rule). Unproven until the spike runs.

## 10. Decisions for the owner
1. **Table layout in the interface.** The TIP exposes `cols` publicly; MAIN exposes `cells` publicly.
   Proposal: neither. The store is handed in; the interface is logical only. Example: `executor/src/finish.rs:148`.
2. **Whether `table` is a crate in domain.** Plan §3.1 says yes, because operators (domain) cannot import the store.
3. **The spike.** The table contract is unproven until the spike passes (+0.2% instructions or less).

## 11. Not checked
Carried from the first draft, plus what this pass added. Nothing below is a claim; each line is a thing a
later pass must settle.

**Still not checked from the first draft**
- Implementers and callers of main's `Table` and the tip's `Table`. Partly narrowed this pass, not finished:
  by `git grep -l "domain::rows::types"` at `f4ae3bd802` the file counts by crate are operators 88,
  execution 31, derived 31, execution_domain 29, execution_runtime 24, core 18, values 8; at `2a207464c`
  only four files name `domain::rows` at all (`core/src/domain/finish/service.rs`,
  `execution_domain/src/domain/spelling/service.rs`, its `service_test.rs`, and
  `values/src/domain/table_words/types.rs`). The tip's executor table is touched on **206 lines in 35
  files**, all under `crates/ryo/crates/executor/src/` (`git grep -E "\.cols"` at `2a207464c`), the heaviest
  being `finish.rs` 75, `finish/select_kernel.rs` 16, `pair_summary/exact.rs` 10, `finish/ordered_dfs.rs` 9,
  `finish/antijoin_kernel.rs` 9, `finish/worklist.rs` 8. **The first draft's figure of "179 lines in 25
  files" (taken from the plan) does not match: the count at `2a207464c` is 206 lines in 35 files.** Neither
  number is a per-call count, and no list of which call each line makes has been built.
- Tests on both sides: now counted (section 6) but not read. 166 `#[test]` in main's operators, 49 in the
  tip's executor across 21 files, 30 -> 18 in `values`. Which of the 35 operator seam tests can be restated
  without an operators crate is not checked.
- Exact line numbers for `Ty` and `V` on the tip: **now checked and fixed.** `pub enum Ty {` is
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:101` (the draft said "line ~101") and
  `pub struct V(Ty2, u64);` is `@2a207464c:crates/ryo/crates/executor/src/finish.rs:123` (the draft said
  "~123"). Also exact now: `pub enum Ty2 {` at `:125`, `pub fn ty_of(k: CellKind) -> Ty {` at `:110`.
- The drafted interface (section 5): **now written**, and unverified in the ways listed below.
- The spike: not run. `domain/table/spike.md` does not exist. The three code paths the plan names for it all
  exist at the tip and were confirmed present this pass (`finish/view_cols.rs`, `finish/fact_view.rs`,
  `finish/antijoin_kernel.rs`, `finish/symbol_predicate.rs`, `finish/select_kernel.rs`), but the measurement
  script the plan cites, `build/one-engine/scripts/price.py`, is **not in the product repository at
  `2a207464c`** (`git ls-tree` finds nothing under `build/one-engine/scripts`): where the instruction-count
  tool lives is not checked.

**Added by this pass**
- `matching`, `gather` and `append` are `@new` with no live counterpart found. Whether the executor's index
  machinery already has an equivalent shape (and therefore whether these three are a rename rather than an
  invention) is not checked; I looked only at public signatures, not at the kernel bodies.
- `write(&Shape)` carries no row-count hint, but the tip's constructors do
  (`with_rows(..., n: usize)`, `retained(..., rows: usize)`, and `take_col`'s size classes). Whether dropping
  the hint from the signature costs the pooling gain (37) is **not measured** and is exactly what the spike
  must answer.
- `Shape` the drafted type collides with `ShapeIndex`/`ShapeClasses`/`ShapeKey` in `language`
  (`@f4ae3bd802:crates/ryo/crates/language/src/domain/shapes/types.rs:21`), a different concept (a file's
  expression-shape index). Whether the word is free is for the owner; not checked against the tastemaker
  naming rules.
- `TreeRead`/`TreeWrite` name 15 of the 42 `pub fn` in the tip's
  `crates/ryo/crates/language/src/domain/node_table/service.rs` (1,578 lines). The other 27 have no fate on
  this sheet, and four of them (`kind_codes`, `field_codes`, `parent_ids`, `starts`, `ends`) hand out raw
  `&[u16]`/`&[u32]` slices, which is layout. Whether the tree is this crate's second concept or a crate of
  its own is **not settled**, and `builder`/`finished` (opening and closing a build) have no drafted line.
- `node_table` does not exist on main, so the whole tree half of the draft rests on tip-only quotes. What
  main's equivalent is (`language/src/domain/elements/service.rs` has 36 `pub fn` including `kind_at`,
  `parent_at`, `children`, `end_at`, `element_count`, over `pub struct Elements` at
  `@f4ae3bd802:crates/ryo/crates/language/src/domain/elements/types.rs:114`) and whether `TreeRead` should be
  drafted from it instead is not checked.
- The counts in section 5g: the "26 public items" at the tip is a mechanical count of
  `^pub (struct|enum|fn|trait|type|const)` and `^    pub fn ` within `finish.rs:90-300`, a range I chose by
  reading. A different range gives a different number; nobody has agreed where the table concept ends in that
  4,388-line file.
- Section 4's "lost that should not have been: none identified yet" is now wrong: the two `rows` case rows
  (`the_cell_at_a_column_is_the_value_there`, `a_column_past_the_row_is_refused`) and four service functions
  (`row_in`, `reset`, `positions`, `cell`) were deleted between the revisions. Whether anything else of the
  concept was lost is not checked; I diffed only `values/src/domain/rows/`.
- Benchmarks: the two operators benches exist on main only, and nothing at the tip benchmarks a kernel over a
  table. Whether gains 42, 43 and 30 have any gate at the tip is not checked (it belongs in
  `perf-gains.tsv`). `query_language/benches/performance.rs` and `sdk/benches/performance.rs` register no
  `#[divan::bench]` at the tip; whether that is deliberate is not checked.
- The `tm` image is named in `executor/tests/image_sources.rs:5` but its binary is not in the `lint_image`
  package (which declares `vec`, `vec_ts`, `vec_rust`). Where `tm` is built is not checked.
- No check script has been run over this sheet or `api.txt`. `contracts/checks/` does not exist yet, so
  every `@rev:path:line` here was verified by hand with `git show` one line at a time; a machine pass
  (`check-quotes.py`) has not confirmed them, and `check-types.py` cannot run until a second `api.txt`
  exists.
- `IMPORTS: none` (section 9) is drafted, not verified: `check-graph.py` does not exist. The drafted types
  are all moved-in leaves, so the claim is plausible but unchecked.

