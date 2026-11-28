# outbound/store
STATUS: drafted (28 Nov 2026; not reviewed, not agreed). Every line that names a table type is UNPROVEN until
`domain/table/spike.md` records a pass; that file does not exist yet.

Revisions: MAIN = f4ae3bd802 (release 0.2.0, live). TIP = 2a207464c (perf branch, build 139). Read only with
`git -C /workspaces/idiomatic-public-benchmark show "REV:PATH"`. `X` below is `crates/ryo/crates/executor/src` at TIP.
Bead: ryo-fmx3.1.13.23.4.14.

## 1. Owns
The tables during a run: how their cells are laid out in memory, and where that memory comes from and goes back to.
(One thing: layout and memory are one concern here, because the pool is a pool of columns and only the layout knows
what a column is. It does not own what a table means (domain/table), what is kept between runs (outbound/cache), or
how much memory the process may use (decision 2).)

## 2. On main (f4ae3bd802)
**There is no store on main.** The owner decided on 27 Nov that "THE STORE provides the tables during a run"
(`build/ports/stage2/CRATES-AND-CONTRACTS.md` line 109); before that, the concept was spread over three crates:
- **The row table** `values::domain::rows::Table { kinds, cells: Vec<Value>, count }`: the table sheet's (section 2
  there); not repeated here.
- **The pool of tables and buffers**, inside operators: `operators/src/domain/kept_room` (521 lines with tests, 222
  without; 3 `#[test]` over 11 case rows; 0 public items, 19 `pub(crate)` items, 2 of them traits). Its two
  crate-internal contracts:
  ```
  pub(crate) trait KeptRoom {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/types.rs:14
  pub(crate) trait Poolable: Default {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/types.rs:79
  impl Poolable for Table {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/types.rs:89
  pub(crate) struct KeptPool<T> {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/types/kept_pool/value.rs:7
  pub(crate) const KEPT_ROOM: usize = 1 << 12;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/constants.rs:7
  pub(crate) const POOL_INDEXES: usize = 64;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/constants.rs:12
  pub(crate) const POOL_TABLES: usize = 256;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/constants.rs:15
  pub(crate) const POOL_TABLE_KINDS: usize = 64;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/constants.rs:18
  ```
  Implemented by `Vec`, `hashbrown::HashTable`, `VecDeque` (KeptRoom) and the row `Table` (Poolable). Held by the
  worker's buffers: `    pub(crate) pool: KeptPool<ryo_values::domain::rows::types::Table>,`
  `@f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/file_buffers/value.rs:45`, and the index
  pools `pub(crate) struct IndexPools {`
  `@f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types/shared_index/value.rs:15`. Callers: 20 files
  of operators name `kept_room`, `KeptPool`, `Poolable` or `KeptRoom` (by `git grep -l`); no other crate.
- **The tree of one file**: `language/src/domain/elements` (1,088 lines with tests, 2 `#[test]`):
  `pub struct Elements {`  `@f4ae3bd802:crates/ryo/crates/language/src/domain/elements/types.rs:114` ("its syntax tree
  in pre-order, column by column, 28 bytes for each element"). Decision 1 asks whether the tree is the store's.
- **A memory probe** `MemoryPort` (core, 412 lines with tests, 5 `#[test]`), which the plan's Appendix A 3.12 and
  section 8.2 list as store code: `pub trait MemoryPort: Send + Sync {`
  `@f4ae3bd802:crates/ryo/crates/core/src/outbound/memory/interface.rs:7`, two methods (`budget`, `resident_bytes`).
  On main it is called by core's file analysis, cross-file phase and storing services and composed by the command
  line: `    let memory: Arc<dyn MemoryPort> = Arc::new(memory::factory::sysinfo(budget));`
  `@f4ae3bd802:crates/ryo/cli/src/resources/query/run/composition.rs:62`. It measures the process, not a table
  (decision 2).

Proof on main: the 11 pool rows, for example
```
        name: CaseName("a_buffer_within_the_cap_keeps_its_room_emptied"),  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/service_test.rs:27
        name: CaseName("a_dropped_table_is_kept_emptied_for_the_next_node"),  @f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/types/kept_pool/value_test.rs:39
```
and two allocation tests, each alone in its process with `dhat`:
```
        "allocation::operators::allocations_do_not_grow_with_rows_or_the_graph",  @f4ae3bd802:crates/ryo/crates/operators/tests/allocation.rs:19
        "allocation::language::reading_allocates_as_the_parser_library_does",  @f4ae3bd802:crates/ryo/crates/language/tests/allocation.rs:16
```
Tastemaker entries on it: none by name; `kept_room` sat under `crate-operators` and `nu-hex-operators`
(`crates/ryo/tastemaker.toml` lines 116-117 at main), the tree under `crate-language` (line 130).

## 3. On the perf branch (2a207464c)
Everything is inside the proof-of-concept executor, plus the tree in the language crate. About 3,150 lines without
tests in the ranges below; 160 public items (counted by `^\s*pub (fn|struct|enum|type|const|trait|static)`):

| Piece | Where | Public items |
|---|---|---|
| the computed table, the column pool, the worker's share | `X/finish.rs:100-292` | 25 |
| one column's index (rows by key) and its kept buffers | `X/finish.rs:3784-3990` | 12 |
| list, answer and row-list pools | `X/finish/recycle.rs` (133 lines) | 9 |
| packed columns and blocks (a table that leaves a file) | `X/finish/population.rs:36-370` | 30 |
| lent (filled) columns read in place | `X/finish/fact_view.rs:1-175` | 18 |
| a file version's raw tree columns and tree indexes | `X/finish/file_version.rs:1-129` | 10 |
| the tree and its writer | `crates/ryo/crates/language/src/domain/node_table/{types,service,constants}.rs` (1,977 lines) | 54 |
| a language's fill sink | `crates/ryo/crates/language/src/domain/facts/types.rs:265-277` | 2 |

The contract actually called is no contract: public fields and free functions.
```
pub struct Table {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:148
    pub cols: Vec<Vec<u64>>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:151
    static POOL: RefCell<([Vec<Vec<u64>>; CLASSES], usize)> = const { RefCell::new(([const { Vec::new() }; CLASSES], 0)) };  @2a207464c:crates/ryo/crates/executor/src/finish.rs:159
pub const POOL_CELLS: usize = 1 << 22;  @2a207464c:crates/ryo/crates/executor/src/finish.rs:163
    static POOL_LIMIT: std::cell::Cell<usize> = const { std::cell::Cell::new(POOL_CELLS) };  @2a207464c:crates/ryo/crates/executor/src/finish.rs:167
pub struct PoolBudget(usize);  @2a207464c:crates/ryo/crates/executor/src/finish.rs:172
    pub fn for_workers(workers: usize) -> Self {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:175
impl Drop for PoolBudget {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:180
pub const CLASSES: usize = 20;  @2a207464c:crates/ryo/crates/executor/src/finish.rs:186
pub fn take_col(p: &mut ([Vec<Vec<u64>>; CLASSES], usize), n: usize) -> Vec<u64> {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:192
impl Drop for Table {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:209
    pub fn retained(names: Names, tys: Tys, rows: usize) -> Self {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:218
        let cols = (0..names.len()).map(|_| Vec::with_capacity(rows)).collect();  @2a207464c:crates/ryo/crates/executor/src/finish.rs:219
    pub fn with_rows(names: impl Into<Names>, tys: impl Into<Tys>, n: usize) -> Self {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:227
    pub fn copy(&self) -> Table {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:239
    pub fn release(&mut self) {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:258
                if cap == 0 || cap > POOL_CELLS / 16 || *cells + cap > POOL_LIMIT.with(std::cell::Cell::get) {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:267
pub struct Index {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:3787
pub const IX_CELLS: usize = 1 << 20;  @2a207464c:crates/ryo/crates/executor/src/finish.rs:3801
    pub fn build_column(n: usize, column: fact_view::Column<'_>) -> Index {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:3847
pub fn table_index(f: &mut File<'_>, tn: usize, t: &Table, cols: &[usize]) -> Rc<Index> {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:3936
pub const KEEP: usize = 1 << 14;  @2a207464c:crates/ryo/crates/executor/src/finish/recycle.rs:12
pub const ROW_CELLS: usize = 1 << 20;  @2a207464c:crates/ryo/crates/executor/src/finish/recycle.rs:22
pub fn rows_with(fill: impl FnOnce(&mut Vec<u32>)) -> Rc<Vec<u32>> {  @2a207464c:crates/ryo/crates/executor/src/finish/recycle.rs:26
pub fn take_cols(k: usize) -> Vec<Vec<u64>> {  @2a207464c:crates/ryo/crates/executor/src/finish/recycle.rs:76
pub fn rc(t: Table) -> Rc<Table> {  @2a207464c:crates/ryo/crates/executor/src/finish/recycle.rs:104
pub fn recycle(mut r: Rc<Table>) {  @2a207464c:crates/ryo/crates/executor/src/finish/recycle.rs:123
/// All representations decode to the exact canonical u64, including public nulls.  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:38
pub enum Words {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:40
pub type BlockBytes = Arc<dyn AsRef<[u8]> + Send + Sync>;  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:52
    pub fn pack(v: std::borrow::Cow<'_, [u64]>) -> Self {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:104
pub struct ColumnBlock<C> {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:158
pub type WorkerBlock = ColumnBlock<Arc<[Words]>>;  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:169
pub enum Column<'a> {  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:21
    Words(&'a [CellWord], &'a [u32], &'a rustc_hash::FxHashMap<u32,u32>, u32),  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:25
    pub fn cell(self, column: usize, row: usize) -> u64 {  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:82
pub fn cached<'a>(cache: &'a ColumnCache, column: usize, lent: Column<'a>) -> Column<'a> {  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:133
pub enum Fact<'a> {  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:150
pub struct FileVersion<'a> {  @2a207464c:crates/ryo/crates/executor/src/finish/file_version.rs:6
    pub kind: &'a [u16],  @2a207464c:crates/ryo/crates/executor/src/finish/file_version.rs:13
    pub by_start: RefCell<Option<Index>>,  @2a207464c:crates/ryo/crates/executor/src/finish/file_version.rs:20
pub struct NodeTable(pub(crate) NodeTableEngine);  @2a207464c:crates/ryo/crates/language/src/domain/node_table/types.rs:31
pub struct NodeTableBuilder<'a>(pub(crate) BuilderState<'a>);  @2a207464c:crates/ryo/crates/language/src/domain/node_table/types.rs:287
pub fn builder(table: &mut NodeTable, kinds: Count) -> NodeTableBuilder<'_> {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:43
pub fn finished<'b>(builder: &'b mut NodeTableBuilder<'_>) -> &'b NodeTable {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service.rs:862
pub struct FilledColumns(pub Vec<FilledColumn>);  @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:265
pub struct FilledColumn(pub Vec<CellWord>);  @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:277
```
The worker's share is set inside the executor's file job:
`            let _budget = finish::PoolBudget::for_workers(shards.len());`
`@2a207464c:crates/ryo/crates/executor/src/check_session/worker.rs:194`. A table's columns go back to the pool
**of the thread that drops it** (`Drop for Table`), so a table is released on the worker that made it. Two kinds of
allocation exist: `with`/`with_rows` take pooled columns by size class; `retained` allocates columns that are not
taken from the pool (used for tables that outlive the file: 7 call sites in `check_session/frontier.rs`,
`finish/population.rs`, `finish/session/{contribution,demand}.rs`, `finish/subject_scope.rs`, `reader_run/packet.rs`).

Callers: the computed table's `.cols` on 206 lines in 35 files of the executor (table sheet, re-counted there);
`recycle::` in 9 executor files; `PoolBudget` in 1 (`worker.rs`); `NodeTable`/`NodeTableBuilder` in 23 files of the
language crate, 14 of the executor, 9 of relations, 4 of the SDK and 100 across the nine languages (python 24, json
12, markdown 12, css 11, html 11, go 8, toml 8, typescript 8, rust 6). The only live memory-probe caller is the SDK:
`    let port = ryo_core::outbound::memory::factory::sysinfo(budgeted(memory));`
`@2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/rows.rs:33`.

Proof at the tip: **no test of the pool** (none in `finish.rs:100-292`, none in `recycle.rs`; no `tests/allocation.rs`
anywhere in the Ryo crates at the tip). What exists: the two `fact_view` tests (the table sheet's), the tree's one
`#[test]` over 11 case rows,
```
        name: CaseName("nodes_take_the_first_ids_in_pre_order_then_tokens_in_source_order"),  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service_test.rs:91
fn succeeds() {  @2a207464c:crates/ryo/crates/language/src/domain/node_table/service_test.rs:331
```
(the table sheet says the tree has "tests nowhere"; it has these, in `service_test.rs`), and one tree-index test,
`    fn parent_probe_includes_each_attached_root_and_concrete_child() {`
`@2a207464c:crates/ryo/crates/executor/src/finish/file_version.rs:132`. Tastemaker: the executor under
`crate-executor` (`crates/ryo/tastemaker.toml` line 105 at the tip), the tree under `crate-language` (line 120).

## 4. What changed between them
| Change | Commit (date) | Effect |
|---|---|---|
| the column pool, by size class | `712f78c3bd` (11-22) "the column pool by size class, columns taken by the room a table needs" | part of perf row 37 |
| answers, lists, file buffers and indexes reused (recycle.rs) | `e0d0dbb81b` (11-22) | part of row 37 |
| the pool's room back to 32 MB | `7ba74b8bb1` (11-22) | row 37: jemalloc -14 to -17%, second-half allocations -67 to -73% |
| the tree as a node table in the language contract | `3097109506` (11-20) | the tree's layout moved from `Elements` to `NodeTable` |
| child ids placed by parent and ordinal | `fd83be7ee0` (11-23) | row 33: first half django 1.826 -> 0.703 |
| provider columns borrowed, read in place | `9ab067a702` (11-23) | row 25 (with `24665d03e4`, `61371d6edf`) |
| a version's node columns, text and indexes lent to each program | `7b201b7869` (11-23) | row 22's table half |
| packed blocks; typed plans on shared populations | `a28abaab18` (11-24) | row 18: peak 1,845.1 -> 1,510.0 MiB |
| the column budget divided among the active file workers | `2bb45169ab` (11-25) | row 54 |
| executor moved into `crates/ryo/crates/executor` | `f2869d1f74` (11-26) | renames only |
| operators deleted, `kept_room` with it | `fc94354b0c` (11-26) | main's pool contracts and 11 rows gone |

**Lost that should not have been:** main's two crate-internal pool contracts (`KeptRoom`, `Poolable`) and their 11
case rows; every allocation test (main had them in operators, core, language, `execution_runtime` and the nine
languages; the tip has none), so perf row 37 has no gate at the tip. The tip's pool replaced main's with a better
measured policy (size classes, a total-cell budget per worker) but with no test of its own.

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
`api.txt` beside this sheet: **6 types and 3 inherent functions a caller names** (`Store`, `Store::for_workers`,
`WorkerTable`, `WorkerReader`, `WorkerWriter`, `FileTree`, `FileTree::writer`, `FileTreeWriter`,
`FileTreeWriter::finished`), plus 9 `impl` lines and 33 method lines that are the table listing's traits
implemented (51 lines before `# NO LONGER PUBLIC`). On main: 0 public (19 crate-internal). On the tip: 160.

**The claim it starts from.** Appendix A 3.12 (`PLAN.md:1349-1359`): `impl TablePort for Store`, TreeRead/TreeWrite
for its node table, and `pub fn store(budget: MemoryBudget, workers: WorkerCount) -> Store`, "one per worker".
Checked against the code and corrected in three places: (1) there is no memory budget in the pool today: its total
is the constant `POOL_CELLS` (32 MB, tuned: "c8 tried 64 MB: no fewer misses, 37 MB more live", finish.rs:162), and
the only input is the worker count, so the constructor takes `WorkerCount` alone; (2) the store is made **on the
worker's thread** (its pools are the thread's), so the edge cannot build it and hand it over: the edge hands
`execute` a maker (`|| Store::for_workers(count)`, a plain closure, no new trait; the execute sheet places it) and
each worker calls it; (3) the plan names "`MemoryPort`" as store code: it is not (decision 2).

**What the store offers, line by line.**
- `Store` + `for_workers(WorkerCount)`: today's `PoolBudget::for_workers` RAII, made the store's handle. Dropping the
  store restores the thread's share as `Drop for PoolBudget` does. The pools stay **thread-local inside the store**
  in the first build (decision 3): that is the measured policy, kept unchanged, and the only place that knows it.
- `impl TablePort for Store`: `write(&Shape)` folds the tip's three constructors; the store chooses pooled columns
  (`with_rows`) or plain ones (`retained`) itself. The row-count hint the tip passes (`with_rows(.., n)`) has no
  place in the drafted `write`: the table sheet lists it as not checked; the store needs it for the size class
  (decision 4 (b), via the prepared shape). `release` is `Table::release` + `recycle::recycle`; dropping a
  `WorkerTable` without `release` still returns its columns (the tip's `Drop for Table`), so a caller that forgets
  costs nothing but the answer allocation's reuse.
- `WorkerTable`: the tip's `Table` with every field private. Not `Send`. Not `Clone` in the draft; today a step's
  table is shared as `Rc<Table>` by the caller and its allocation recycled by `recycle::rc`; decision 4 (e) asks the
  table listing whether `TablePort::Table` should be a cheap shared handle so that sharing and its recycling stay in
  the store.
- `WorkerReader<'t>`: **one reader for a computed table and for a filled one**, as the tip's `Fact` already is
  (`Owned` | `Lent`). Inside it, and private: the lent column cache (a lent column read about as often as it has
  rows is translated once, `fact_view.rs:133`) and one column's index for `matching`, built on first use and kept
  for the table's life (the tip's `Index::build_column` / `File::fact_list`; perf row 22 "columns and indexes set up
  once per file version"). `matching` is the call that makes the index the store's: today the index is reached as
  `Rc<Index>` with public fields on 18 lines naming `Index::build`, `table_index` or `fact_list` (two of them the definitions).
- `WorkerWriter`: `TableWrite` for computed tables, and **`TableFill` for a language's fill**. That replaces the
  language's `FilledColumns(pub Vec<FilledColumn>)`: `FactSource::fill(.., out: &mut FilledColumns)` becomes a fill
  into the store's writer (the language sheet decides `&mut dyn TableFill` or generic), and `finish()` gives a
  `WorkerTable` that `read` lends in place. This is how perf row 25 ("provider columns lent without a copy") is kept:
  the filled columns are the store's own from the start, so there is nothing to borrow and nothing to copy.
- `FileTree`, `FileTreeWriter` (decision 1): the language's `NodeTable` and `NodeTableBuilder`, moved, implementing
  the table listing's `TreeRead` and `TreeWrite`. `writer` and `finished` are the tip's `builder` and `finished`;
  they are inherent here because the table listing has no line to open or close a tree (decision 4 (c)). The tree's
  raw columns (`kind_codes`, `starts`, `ends`, `parent_ids`, `field_codes`, read by `FileVersion` today) and its
  indexes (`FileVersion.by_start`, `by_end`, `by_line`) go inside `FileTree`; the table listing's `TreeRead` does not
  yet answer "the elements starting at an offset" or "on a line", which `FileVersion` serves today (section 11).

**The relations sheet's question (typed cells).** Five crates encode a typed cell through `to_bits` / `from_bits`
(`pub fn to_bits(value: Value) -> Bits {`  `@2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:253`,
`pub fn from_bits(kind: CellKind, bits: Bits) -> Value {`
`@2a207464c:crates/ryo/crates/relations/src/domain/relations/service.rs:267`; callers in language, python, rust,
typescript and values), and the table listing makes `Value` non-public. **The store offers no typed cell.** It takes
and gives `CellWord`s; a column's `CellKind` is in the `Shape`. How a node, a file, a span, a number or a flag is
written as a word is the same for every layout (a row store and a column store write the same word), so it is
logical, not layout, and belongs beside `CellKind` and `CellWord` in domain/table (as `From`/`TryFrom` conversions),
which is the relations sheet's own pick. The store's one promise about words is the tip's: it never re-encodes; a
packed column decodes to the exact word written (`population.rs:38`, quoted in section 3). Likewise
`ryo_relations::Words(pub Vec<u64>)` ("values written as numbers so they can leave a file", which the relations
sheet proposes for this crate) has no caller at the tip (`git grep` finds only its definition); its live concept is
the store's packed form, which stays private (decision 4 (d)). No public line here.

**What the store does not take, and where it goes** (each named in api.txt's `# NO LONGER PUBLIC`): the row-number
lists' pool (`rows_with`, `recycle_rows`, `no_rows`): kernel scratch, operators' `FileBuffers`; a join's hash over
several columns (`Index::build` with `key_of`, `table_index(.., cols: &[usize])`): operators' scratch until
`matching` takes several columns; `inline_key` (a grouping key): operators; `FileBufs` and the thread-local
`SPARE_BUFS` (per-file maps kept for the next file): operators' `FileBuffers`, which the operators sheet already
gives the worker's store (`FileBuffers::new(store)`); `Population`, `CarriedBlock`, `CarriedSchemas`: project; the
version's source bytes, file number and text caches in `FileVersion`: execute.

**The two changes the spike may force** (`domain/table/scope-table-spike.md` 2.3), and what the store does for each.
Both are cheap here, because the store already has the parts:
- **D1, a column cursor** (`ColumnRead::at(row)`, `TableRead::column(c)`): the store's cursor is the tip's
  `fact_view::Column<'a>` (`Owned | Numbers | Nodes | Words`), whose representation is matched once per column and
  then read per row; for a packed column it is `population::Words::at`. Lines: `WorkerColumn<'c>`, its `at`, and
  `WorkerReader::column` (api.txt, conditional section). Nothing is handed out but the cursor.
- **D2, a shape prepared once per step** (`TablePort::Shape`, `prepare`, `write(&Self::Shape)`): the store's prepared
  shape is the tip's `(Names, Tys)` pair, `Rc`-shared with the step's schema: "`never copied per table`"
  (`/// 7: a table's column names and types, shared with its step's schema (never copied per table).`
  `@2a207464c:crates/ryo/crates/executor/src/finish.rs:142`). Line: `PreparedShape` (conditional section). The
  prepared shape is also where the row-count hint and the pooled/plain choice belong (decision 4 (b)), so I recommend
  D2 whatever the count says: without it, `write(&Shape)` copies the column names into every table, which the tip
  stopped doing.

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| 4 room rows (`KeptRoom`) | `@f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/service_test.rs:27` and siblings | **restated** against the store's crate-internal `KeptRoom` (the tip's caps: a column past `POOL_CELLS / 16` or past the worker's share is dropped) |
| 7 pool rows (`KeptPool`, `Poolable`) | `@f4ae3bd802:crates/ryo/crates/operators/src/domain/kept_room/types/kept_pool/value_test.rs:39` and siblings | **restated** over the size-class pool: a released table's columns are taken by the next table of the same class; a column too big, or past the share, is freed; the pool keeps no more than its total |
| allocation: no growth with rows or the graph | `@f4ae3bd802:crates/ryo/crates/operators/tests/allocation.rs:19` | **restated** as the store's port contract test (`fixtures/contract.rs`, alone in its process with `dhat`): a steady file loop of `write`/`gather`/`release` allocates nothing after the first file. It is the gate of perf row 37; the operators sheet restates its own over `FileBuffers<S>` |
| allocation: reading allocates as the parser library does | `@f4ae3bd802:crates/ryo/crates/language/tests/allocation.rs:16` | **restated** against `FileTree` / `FileTreeWriter` if decision 1 is taken (the tree is judged, the parser's own allocations cancel) |
| the tree's 11 case rows | `@2a207464c:crates/ryo/crates/language/src/domain/node_table/service_test.rs:91` and siblings | **carried** with the tree (decision 1) |
| the tree-index test | `@2a207464c:crates/ryo/crates/executor/src/finish/file_version.rs:132` | **carried** into `FileTree` (the parent probe is an index of the tree) |
| a lent and an owned column answer the same cells | `X/finish/fact_view.rs:193` | the table sheet's contract case; the store runs it (filled `WorkerTable` against a computed one) |
| reserved cells are not file dictionary offsets | `X/finish/fact_view.rs:181` | the store's **only if** the translation of a language's words stays in the reader (decision 5); otherwise it goes with the words |
| the per-worker share (`for_workers`) | none on either side | **MISSING**: two stores on two threads each keep at most `total / workers`; dropping the store restores the share |
| packing round trip | none on either side | **MISSING**: every packed form (`Constant`, `Sequence`, `Byte`, `Short`, `Int`, `Full`, `Stored`) reads back the exact word written, nulls included |
| `Store`, `WorkerTable`, readers and writers are not `Send` | none | **MISSING**: a `compile_fail` doctest (precedent: `NodeTable`'s `compile_fail,E0603` at node_table/types.rs:16) |
| the layout cannot be reached | none | **MISSING**: a `compile_fail` doctest on a private field of `WorkerTable` |

## 7. Perf gains it carries
Every row of `perf-gains.tsv` whose crate column names `outbound/store` (6 rows, all live):

| Row | Gain (measured, from the tsv) | What of it is here | What in the draft keeps it |
|---|---|---|---|
| 18 | typed admission and packed answer rows: peak 1,845.1 -> 1,510.0 MiB, retained answer rows 329,091,157 -> 58,324,039 bytes | the packing: `population::Words::pack` (constant, sequence, byte, short, int widths) and the blocks that carry a file's tables out | the packed form stays inside the store, private; it needs a public way in: decision 4 (d), a sendable packed table on `TablePort`. The admission and answer-layout half is execute's |
| 22 | columns and indexes set up once per file version: set-ups 557,238 -> 24,973 (-5.645 CPU-s) | one column's index, kept with the table; the tree's raw columns and indexes (`FileVersion`) | `matching` builds a column's index once and keeps it inside the reader's table; the tree's indexes live inside `FileTree`, which is one per file version and read by every program. Holds only if execute makes one reader per file version, not per program (execute sheet). The symbol-test half is operators' and symbols' |
| 25 | provider columns borrowed: start-up django 166.1 -> 77.6 ms; fastapi 0.97 -> 0.86 CPU-s | the filled columns read in place | the language fills the store's `WorkerWriter` (`TableFill`); `finish()` gives a `WorkerTable` read by `WorkerReader` with no copy. The typed-plan binding half is programs' |
| 33 | child ids placed by parent: first half django 1.826 -> 0.703, dagster 3.878 -> 1.560 | the tree writer's one-pass placement (`node_table/service.rs`) | `FileTreeWriter` keeps `enter_node`/`leave_node`/`token` in one pass (decision 1). The deferred model and fact columns are python's |
| 37 | pools for columns, tables and per-file buffers: jemalloc -14 to -17%, second-half allocations -67 to -73% | the column pool by size class, its 32 MB total and 1/16 cap, the column-list and answer pools, the index buffers | `release` (and drop) return to the worker's pools; `Store` is per worker and its tables are not `Send`, so a table goes back to the worker that made it; the pool's policy is private (decision 3). The allocator stays the binaries' choice (`#[global_allocator]` in the command line, the benchmark crate, Tastemaker's command line and the lint images); per-file buffers are operators' `FileBuffers` |
| 54 | per-worker budget `PoolBudget::for_workers`: dagster peak 1,257.5 -> 480.1 MiB (joined with row 7) | the share of the pool per active worker | `Store::for_workers(WorkerCount)`: the scheduler's count, given at construction |

Room it must keep for gains not yet had: the many-workers prototype (row 13, parked) froze plan, constants and symbol
tables once; nothing of the store was shared there (pools stay per worker), so the draft needs nothing more for it.

**Which past changes would have landed inside this crate alone under the draft.** `712f78c3bd` and `7ba74b8bb1`
(size classes; the pool's room): pool policy only, inside the store. `e0d0dbb81b` (lists, answers, indexes reused):
the table and index parts inside the store; the per-file buffers part in operators. `2bb45169ab` (budget per worker):
inside the store once the edge already passes `WorkerCount` (today it also touched `worker.rs`). The packing of
`a28abaab18`: inside the store. **Would still cross:** `fd83be7ee0` (row 33) changed the python walker too
(`ruff_python_parser/walker.rs`, `types.rs`, `walk.rs`, `embedded.rs`); `9ab067a702` (row 25) needs the language's
fill and the programs' binding; row 22's symbol half is operators' and symbols'; row 39's word translation crosses
store, symbols and execute unless decision 5 removes it.

## 8. Benchmarks that cover it
| Benchmark | What it calls in this crate's code today | After the draft |
|---|---|---|
| the lint images `vec`, `vec_ts`, `vec_rust` (`executor/benches/lint_image`) and `tm`: the Python Ruff, TypeScript Oxlint and Rust Clippy runs and the Tastemaker check, with their case suites and parity | everything here runs on every file (pools, tables, readers, the tree); they name nothing of it: they call the SDK (`idiomatic-sh-ryo-sdk`, feature `bench-hooks`) | unchanged in what they measure; the SDK composes the store (`Store::for_workers` per worker, through lint and execute). They are the measure of "not lost": peak memory (rows 18, 37, 54) and CPU (rows 22, 25, 33, 37) against the kept build-139 images |
| the `benchmark` crate (`tools/benchmark` after the move) and `benchmark-collector` | no store name (`git grep` for `finish::Table`, `PoolBudget`, `recycle::`, `NodeTable`, `node_table::` finds none) | unchanged |
| python `fillers` (`crates/ryo/languages/python/benchmarks/provider/performance/fillers/bench.rs`; per relation `ns-per-element`, `growth-percent`, held by Asymptote) | `NodeTable::default()`, `FilledColumns::default()`, `node_table::element_count` through the SDK's `ryo::languages` re-export (bench.rs:13, :54, :57, :79) | `FileTree::default()`, the store's writer as the fill sink, `TreeRead::element_count`: the same measure; the bench takes the store as a dev-dependency (or the SDK re-exports it: the SDK sheet decides) |
| operators `kernels`, `graphs` (main only; to be restored by the operators sheet) | build row tables directly (`ryo_values::domain::rows::types::Table`) | build their inputs through `ryo_store::Store` (a dev-dependency of operators; the library's import direction is unchanged), so they time the real layout |
| a store bench | none on either side | not proposed; the images and the spike's instruction counts are the measure |

## 9. Checks
IMPORTS: table, scheduler (and symbols only if decision 5 keeps the word translation in the reader)
CROSSED: per cell
- **Directions.** outbound imports domain and outbound: `table` (the traits it implements and their types) and
  `scheduler` (`WorkerCount`, `pub struct WorkerCount(pub u32);`
  `@2a207464c:crates/ryo/crates/core/src/outbound/scheduler/types.rs:14`, the scheduler's word for the count; the
  alternative, a store-owned count, would define the concept twice). Not `language`: the language fills through the
  table listing's `TableFill` and writes the tree through `TreeWrite`, so neither imports the other. If decision 1
  moves the tree here, the tree's diagnostics, decoded properties, acceptance and embedded segments (today language
  and relations types, imported by `node_table`) must first have lines on the table sheet, or the store would import
  `language`. Outside crates: `rustc-hash` (the index's map, as today); nothing else.
- **No layout word in a signature.** None: the concrete types are opaque, every field private; no slice of words,
  `Vec<Vec<_>>`, `Rc` or pool appears. The five tree functions that hand out raw columns today (`kind_codes`,
  `field_codes`, `parent_ids`, `starts`, `ends`) become private.
- **How often it is crossed.** `for_workers` once per worker; `write`, `finish`, `release`, `gather`, `append`,
  `column_into`, `matching` once per step's table (per set); `cell` and the tree reads (`kind_of`, `parent_of`, ...)
  **per cell**, as today's `Fact::cell` and `node_table::kind_of` are; `enter_node`/`token` per tree element. Every
  caller is generic over the store's type (`S: TablePort`, `impl TreeRead`), so each per-cell call is a static,
  inlinable call across the crate under the release profile (`lto = "fat"`, `codegen-units = 1`), never `dyn`. The
  spike measures exactly this cost; D1 is the fix if it is too high. The language's fill sink is the one place the
  plan allows `dyn` (`TableFill`), crossed per row or per column of a fill, as `FilledColumns` is pushed today.
- **Generics, not `dyn`.** No `dyn` in the listing. Not `Send`: by design (a table is released on its worker).
- **Review criteria.** 1 (a pool or layout change touches this crate only: today it touches `finish.rs`, `recycle.rs`
  and 35 files reading `.cols`); 2 (one table type, one reader for computed and filled tables, one fill sink instead
  of `FilledColumns` beside `Table`); 3 (opaque types, private fields); 6 (one definition of a table, of a column's
  index, of the tree); 19 (no trait of its own; 9 caller-named items).

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
**1. Is the tree of a file the store's?**
Example: `NodeTable` lives in the language contract (`node_table/types.rs:31`), but the executor reads its raw columns
(`FileVersion { kind: &[u16], start, end, parent, field }`, file_version.rs:13-17) and builds indexes over them
(`by_start`, `by_end`, `by_line`); the table sheet puts `TreeRead`/`TreeWrite` in domain and leaves "whether the tree
is a second concept" open; the trunk reader found "two homes (outbound/language vs outbound/store)".
Why: if the tree stays with the language, the store is not "the one place a layout exists" and the tree's indexes
(row 22) and one-pass writer (row 33) sit outside it. If it moves, the tree's extra content (diagnostics, decoded
properties, acceptance, embedded segments: 20 of the 42 functions without a drafted line) needs words in domain/table
first.
What we get with A (move): every layout in one crate; a later layout change (one arena for tree and tables) lands in
one crate; the nine languages write through `impl TreeWrite` either way.
**My pick:** A, the tree moves here as `FileTree`/`FileTreeWriter`, after the table sheet gives the 20 remaining
functions their lines. If the table sheet keeps the tree as a separate concept, B: the tree stays in
outbound/language behind the same traits, and this listing loses its tree section (21 lines).

**2. `MemoryPort` is not the store's.**
Example: `pub trait MemoryPort: Send + Sync {`  `@2a207464c:crates/ryo/crates/core/src/outbound/memory/interface.rs:7`
answers "how much memory may the run use" and "how much does the process hold"; at the tip its only caller is the
SDK's `query run` (rows.rs:33), checking the process before and after a request. The pool's limit is a constant
(`POOL_CELLS`) shared among workers, not read from it.
Why: Appendix A 3.12 and plan 8.2 list it as store code; the architecture review (B10) and the workshop log (row 11,
`CRATES-AND-CONTRACTS.md` line 34) place it in the lint use case.
What we get: the store owns one thing; the memory probe stays a port of the crate that enforces the process budget.
**My pick:** `MemoryPort` goes to inbound/lint as its own outbound port (with its sysinfo adapter); the lint sheet
carries it. Nothing of it in this listing.

**3. Where the pools live, and what `release` means.**
Example: the tip keeps them thread-local (`static POOL`, finish.rs:159) and returns columns on `Drop for Table`;
main kept them in the worker's buffers (`FileBuffers.pool: KeptPool<Table>`) and gave them back explicitly.
Why: thread-local pools are the measured policy (row 37) and need no handle in any call; pools inside the `Store`
value are explicit but make every table either carry a handle back (a pointer per table) or be released by hand.
What we get with thread-local inside the store: no change to the measured code; the `Store` value is the share
(today's `PoolBudget`); callers may call `release` or drop, and the contract test pins both. A later move to pools in
the value lands inside this crate only, since callers already go through `release`.
**My pick:** thread-local inside the store for the first build; `Store` made on the worker's thread; main's
`KeptRoom` and `Poolable` kept as the store's crate-internal contracts, restated over the size classes.

**4. What the table listing must add for the store to keep its gains** (lines for the table sheet; none is in this
listing).
Example and line for each: (a) **D1** column cursor, if the spike asks: `fact_view::Column::at` (fact_view.rs:30)
is today's cursor. (b) **D2** prepared shape, always: finish.rs:142 "never copied per table", and the room hint the
tip passes to `with_rows(.., n)` and the pooled/plain choice (`retained`) belong on it. (c) **open and close a
tree** on `TablePort` (`type Tree: TreeRead`, `type TreeWriter<'t>: TreeWrite`, `fn tree(&mut self, kinds: Count)`,
`finished`): today's `builder`/`finished`; without them a generic caller cannot make a tree. (d) **a sendable packed
table** (`type Packed: Send + Sync`, `fn pack(&mut self, Self::Table) -> Self::Packed`, and a reader over it):
row 18's packed blocks and project's shared populations (`WorkerBlock = ColumnBlock<Arc<[Words]>>`, population.rs:169)
need it, since a `WorkerTable` never leaves its worker. (e) **`TablePort::Table: Clone` as a shared handle**: callers
hold `Rc<Table>` today and the store recycles that allocation (`recycle::rc`, recycle.rs:104); with the bound, the
sharing and its recycling stay in the store. (f) **the width of `RowNumber`**: the store's row lists and indexes
are `u32` (`Index.rows: Vec<u32>`); the table listing's `RowNumber(pub usize)` doubles them (spike finding F2).
Why: each is a live property of the tip that the drafted `TablePort` cannot express.
What we get: the six store gains keep their mechanism behind the port.
**My pick:** add (b), (c), (d), (e) and `RowNumber(pub u32)` to the table listing now; add (a) if the spike asks.

**5. Who translates a language's words into the run's.**
Example: a filled column holds the file's words, and the reader translates them on every read
(`    Words(&'a [CellWord], &'a [u32], &'a rustc_hash::FxHashMap<u32,u32>, u32),`
`@2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:25`), with a cache once a column is read often
(`cached`, fact_view.rs:133); node words drop a file part (`value as u32`, fact_view.rs:36).
Why: if the translation stays, `read` needs a translation handed in (no slot for it in `TablePort::read`) and the
store imports `symbols`; the symbols sheet's decision 3 (a language interns into a layer of the run's dictionary)
removes it.
What we get with symbols decision 3: a filled table is a plain `WorkerTable`, read with no translation and no cache;
`IMPORTS` stays `table, scheduler`.
**My pick:** take the symbols sheet's decision 3 and drop the translation from the store; otherwise the table listing
gains `TablePort::lend(&self, &Self::Table, &Translation) -> Self::Reader<'_>` and this crate imports `symbols`.

## 11. Not checked
- The spike: not run (`domain/table/spike.md` absent). Every `impl ryo_table::*` line is UNPROVEN; whether per-cell
  `cell` through the reader costs more than +0.2% instructions is exactly its question.
- Whether pools inside the `Store` value (decision 3's alternative) cost anything against the thread-local pools: not
  measured.
- The row-count hint: whether `write` without it (draft A) loses row 37's size-class hits: not measured.
- `matching` versus the tip's index callers: I counted 18 lines naming `Index::build`, `table_index` or
  `fact_list` but did not read each to see how many key on one column (which `matching` serves) and how many on
  several (operators' scratch).
- The tree: the 20 `node_table` functions without a drafted line (`ordinal_of`, `decoded_of`, `property_of`,
  `implied_of`, `acceptance_of`, `syntax_error_count`, `own_end`, `is_token`, `embedded_segments`, `classify_fields`,
  `embed`, `enter_implied_node`, `enter_unlocated_node`, `decoded_token`, `diagnose_with_metadata`, ...) were not read;
  nor whether `TreeRead` needs "elements starting at an offset" and "on a line" (`FileVersion.by_start`, `by_line`).
- The 160 public items at the tip are a count over the line ranges chosen in section 3 (`finish.rs:100-292` and
  `:3784-3990`, `population.rs:36-370`, `fact_view.rs:1-175`, `file_version.rs:1-129`); a different cut gives a
  different number. Struct fields were not counted.
- Main's tree (`Elements`) was not compared function by function with the tip's `NodeTable`.
- The python `fillers` bench's other users of `NodeTable` (`ryo_python::fixtures`) and the go provider bench were not
  read.
- Whether `retained` (unpooled columns) is used only for tables that outlive the file: inferred from the 7 call
  sites' files, not read.
- Main's allocation tests outside operators and language (core, `execution_runtime`, the nine languages): counted, not
  read; whether any of them judged the pool.
- `cargo` was not run; no `tm check` on a skeleton of this listing; `check-types.py` and `check-graph.py` were not run
  over the folder (the `scheduler` sheet does not exist yet, so `check-graph.py` would report its import as a crate
  with no sheet).
- Build 133 was not read for this crate (nothing of the store existed there that is on neither side, as far as the
  inputs say).
