PROCEED
BEADS: ryo-fmx3.1.13.23.4.51

# Scope: the table spike (three hot paths behind the drafted table contract, measured)

Owner, 2026-11-28: "table spike is fine" (bead notes, 15:20 UTC). A throwaway measurement on branch
`claude/spike-table-port` from `2a207464c`, worktree `/workspaces/ryo-spike-table` (the PM creates it), never merged
and never pushed. Its product is a number and a list of interface lines, written to
`build/ports/stage2/contracts/domain/table/spike.md`; the code is deleted with the branch.

Inputs read: PLAN.md sections 0-7 and 9-10 and Appendix A 3.1; `domain/table/sheet.md` (section 5 and the rest) and
`api.txt`; `discovery-inputs/signatures-evaluator-table.md` and `review-performance.md`; the five memory files the brief
names; `review-criteria.md`; the code at `2a207464c` through `git -C /workspaces/idiomatic-public-benchmark` only; the
measuring scripts in the evidence repository; the kept programs in `/workspaces/idiomatic-target/ryo-redo-ts/landing-139`.
None was missing. One input is wrong and is corrected here: PLAN section 7 step 0 and sheet 8b say "instruction counts
with `price.py`". `price.py` measures CPU seconds through `wait4` (price.py:94; CPU computed at :97) and never counts instructions. The
counting tool is `build/one-engine/scripts/price-line-run.py` in mode `cg`, which runs Callgrind (price-line-run.py:70-73).

Quotes are written `@2a207464c:path:line`. 47 were checked with `git show "2a207464c:path" | sed -n 'Np'`; one was
wrong and has been fixed (`dead_code = "deny"` is at `crates/ryo/crates/executor/Cargo.toml:59`).
Round 2 (after the scope review's S2 FAIL): the sweep in 2.1 was redone by `git grep` at 2a207464c over every
reader of a lent fact (`.fact(`, `.columns()`, `fact_view::Column`, `Columns<`, `build_column`) and every reader of
the antijoin's computed table. 14 copies are not routed; each is named NOT TESTED with an id, and spike.md lists them
first. 22 further quotes were checked in round 2.

---

## 1. Should it happen

**What it gets the goal.** Every other contract sheet is written against the table contract. PLAN section 7 step 0
says: "an unproven table means unproven sheets"; `api.txt` line 8 says "UNPROVEN until domain/table/spike.md records a
pass". This spike is the only step that turns the table's UNPROVEN mark into a measured pass or fail. It also answers
the blocking finding B1 in `review-performance.md` (who owns the layouts that select reads in place) and the third trap
in PLAN 10.5 ("Operators cannot be complete while select reads the tree and lent columns directly"), for two of the
three layouts.

**Step or detour.** It is a step. Under rip and replace the regroup builds straight to the agreed contracts, so a
contract that costs more than +0.2% would land in every crate at once. The spike is the cheapest place to find that
out. The code is thrown away on purpose and costs about 450 changed lines (a guess). The owner approved the spike as a
throwaway, so criterion 10 is met by design: nothing lands, and the result is lines on the sheet.

**Planned and in-flight work.** No other bead touches these files. The regroup (PLAN section 9) is not approved and
waits on the sheets, and the sheets wait on this spike, so the spike goes first. The trunk is frozen at build 139, and
the spike reads it without changing it.

**What the three paths decide, and what they do not.**

- **They decide** whether the two non-tree layouts can sit behind the drafted `TablePort`, `TableRead` and
  `TableWrite`, with the store's type fixed at compile time, at no more than +0.2% instructions. The two layouts are
  the computed column table (`pub struct Table {` @2a207464c:crates/ryo/crates/executor/src/finish.rs:148) and the
  language's lent columns (`pub enum Fact<'a> {` @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:150).
  They cover four access patterns:
  - per-cell reads in filters (view tests, the text test);
  - per-set gathers from both layouts (select's projection, antijoin's kept rows);
  - key-column reads for hashing (antijoin `knock`);
  - one whole-column read (`column_into` at antijoin's keyed read).
- **They do not decide** the tree layout. `TreeRead` and `TreeWrite` are not exercised: the `Native::Tree`, `Child`,
  `Location` and `Token` arms of `view_cols` and `native_get` stay as they are. Also not exercised:
  - `matching` (index probes stay in `native_probe`,
    `pub fn native_probe(&self, run: &Run, nat: Native, col: usize, v: u64, out: &mut Vec<u32>) {` @2a207464c:crates/ryo/crates/executor/src/finish.rs:1039);
  - `append` (union), `row`, `TableWrite::column` and `TableFill`;
  - an explicit `release` (the table's `Drop` already returns its columns to the pool,
    `impl Drop for Table {` @2a207464c:crates/ryo/crates/executor/src/finish.rs:209).

- **They exercise the filter and key-read patterns only partly.** The main copy of the lent-column filter is
  the compiled view filter (`finish/cold_pred.rs` with `cold_word.rs`). It runs *before* `retain_original` whenever a
  compiled kernel is bound (`if !f.vrows.take_filtering_entry() || !super::cold_test::entry(f, run, v, rows) { retain_original(f, run, v, rows); }`
  @2a207464c:crates/ryo/crates/executor/src/finish/view_cols.rs:99; and `cold_test::retain` at cold_test.rs:130-140).
  The spike does not route it. Nor does it route:
  - the project population's own blocks (`finish/population.rs`), whose `contains_keys` reads the antijoin's left
    `.cols` through the raw `&Table` the antijoin passes at antijoin_kernel.rs:101;
  - the join key reader (`view_key.rs`), the single-row probe (`view_probe.rs`), and the fact index (`fact_list` ->
    `Index::build_column`, finish.rs:1019-1033).

  So a PASS proves the routed copies only. The sheet's marks for "per-cell filter reads over lent columns" and "key
  reads" become **"partly exercised; NOT TESTED copies: <ids>"**, never "proven".

  `spike.md` must list every NOT TESTED copy of 2.1 in its first lines (format in section 3), with each copy's share
  of the base's instructions, so a reader sees at once what the number does not cover. The sheet's mark for the
  unexercised methods stays "not exercised".

---

## 2. The design

### 2.0 Behaviour (BDD), drafted first

This is a pure refactor on a throwaway branch. It adds, changes or removes no user-visible behaviour. Each scenario
below maps to a tool that already exists. No runner, feature file, test or Tastemaker asset is added.

```gherkin
# changed: none (refactor). Proved by the existing gate: price.py gate (findings byte-equal to the reference)
Scenario: findings are unchanged on the five Python workloads
  Given the spike's vec program built by images.sh from the spike commit
  When price.py gate runs django, pandas, fastapi, dagster and cases with ONE_PATH_IMAGES set to it
  Then each prints "<workload> gate True <cpu> <peak_mib>"

# new measurement, not behaviour. Proved by price-line-run.py cg (Callgrind) on both programs
Scenario: the contract costs at most +0.2% instructions
  Given the kept build-139 vec (sha256 e7917459...) and the spike's vec, both run on the same pinned pack
  When each runs fastapi and dagster under Callgrind
  Then (spike - base) / base <= +0.200% on each bench, or the result is FAIL

# measurement validity. Proved by the same tool
Scenario: the measurement is calibrated
  Given build 139's vec counted twice on fastapi, and 2a207464c rebuilt unchanged by the same images.sh call
  Then the two build-139 counts differ by at most 0.05%
  And the rebuild differs from build 139 by at most 0.05% on fastapi and on dagster

# held by the compiler and by the acceptance script's greps
Scenario: the routed kernels reach the tables only through the contract
  Then no `dyn TablePort|TableRead|TableWrite` exists, table_port.rs returns no slice, and antijoin_kernel.rs has no `.cols`

# held by the acceptance script
Scenario: the branch is thrown away
  Then claude/spike-table-port has no remote branch and no upstream, and main does not contain it
```

### 2.1 Concepts and placement

The executor is a flat proof-of-concept crate with no nu-hex layers (PLAN 10.5). Placement here is by file role inside
`finish/`, and both new files are deleted with the branch.

| Concept | Lives now (2a207464c) | In the spike | Why |
|---|---|---|---|
| the table contract | nowhere (`@new` in api.txt) | `finish/table_port.rs`: traits and types only, no implementation | the one place a kernel may name, standing in for the future `domain/table` crate |
| the store over today's table | `Table` with public fields, finish.rs:147-292, and its pool at finish.rs:155-207 | `finish/table_store.rs`: `Store`, `OwnedRead`, `Writer` implement the contract over `Table` unchanged | the only file that touches `.cols` for the routed paths: today's layout behind one seam |
| the lent reader | `Fact`/`Columns`/`Column` in fact_view.rs:21-169 | `FactRead` in `table_store.rs` wraps `Fact` and calls fact_view's own reads (`at` :30, `cell` :82/:160, `get` :117, `extend_into` :46) | no second translation of lent words: fact_view.rs is reused unchanged (criterion 6) |
| the text function bound per symbol | `TextTest::symbol` (`pub fn symbol(&self, run: &Run, id: u32) -> bool {` @2a207464c:crates/ryo/crates/executor/src/finish/symbol_predicate.rs:102) | unchanged; it receives the word read through `TableRead` | functions belong to the operators' domain (owner, PLAN section 1); only how the argument is read changes |
| cell word, cell kind, row count, row number, column number | `CellWord` @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:289; `CellKind` @2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:220; `RowCount`, `RowIndex` @2a207464c:crates/ryo/crates/values/src/domain/rows/types.rs:7,4; `ColumnIndex` @2a207464c:crates/ryo/crates/values/src/domain/plan_parts/types.rs:54 | re-exported by `table_port.rs` (`RowIndex as RowNumber`, `ColumnIndex as ColumnNumber`) | the sheet says these are the existing types renamed; re-exporting adds no second definition |
| `Shape`, `ColumnName` | nowhere (`Names`/`Tys` are `Rc<Vec<String>>`/`Rc<Vec<Ty>>`, finish.rs:143-144) | defined in `table_port.rs` exactly as in api.txt lines 70 and 81 | drafted types; `table_store.rs` holds the one conversion to and from `Names`/`Tys` |

**Every existing copy, swept, with its disposition** (the builder's sweep starts here). Round 2 re-swept with
`git grep -n -E '\.columns\(\)|\.raw\(|fact_view::cached|build_column|\.fact\(' 2a207464c -- crates/ryo/crates/executor/src`
and with `fact_view::Column|Columns<` and `contains_keys`. A copy that is not routed is **NOT TESTED** under the id in
brackets, and appears in spike.md's first lines.

| Concept | Copy | Disposition |
|---|---|---|
| reading a cell of a lent fact | view_cols.rs:57-59 (`cell`, `Native::Fact` arm) | routed: `FactRead` through `TableRead` |
| | view_cols.rs:107-131 (`retain_original`, Eq/Ne/In over a fact) | routed: one generic helper `keep_where` |
| | view_cols.rs:143-151 (`gather_ids`, fact arm) | routed: one generic helper `gather_column` |
| | finish.rs:920-962 `native_get`, fact arm :942 (`self.fact(usize::from(i)).map_or(NULL, \|t\| t.cell(c, r))`) | **NOT TESTED [native_get]**: `native_get` is the tree-and-location reader; outside the three paths |
| | finish/cold_pred.rs:19-163 (`Action::retain`, `Action::bind`, `action`: `Fact`, `Pair`, `Rows` read `cols.get(c).at(r)`; `Row` is a boxed `dyn Fn` called per row, cold_pred.rs:7) and :291-390 (`Program::rows` entry over a fact, `Program::retain` taking `f.fact(i)...columns()` at :381) | **NOT TESTED [cold_pred]**: the compiled view filter. It runs before `retain_original` whenever a kernel is bound (view_cols.rs:99, cold_test.rs:130-140), so it is the main copy of the lent filter |
| | finish/cold_word.rs:26-45 (`Word::scan`, `Word::retain` over a `fact_view::Column`, called from cold_pred.rs:36, :346, :352) | **NOT TESTED [cold_word]**: part of the compiled filter |
| | finish/view_key.rs:18 (`Src::Tab` key column `&table.cols[col]`), :24-25 (lent `t.columns().get(c)`), read per row by `read` at :32-43 | **NOT TESTED [view_key]**: the join's key reader; kernel 2's path |
| | finish/view_probe.rs:183-192 (a one-row fact read as `columns().get(c).at(0)`; otherwise `fact_list`) | **NOT TESTED [view_probe]** |
| | finish.rs:1019-1033 `fact_list` -> `Index::build_column` (finish.rs:3847: `column.extend_into`) | **NOT TESTED [fact_list]**: the index over a lent column is `matching`'s concept, not exercised |
| | finish/population.rs:415-425 (`full`), :536-548 (`contains_keys`), :551-553 (`neighbors`), :556-565 (`probe`) over the population's own `Words` blocks (`pub enum Words {` population.rs:40) | **NOT TESTED [population]**: a third stored layout (the project phase's kept blocks), not drafted on the sheet; `contains_keys` also reads the antijoin's left table as `left.cols[..][row]` (:544, :547) through the raw `&Table` the antijoin passes at antijoin_kernel.rs:101. So "every `.cols` and every table `.n` read goes through the reader" holds for antijoin_kernel.rs only, not for what it calls |
| | row counts and kinds read off a fact: finish.rs:988, :1009 (`len`), :3578 (`ty`), keyed.rs:163, view_choice.rs:36, emptyprop.rs:245-246 | **NOT TESTED [fact_meta]**: `rows()` and the kind half of `shape()`, outside the three paths |
| | reader_run/packet.rs:134-137 (`Fact::Lent { .. columns: Columns::Lent(..) }` built by the language binding) | **NOT TESTED [lent_constructor]**: where a lent reader comes from. The routed code names `FactRead::from(fact)` directly, not through any port (finding F5) |
| | **callers that become routed reads without a change of their own:** `view_cols::gather` callers finish.rs:1835, :2975, contribution.rs:371, select_kernel.rs:1091, view_pair.rs:95 and `gather_ids` at finish.rs:1902; `view_cols::cell` callers finish.rs:2509, :3425, :4026, view_input.rs:126, view_key.rs:40, antijoin_kernel.rs:118 | their **lent reads are routed** (through `gather_ids`' and `cell`'s fact arms); their **output columns are still written raw** (`&mut out.cols[c]`). spike.md says so beside the crossing counts, which include these callers |
| gathering rows of columns into a new table | antijoin_kernel.rs:215-231 `gather` | routed: `TableWrite::gather` |
| | select_kernel.rs:1112-1126 (`ksel_one` projection) | routed for `Src::Tab` and for a view over a present fact; the tree-view fallback (today's lines 1120-1123 when the base is not a fact) stays |
| | select_kernel.rs:1079-1095 (kernel 2's join output) | **NOT TESTED [kernel2]**: outside the three paths; owned by the operators sheet |
| | the table writes of finish.rs:1835, :1902, :2975, contribution.rs:371, select_kernel.rs:1091, view_pair.rs:95 | **NOT TESTED [raw_writes]**: their reads are routed (row above); their writes are not |
| | `union_kernel.rs` (whole file) | **NOT TESTED [union]**: `append` is not exercised |
| key-column reads for hashing | antijoin_kernel.rs:151-212 `knock` | routed: generic over `R: TableRead` |
| | select_kernel.rs:933-956 (kernel 2's index); `Index::build` (finish.rs:3838) | **NOT TESTED [kernel2]** (same id as above); `build_column` is under [fact_list] |
| | population.rs:536-548 `contains_keys` (the antijoin's population path) | **NOT TESTED [population]** (same id as above) |
| a column against a constant, or a text test, on a computed table | select_kernel.rs:158-182 (`filter`, `Src::Tab` arms: text test, Eq to a constant, two columns) | routed: one `OwnedRead` per call |
| | select_kernel.rs:114-153 `holds` through `get`; `Exec::select` (`pub fn select(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {` @2a207464c:crates/ryo/crates/executor/src/finish.rs:2559) | **NOT TESTED [interp_select]**: the generic per-row interpreter path |
| the tree layout | the `Native::Tree`, `Child`, `Location`, `Token` arms of view_cols.rs, `native_get`, cold_bucket.rs, walk.rs, cold_pred's kind and field actions | **NOT TESTED [tree]**: `TreeRead` is not in the spike |
| making a table with a shape | `Table::with`/`with_rows`/`retained` (finish.rs:218-237), `Exec::empty`/`empty_rows` (finish.rs:1947-1955) | used inside `Writer` only (pool kept, gain 37); routed callers stop calling them |
| `Ty` to and from `CellKind` | `ty_of` (`pub fn ty_of(k: CellKind) -> Ty {` @2a207464c:crates/ryo/crates/executor/src/finish.rs:110) | reused; the inverse `kind_of(Ty) -> CellKind` is added once in `table_store.rs` (no inverse exists at 2a207464c: `git grep "=> CellKind::Symbol"` under executor finds nothing) |

### 2.2 Structure (spike branch only; `+` added, `~` changed; executor: flat, no nu-hex layer)

```
crates/ryo/crates/executor/src/
├── finish.rs                     ~ two lines: `pub mod table_port;` `pub mod table_store;`
└── finish/
    ├── table_port.rs             + the drafted contract (traits, types, re-exports); no implementation
    ├── table_store.rs            + Store, OwnedRead, FactRead, Writer (and the variant-B cursors); the only `.cols` of the routed paths
    ├── view_cols.rs              ~ fact arms of cell, retain_original, gather_ids
    ├── select_kernel.rs          ~ filter (Src::Tab arms), K (4 fields), ksel_make (fills them), ksel_one
    ├── antijoin_kernel.rs        ~ whole file: Prep (2 fields), prep, whole, keyed_left, over, knock, gather
    ├── fact_view.rs                unchanged (reused)
    └── symbol_predicate.rs         unchanged (reused)
crates/ryo/crates/sdk/src/resources/linter/lint/session/measure.rs   ~ count commit only: one eprintln line at the end of print_measurement
```
Evidence repository: `+ build/ports/stage2/contracts/domain/table/spike.md`. The tools also write, as they always do,
`build/one-engine/runs/price-line/spk*/` (Callgrind traces) and `build/one-engine/runs/spk-*-gate.json`.

### 2.3 Interfaces and contracts

**Variant A: api.txt lines 11-34, word for word** (`TableFill` and the tree traits are left out: not on the three paths).

```rust
// crates/ryo/crates/executor/src/finish/table_port.rs   (THROWAWAY, bead ryo-fmx3.1.13.23.4.51)
pub use crate::languages::{CellKind, CellWord};                                   // @2a207464c:crates/ryo/crates/executor/src/languages.rs:8,9
pub use ryo_values::domain::rows::types::{RowCount, RowIndex as RowNumber};       // RowNumber(pub usize)
pub use ryo_values::domain::plan_parts::types::ColumnIndex as ColumnNumber;       // ColumnNumber(pub u32)

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ColumnName(pub Box<str>);                                              // @new (api.txt:70)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Shape { pub columns: Vec<(ColumnName, CellKind)> }                     // @new (api.txt:81)

pub trait TablePort {                                                             // @new
    type Table;
    type Reader<'t>: TableRead where Self: 't;
    type Writer: TableWrite<Table = Self::Table>;
    fn read<'t>(&'t self, table: &'t Self::Table) -> Self::Reader<'t>;
    fn write(&mut self, shape: &Shape) -> Self::Writer;
    fn release(&mut self, table: Self::Table);
}
pub trait TableRead {                                                             // @new
    fn shape(&self) -> &Shape;
    fn rows(&self) -> RowCount;
    fn cell(&self, row: RowNumber, column: ColumnNumber) -> CellWord;
    fn column_into(&self, column: ColumnNumber, into: &mut Vec<CellWord>);
    fn matching(&self, column: ColumnNumber, key: CellWord, into: &mut Vec<RowNumber>);
}
pub trait TableWrite {                                                            // @new
    type Table;
    fn row(&mut self, cells: &[CellWord]);
    fn column(&mut self, cells: &[CellWord]);
    fn gather(&mut self, from: &impl TableRead, rows: &[RowNumber], columns: &[ColumnNumber]);
    fn append(&mut self, from: &impl TableRead);
    fn finish(self) -> Self::Table;
}
```

**The implementation (variant A), `finish/table_store.rs`.** Every field is private, and no type has an inherent
method except `From`. Code outside this file can reach a table only through the three traits.

```rust
pub struct Store;   // no state: today's pools are the worker's thread-locals (finish.rs:155-207); one value per routed function
pub struct OwnedRead<'t> { table: &'t Table, shape: OnceCell<Shape> }       // shape() builds once from names/tys
pub struct FactRead<'a> { fact: fact_view::Fact<'a>, shape: OnceCell<Shape> }
pub struct Writer { names: Names, tys: Tys, table: Option<Table> }          // columns taken from the pool at the first gather
impl<'a> From<fact_view::Fact<'a>> for FactRead<'a>;
impl TablePort for Store { type Table = Table; type Reader<'t> = OwnedRead<'t>; type Writer = Writer; /* ... */ }
impl TableRead for OwnedRead<'_>;   // cell = CellWord(table.cols[c][r]); rows = RowCount(table.n)
impl TableRead for FactRead<'_>;    // cell = CellWord(fact.cell(c, r))  i.e. fact_view.rs:160 -> :82, the existing per-cell read; no cache of its own
impl TableWrite for Writer;         // gather: Table::with_rows(names, tys, rows.len()) on first use, then output column j += from.cell(r, columns[j]) for r in rows
pub fn row_numbers<T>(rows: &[u32], f: impl FnOnce(&[RowNumber]) -> T) -> T;   // widens into one thread-local reused Vec; never nested
fn kind_of(ty: Ty) -> CellKind;                                             // the inverse of ty_of
```
The rest of variant A, pinned so that two builders write the same thing:
- **`Store::write(&Shape)`** makes fresh `Names` and `Tys` from the shape, allocated each time. This is the drafted
  signature's real cost over today's table: a shape lent by reference has to be copied into the table. There is no
  cache keyed by address and no interner.
- **`Store::release(t)`** drops `t`; the drop already returns the columns to the pool.
- **`FactRead::shape()`** panics with "not exercised: a lent fact carries kinds but no column names". No truthful body
  exists. This is finding F1 in section 3.
- **`matching`** scans the column (`for r in 0..rows`).
- **`column_into`** reads through `Column::extend_into` (fact_view.rs:46) for a lent fact and through
  `extend_from_slice` for an owned table, then wraps each word in `CellWord`.
- **`Writer::row`, `column`, `append`** are the plain push, push-column and append loops. They are not called in the
  routed paths.

**Variant B is A plus exactly two named interface changes.** It is built only if A fails (section 3, step 6).

```rust
// D1: a column cursor. A logical reader of one column; no storage is handed out.
pub trait ColumnRead { fn at(&self, row: RowNumber) -> CellWord; }                // @new (D1)
// added to TableRead:
type Column<'c>: ColumnRead where Self: 'c;                                       // @new (D1)
fn column(&self, column: ColumnNumber) -> Self::Column<'_>;                       // @new (D1)
// D2: a prepared shape. Replaces `fn write(&mut self, shape: &Shape) -> Self::Writer;`
type Shape;                                                                       // @new (D2)
fn prepare(&mut self, shape: &Shape) -> Self::Shape;                              // @new (D2), once per step at prep
fn write(&mut self, shape: &Self::Shape) -> Self::Writer;                         // changed (D2)
```
In `table_store.rs` for B:
- `type Shape = Prepared`, with `pub struct Prepared { names: Names, tys: Tys }` (write clones two `Rc`s, as
  `empty_rows` does today);
- `OwnedRead::Column<'c> = OwnedColumn<'c>`, with `pub struct OwnedColumn<'c>(&'c [u64])` and a private field;
- `FactRead::Column<'c> = fact_view::Column<'c>`, made by `fact.columns().get(c)` (fact_view.rs:117, today's hoisted
  read). `ColumnRead::at` calls `Column::at` (fact_view.rs:30).
- `Writer::gather` and every routed filter loop take one cursor per column and call `.at(r)` per row.
- The dedup in `ksel_one` reads by `cell` in both variants.

**Routed signatures** (the calling code in `Exec` stays concrete and makes `let mut store = Store;` once per function):

```rust
// antijoin_kernel.rs
pub struct Prep { /* existing fields */ pub shape: Shape /* B: Prepared */, pub all_cols: Box<[ColumnNumber]> }
pub fn knock<R: TableRead>(pr: &Prep, l: &R, r: &R, w: &mut Work);                                // was (pr, l: &Table, r: &Table, w)
pub fn gather<S: TablePort<Table = Table>>(store: &mut S, pr: &Prep, l: &Rc<Table>, keep: &[u32]) -> Rc<Table>;   // B: S: TablePort<Table = Table, Shape = Prepared>
// prep, whole, keyed_left, over: signatures unchanged; bodies read l and r through store.read(..)
// select_kernel.rs
pub struct K { /* existing fields */ pub shape: Shape /* B: Prepared */, pub tab_cols: Box<[ColumnNumber]>, pub base_cols: Option<Box<[ColumnNumber]>>, pub tab_width: usize }
// filter and ksel_one: signatures unchanged
// view_cols.rs: cell, retain_original, gather_ids keep their signatures; two private generic helpers are added
fn keep_where(r: &impl TableRead, column: ColumnNumber, rows: &mut Vec<u32>, test: impl Fn(u64) -> bool);
fn gather_column(r: &impl TableRead, column: ColumnNumber, rows: impl Iterator<Item = u32>, out: &mut Vec<u64>);
```
Nothing is sealed, because the code is thrown away; the final `domain/table` crate seals according to its sheet.
**Never public and never added:** a method or associated type that hands out `&[u64]`, `&[CellWord]`, `&Vec<_>`,
`Vec<Vec<_>>`, `&Table`, a raw pointer or a storage index.

### 2.4 Logical flow and primitives (the operators' terms for the eventual crates)

1. **Select over a lent fact.** `Exec::ksel_run_` (application service, unchanged) runs the select.
   1. `view_cols::retain` filters the candidate rows. It first tries the compiled filter (`cold_test::entry`,
      view_cols.rs:99), which is NOT TESTED [cold_pred]. Only when no compiled kernel takes the entry does
      `retain_original`'s fact arm run. That arm calls `keep_where(&FactRead::from(fact), c, rows, test)` (a domain
      service, generic over the port `TableRead`).
   2. The reader's `cell` (A) or `column(c).at(r)` (B) is the adapter, which reuses fact_view's lent read.
   3. `Exec::ksel_one` (domain service) makes the output: `store.write(&kp.shape)` (port `TablePort`, adapter
      `Store`), then `w.gather(&FactRead::from(fact), rows, kp.base_cols)` (port `TableWrite`, adapter `Writer`, which
      takes its columns from the existing pool through `Table::with_rows`), then `w.finish()`.
   4. When the projection drops columns, the dedup reads `store.read(&out)` by `cell` and gathers the kept rows into
      a second writer, opened before the reader (see F3).
2. **Antijoin.** `antijoin_kernel::over` (application service) reads the left table through `store.read(l)`.
   1. `knock(pr, &lr, &rr, work)` (domain service, generic over `TableRead`) hashes the key columns through
      `cell`/`column`.
   2. `gather(&mut store, pr, l, keep)` gathers every column of the left table (`pr.all_cols`) at `keep` through
      `TableWrite::gather`. `keep` is widened by `row_numbers`.
   3. The keyed read (today's line 130) takes the key column with `column_into`, the one whole-column call on the
      three paths.
3. **Text function bound per symbol.** `select_kernel::filter`, for a `Src::Tab` source, makes one `OwnedRead`.
   1. It calls `test.symbol(run, rd.cell(r, c).0 as u32)` (A) or `.column(c).at(r)` (B).
   2. `TextTest::symbol` and its per-symbol answer pages (`Bound::answer`, a domain value) are reused unchanged.
   3. The Eq-to-a-constant and two-column arms of the same match read the same way.

### 2.5 The one path, and the one thing it must not do

- The routed functions are edited in place. No switch, environment variable or flag chooses between old and new code.
  No second copy of `knock`, `gather`, `ksel_one` or `filter` remains.
- The old paths that already exist behind `VEC_OLD_ANTIJOIN` and `VEC_OLD_SELECT`
  (`frame::Op::Antijoin if !sw!("VEC_OLD_ANTIJOIN") => return antijoin_kernel::whole(self, f, run, p, node),`
  @2a207464c:crates/ryo/crates/executor/src/finish.rs:1982) stay as they are. The spike neither adds nor removes them
  (criterion 2).
- **The one thing it must not do:** widen the contract into today's `cols` under another name, for example
  `fn columns(&self) -> &[Vec<u64>]`, `fn column(&self, c) -> &[CellWord]`, an `as_slice` or `raw` accessor, a public
  field on a reader, or a reader that hands back the `&Table`.
- Anything the compiler forces that is not D1 or D2: **stop**, write the exact line and the reason in `spike.md` under
  "Forced changes", and ask the PM for a re-scope. Do not build or measure it.

### 2.6 Functions and line ranges that change (at 2a207464c)

| File | Lines | Change |
|---|---|---|
| finish.rs | after :70 and :74 | `pub mod table_port;` `pub mod table_store;` |
| finish/view_cols.rs | 57-59, 107-131, 143-151 (+ two helpers) | fact arms read through `FactRead` |
| finish/select_kernel.rs | 158-182 | `Src::Tab` arms read through `OwnedRead` |
| | 215-231 | `K` gains `shape`, `tab_cols`, `base_cols`, `tab_width` |
| | 350-439 | `ksel_make` fills them when `k == 1`: the shape from `self.schema(run, node)?`; `tab_cols` from `items`; `base_cols` from `self.view(run, sp.srcs[0])?`'s `map`; `tab_width = self.schema(run, sp.srcs[0])?.names.len()`. This equals `t.names.len()` because a step's tables share its schema's names (`/// 7: a table's column names and types, shared with its step's schema (never copied per table).` @2a207464c:crates/ryo/crates/executor/src/finish.rs:142) |
| | 1106-1173 | `ksel_one`: projection through the writer; dedup through the reader and a second writer; the tree-view fallback unchanged |
| finish/antijoin_kernel.rs | 11-21, 40-231 | `Prep` gains `shape` and `all_cols`; `empty` is made by `store.write(&shape).finish()`; every `.cols` and every table `.n` read goes through the reader; `knock` is generic; `gather` goes through the writer |
| sdk/.../session/measure.rs | end of `print_measurement` (`pub(crate) fn print_measurement() {` @2a207464c:crates/ryo/crates/sdk/src/resources/linter/lint/session/measure.rs:47) | count commit only: `eprintln!("TABLE_PORT_CROSSED {}", ryo_executor::finish::table_store::crossed());` |

### 2.7 Isolation test and census

- **Replacing the implementation behind the seam.**
  - Today, changing the table layout touches every `.cols` line in the executor: **206 lines in 35 files** (counted
    with `git grep -c -E '\.cols\b' 2a207464c -- crates/ryo/crates/executor/src`; the sheet's figure, re-counted).
  - After the spike, the routed paths go through one file, `table_store.rs`. Outside the store, `.cols` drops by 18
    lines: antijoin_kernel.rs 9 -> 0 and select_kernel.rs 16 -> 7 (kernel 2's index at 933, 936, 955 and 956,
    kernel 2's output at 1082 and 1086, and the tree-view fallback at 1113 remain). That is **188 lines in 34 files**
    (expected; the builder reports the actual figure).
- **Adding the next kernel** (say union) behind the contract would touch `union_kernel.rs` only. The store does not
  change.
- Neither list grows. No ratcheted census row runs on a throwaway branch, and none rises: the spike adds 2 files and
  removes no boundary (criterion 1).

### 2.8 Measurement: the tools, their defects, and whether +0.2% can be seen

| Tool | What it can do here | Known defect (PLAN section 9 and this read) | Use |
|---|---|---|---|
| `perf stat -e instructions:u` | nothing | this VM has no hardware counters: `<not supported>` (run 2026-11-28) | not used |
| `build/one-engine/scripts/price-line-run.py REV IMAGES cg PROJECT` | Callgrind instruction count of the image on the pinned request, whole command, one worker; its `.verified` file proves the run finished | (a) its `old` arm uses the pinned 5e10c3cbd command line (`funnel spec ...`, price-line-run.py:47-62), so build 139 must run as an `entry` arm (`PRICE_ARMS=entry`) under its own tag; (b) it compares dagster with the archived oracle, which build 139 does not equal (139 equals the adjudicated reference: landing gate json `equal: true, archived_equal: false`), so `PRICE_ALLOW_UNEQUAL=1` is set and the output is compared by hand; (c) temporary paths are random, so run-to-run noise is unmeasured: no two independent runs of one image exist (the matching pairs in `runs/price-line` are hard links) | **the acceptance count** |
| `build/one-engine/scripts/price.py gate TAG ...` | findings byte-equal per workload (dagster against the adjudicated reference) | it measures CPU through `wait4`, not instructions; `pairs` compares with the pinned 5e10c3cbd, never with 139; it hard-codes `G` and `I` paths | **findings only** |
| `price-root-split.py` | per-mechanism split of two traces | it truncates the tag to 9 characters and overrides `PRICE_COMPARE_TAG` (lines 9-10); its labels assume a pinned old binary; it matches function names, so renames blind it (review B6) | not used; `callgrind_annotate` (standard) gives the per-function attribution |
| `short-check.sh` | whole short gate | it builds four programs and compares identity against the newest pre-switch image (build 133); it does not check its `git archive` | not used as a whole; its build line (images.sh in a build lane) and its workloads line (price.py gate under the full and quiet locks) are copied |
| `images.sh REV OUTDIR vec` | builds `vec` alone from `git archive REV` in the bench profile (fat LTO, one codegen unit) | it extracts into `OUTDIR/src` without clearing it (harmless here: the spike only adds files); its embedded source path differs from landing-139's | build every arm into one `OUTDIR` (`$T/out`) and copy, so that all spike arms share one source path; the control arm measures any drift from landing-139 |
| `wait-timing-window.sh` | waits while a timing window is open | it waits for ever unless `WAIT_MAX` is set | run before every build and every count |

**Can +0.2% be measured?** Yes, if the runs are deterministic. Callgrind counts do not depend on load. 0.2% of
fastapi is about 36 M instructions, taking build 73's 18.16 B as the reference (PRICE-STATUS; build 139's figure has
not been counted: unverified). The spike checks noise and drift itself:
- `NOISE`: build 139 counted twice on fastapi must agree within 0.05%;
- `CALIBRATION`: 2a207464c rebuilt by the same images.sh call must agree with landing-139 within 0.05% on both benches.

If either check fails, the result is INVALID: no RESULT line, the acceptance fails, and the PM decides. CPU pairs are
not part of the acceptance (decision 1).

### 2.9 Cases and the planted-bug table

In spike terms the "seam tests" are measurements on whole workloads, because the branch and any tests on it are
deleted. Spike mode (owner, 2026-11-21: "prioritize rapid prototyping..."; memory `prototype-before-all-in`: "no
planted bugs, no tm cleanup, no doc gates") means planted bugs are **computed from the crossing counts, not built**
(decision 3).

| Behaviour | Planted bug | What catches it |
|---|---|---|
| reads stay statically dispatched | `&dyn TableRead` passed to `knock` or `keep_where` (compiles in A; in B the GAT makes `TableRead` not object-safe) | the acceptance grep `dyn +Table(Port\|Read\|Write)`; and the instruction count, wherever `crossed(cell or at) × ~5 extra instructions` exceeds 0.2% of the base (spike.md states the arithmetic from the count build; ~5 is a guess) |
| no copy per column | `cell` or `gather` built on `column_into` (a whole column copied per call) | the count: `column_into` crossings and copied cells per bench from the count build; the attribution rows (`memcpy` and `table_store` own instructions, base against spike) |
| no copy of the shape per table | A's `write(&Shape)` | measured as A against B (D2), and as `malloc`/`_rjem_*` rows in the attribution |
| no per-cell column lookup | A's `FactRead::cell` (fact_view :82's cache check on every cell) | measured as A against B (D1); `view_cols`/`fact_view` rows in the attribution |
| the pool is kept (gain 37) | the writer takes plain `Vec::with_capacity` instead of `Table::with_rows` | the allocator rows of the attribution (`_rjem_malloc` call counts, base against spike) |
| findings unchanged | a gather that drops the last row, a widened row out of range | price.py gate on five workloads; the counted runs' own output (fastapi: no `cg.unequal-findings`; dagster: byte-equal to the adjudicated reference) |
| nothing reached past the seam | a `.cols` read left in the antijoin kernel | the acceptance grep on antijoin_kernel.rs |
| no hidden second path | an environment switch between old and new code; a cache keyed by address | the acceptance grep of added lines for `sw!(`, `env::var` and `as *const` |
| not observable at the seam | the mark of an unexercised method (`matching`, `append`, `row`, `release`, tree) | listed in spike.md as not exercised: no routed path calls them, so no count can see them |
| not observable at the seam | the cost of putting a NOT TESTED copy behind the contract (cold_pred, population, view_key, ...) | not measured. spike.md gives each copy's exclusive share of the base's instructions (step 8), which bounds what the number leaves out |

### 2.10 Enforcement

| Boundary | What holds it |
|---|---|
| routed code reads tables only through the contract | the compiler: generic helpers (`knock<R: TableRead>`, `keep_where`, `gather_column`, `gather<S: TablePort>`), private reader and writer fields, no inherent methods |
| no `dyn` | the acceptance grep (and in B the GAT makes `TableRead` not object-safe) |
| no slice handed out | the acceptance grep on table_port.rs for slice returns, `Vec<Vec`, `cols`, `raw` |
| antijoin behind the seam | the acceptance grep: zero `.cols` in antijoin_kernel.rs |
| only the listed files change | the acceptance `git diff --name-only` against the list |
| one store value per function, writers opened before readers | the reviewer only: Rust cannot stop a second ZST `Store` value without making `Store` non-constructible, which a throwaway does not need |

No Tastemaker asset: the executor is outside the Tastemaker rules on this branch, nothing recurs, and the final
`domain/table` crate gets its entries from its own sheet (criterion 9).

### 2.11 Smallest version

Two new files of about 60 and 200 lines, and edits in three files of about 190 lines (guesses): one contract module,
one store, and routing of exactly the functions in 2.6. B adds about 60 lines. The count commit adds about 30. The
only tools are the existing images.sh, price-line-run.py, price.py and wait-timing-window.sh, plus the standard
`callgrind_annotate`. No new script, test, bench or crate is added.

---

## 3. The build plan

**Fixed values** (each command is self-contained; shell state does not carry over):
```
B=/workspaces/ryo-poc-evidence/next/build    W=/workspaces/ryo-spike-table    T=/workspaces/idiomatic-target/ryo-spike-table
L=/workspaces/idiomatic-target/ryo-redo-ts/landing-139   (vec sha256 e7917459c05f1139e24f6b251d19e8ba34502f97b1773db91922484c0361d97d)
PACK: TIP_TREE=/workspaces/idiomatic-target/ryo-redo-empty/tm-int/tip-2a207464c TIP_PYTHON_PACK=/workspaces/idiomatic-target/ryo-redo-empty/tm-int/tip-2a207464c/crates/ryo/languages/python/benchmarks/linters/ruff
```
The pack is pinned at 2a207464c for every arm (it exists; `tm-int/scripts/tip-pack.sh` made it).

**Commands.**
- **BUILD ARM REV** (ARM is `ctl|a|b|cnt`; build lane lock; cores 4-9 by images.sh default):
  `bash $B/ports/wait-timing-window.sh spike-table && bash -c "cd $W && IMAGES_WAIT_SCRIPT=/bin/true IMAGES_TARGET=$T/target bash $B/codex/lane.sh build bash $W/crates/tastemaker/benchmarks/project/live/images.sh REV $T/out vec" && mkdir -p $T/img-ARM && cp $T/out/vec $T/out/SHA256SUMS $T/out/revision $T/img-ARM/`
- **COUNT TAG IMAGES PROJECT** (no lock: untimed and load-independent; never on cores 1-3, which are the timing
  lanes):
  `bash $B/ports/wait-timing-window.sh spike-table && <PACK> PRICE_ARMS=entry PRICE_COMPARE_TAG=TAG PRICE_ALLOW_UNEQUAL=1 nice -n 10 taskset -c 4-9 python3 -IB $B/one-engine/scripts/price-line-run.py 2a207464c IMAGES cg PROJECT`
  The total is the `summary:` line of `$B/one-engine/runs/price-line/TAG/PROJECT/entry/trace.cg`.
- **GATE ARM** (the locks short-check uses):
  `bash $B/ports/wait-timing-window.sh spike-table && flock /workspaces/.tm-full.lock flock -s /workspaces/.quiet.lock env <PACK> ONE_PATH_IMAGES=$T/img-ARM python3 -IB $B/one-engine/scripts/price.py gate spk-ARM django pandas fastapi dagster cases`
  It must print five lines `<workload> gate True <cpu> <peak_mib>`.

**Order, proofs and times.** Times are guesses unless a measurement is cited.

0. **Before anything:** `df -h /workspaces`. It showed 88 GB free when I scoped and 479 GB at review time (2026-11-28). Stop below 50 GB. Check that the
   worktree is on `claude/spike-table-port` at 2a207464c.
1. **In the background, at once** (runs during steps 2-4):
   - COUNT `spk139a $L fastapi`, COUNT `spk139b $L fastapi` and COUNT `spk139a $L dagster`, in parallel. A fastapi
     Callgrind took 2 min 45 s on 2026-11-25 (t22-fastapi, 15.0 B instructions). Dagster has never been counted:
     about 25-35 min, scaled from 16.93 against 1.85 CPU-s.
   - BUILD `ctl 2a207464cebad3f30235abc5c9276de4bc6a1e9e`. images.sh says about 20 min cold for four programs; vec
     alone is about 10-20 min.
2. **Commit 1, the contract:** `table_port.rs` and the two `pub mod` lines. Proof: it compiles in step 4's build.
   Tastemaker assets: none (commit 0 is empty, so it is skipped).
3. **Commit 2, variant A:** `table_store.rs` and the routed edits of 2.6. Proof: as in step 4.
4. After the control build: BUILD `a <commit 2>` (4-8 min warm). Then GATE `a` (five True; about 1-2 min: 26 s of
   runs in the landing-139 gate jsons). Then, in parallel:
   - COUNT `spk-ctl $T/img-ctl fastapi` and `... dagster`;
   - COUNT `spk-a $T/img-a fastapi` and `... dagster`.
5. **Validity:** NOISE (spk139a against spk139b, fastapi) and CALIBRATION (spk-ctl against spk139a, both benches),
   each within 0.05%. If either fails, record it, stop, and report INVALID.
6. **If A is above +0.200% on either bench, commit 3, variant B:** D1 and D2 exactly as in 2.3. You may start coding
   as soon as fastapi's A count is over the bar. Then BUILD `b`, GATE `b`, COUNT `spk-b` on both benches. B is the
   last variant. If it also fails, RESULT is FAIL.
7. **Commit 4, the counters** on top of the final variant. `table_store.rs` gets
   `pub static CROSSED: [AtomicU64; N]`, one `fetch_add(1, Relaxed)` at the top of each implemented method, plus
   `gathered_cells` (rows × columns per gather) and `copied_cells` (per `column_into`), and
   `pub fn crossed() -> String`. measure.rs gets its one line. Then BUILD `cnt` and GATE `cnt`. Read
   `TABLE_PORT_CROSSED` from the end of `diagnostics` in `$B/one-engine/runs/spk-cnt-{fastapi,dagster}-gate.json`.
   These counts are never used as instruction counts.
8. **Attribution:** `callgrind_annotate --inclusive=no TRACE | grep -E 'table_store|table_port|select_kernel|antijoin_kernel|view_cols|fact_view|symbol_predicate|_rjem_malloc|memcpy'`
   on spk139a and on each measured variant, for both benches. Variant A's `FactRead::cell` goes through
   `Columns::cell` (fact_view.rs:82-98), which bumps the per-column read counter on every cell and translates the whole
   column at 1/8 of the rows, against `get`'s once-per-call count and 1/4. Whole-column translation therefore fires at
   a different point, and spike.md records that as part of A's cost.
   **NOT TESTED shares:** for each id, sum the exclusive instructions in the fastapi spk139a trace of the functions
   whose name matches its pattern:

   | id | pattern |
   |---|---|
   | cold_pred | `cold_pred::` |
   | cold_word | `cold_word::` |
   | view_key | `view_key::` |
   | view_probe | `view_probe::` |
   | fact_list | `fact_list\|build_column` |
   | population | `population::` |
   | native_get | `native_get` |
   | union | `union_kernel::` |

   Divide each sum by the trace's total. Use `callgrind_annotate --inclusive=no --threshold=100 TRACE | grep -E PATTERN`
   and add up the first column. The ids fact_meta, lent_constructor, kernel2, raw_writes, interp_select and tree have
   no separable function name: write `n/a`.
9. **Write spike.md**, run the acceptance below from `$W`, then `rm -rf /workspaces/idiomatic-target/ryo-spike-table`
   (exact path, never a glob; sha256s are already recorded). Report `df -h /workspaces`. The traces stay in
   `runs/price-line`. The PM records the result on `domain/table/sheet.md` and in the index, and removes the worktree
   and the branch after acceptance.

**Proofs only the integrator can run:** none. Every proof above is run by the builder: instruction counts, findings
byte identity, and the acceptance script. No test goes red and then green, because no test is added (see below).

**Expected against actual** (spike.md fills the actual column; the expected figures are guesses):

| # | What | Expected (source) | Actual |
|---|---|---|---|
| H1 | A: per-cell lent reads in the **routed** copies (fact_view :82 on every cell instead of `get` once and `at` per row) | 0 to +1.0% fastapi, **an upper bound**. The 28.5 M `Column::at` calls (review-performance section 2, item (b), line 64; not counted at 139) include the NOT TESTED readers (cold_pred, cold_word, view_key, view_probe, fact_list's `extend_into` path) as well as the routed ones. The routed share is unknown until the count build: the real expectation is the `cell` crossings of variant A × ~5-10 instructions | |
| H2 | A: shape copied per output table | +0.1 to +0.5% (one allocation of the names per `ksel_one` and antijoin gather; the count build gives the number of writes) | |
| H3 | widening u32 rows to `RowNumber(usize)` | under 0.02% | |
| H4 | owned-table `cell` in knock and filter | about 0 (the column load is loop-invariant and hoisted) | |
| H5 | B (D1 + D2) in total | within ±0.05% | |

**spike.md: the required lines** (the acceptance reads them). All of them come before the first `## ` heading, and
the NOT TESTED lines come first of all, right after the title. There is exactly one line per id: native_get, cold_pred,
cold_word, view_key, view_probe, fact_list, population, fact_meta, lent_constructor, kernel2, raw_writes, union,
interp_select, tree.
```
NOT TESTED <id>: <path:lines as in 2.1>; fastapi exclusive Ir <n> (<x.xx>%)|n/a
...                                                                    (one per id, 14 lines)
RESULT: PASS|FAIL
VARIANT: A|B
BRANCH: claude/spike-table-port
BASE: /workspaces/idiomatic-target/ryo-redo-ts/landing-139/vec sha256 e7917459c05f1139e24f6b251d19e8ba34502f97b1773db91922484c0361d97d
CONTROL: /workspaces/idiomatic-target/ryo-spike-table/img-ctl/vec sha256 <64 hex> rev 2a207464cebad3f30235abc5c9276de4bc6a1e9e
SPIKE: /workspaces/idiomatic-target/ryo-spike-table/img-<a|b>/vec sha256 <64 hex> rev <40 hex>
NOISE fastapi run1=<n> run2=<n> delta=<+|-x.xxx>%
CALIBRATION fastapi base=<n> control=<n> delta=<+|-x.xxx>%
CALIBRATION dagster base=<n> control=<n> delta=<+|-x.xxx>%
COUNT A fastapi base=<n> spike=<n> delta=<+|-x.xxx>%
COUNT A dagster base=<n> spike=<n> delta=<+|-x.xxx>%
COUNT B fastapi ...   COUNT B dagster ...        (only with VARIANT: B)
CROSSED fastapi read=<n> write=<n> ... cell=<n> ... gather=<n> gathered_cells=<n> ...
CROSSED dagster ...
```
`delta` is (spike − base) / base × 100, to three decimals. RESULT is PASS only if both deltas of the final variant are
at most +0.200. The prose sections are:
- what was not exercised (the methods; the NOT TESTED copies are the lines at the top);
- the expected-against-actual table;
- trait methods used and how often crossed;
- where the instructions went (the attribution rows);
- **Forced changes**: each with the exact interface line before and after, and why. For example: "D1: a whole-column
  cursor had to be added because `cell` resolves the lent column on every call"; not a slice, which would reveal the
  layout;
- findings for the table sheet;
- the `.cols` census outside the store;
- not checked.

**Findings to record whatever the counts say** (they come from the design, not the numbers):
- F1: a lent fact has kinds but no column names, so `TableRead::shape()` cannot be answered truthfully by a lent
  reader. The sheet should say where names live (the relation's declaration) or narrow `shape`.
- F2: `RowNumber(pub usize)` against the tip's `u32` row lists. The spike widens at the boundary. The sheet must choose
  the width; usize doubles every row list.
- F3: `read(&self)` alongside `write(&mut self)` means a writer must be opened before any reader of the same store is
  taken. `ksel_one`'s dedup opens its second writer before it knows whether it needs one.
- F4: `gather` called twice on one writer is not defined by the draft (append rows or columns?). The spike calls it
  once per writer.
- F5: the lent reader is not obtained through `TablePort`. `view_cols` and `ksel_one` name `FactRead::from(fact)`
  directly, and the lent columns are built by the language binding (reader_run/packet.rs:134-137). The sheet must
  say who constructs a lent reader: the language, the store, or a port of its own.
- F6: the compiled view filter (cold_pred.rs:7, `pub type Row = Box<dyn Fn(&File<'_>, &mut Run, Columns<'_>, u32) -> bool>;`)
  already makes one dynamic call per row on today's code. A later routing of [cold_pred] has to keep or remove that
  call; the spike measures neither.
- F7: a third stored layout, the population's `Words` blocks (population.rs:40, `columns()` at :347), is read by key
  (`find`, `each_match`) and is absent from the drafted contract.

Existing reds: none at 2a207464c. The landing-139 gate jsons are True on all five Python workloads.

**What does not apply, because nothing lands** (one line each):
- tests red then green: no seam test is written, since any test on the branch is deleted with it (memory "no
  throwaway tests"); findings identity is the proof;
- `cargo test` of the executor: not run, since it needs a separate debug build; the image build compiles the library;
- Tastemaker assets and `tm check`: none, and not run on a throwaway branch;
- api.txt listing match, semver, docs and READMEs (criteria 12-16): no crate is made and nothing ships;
- review rounds after this scope: spike mode, one scope review only;
- CI, pull request, merge, push: forbidden; the branch stays local;
- census ratchets: not run; the `.cols` figure is reported instead;
- the other families (TypeScript, Rust, Tastemaker images): not built (decision 2).

**Criteria met (by number).**
- 1: replacing the store touches one file; the next kernel touches one file.
- 2: edited in place, with no switch.
- 3: generics, private fields, no `dyn`.
- 4: the measured contract, with unobservable behaviour argued in 2.9.
- 5: n/a; the executor is flat, and the new files are named by role in `finish/`.
- 6: existing types re-exported; one conversion.
- 7: five workloads byte-equal.
- 8: every change has a measured proof.
- 9: compiler, then greps; no Tastemaker asset.
- 10: approved throwaway.
- 11: the per-cell question itself.
- 17: noise and calibration checked.
- 18: Callgrind, `callgrind_annotate`, existing scripts.
- 19: about 450 lines, no new tool.
- 20: the store is the only place that sees `.cols`.
- 21: the three kernels share one store.

ACCEPTANCE:
set -euo pipefail
S=/workspaces/ryo-poc-evidence/next/build/ports/stage2/contracts/domain/table/spike.md
E=/workspaces/ryo-poc-evidence/next/build/one-engine/runs
BASE=2a207464cebad3f30235abc5c9276de4bc6a1e9e
X=crates/ryo/crates/executor/src
test -f "$S"
test "$(git rev-parse --abbrev-ref HEAD)" = claude/spike-table-port
git merge-base --is-ancestor "$BASE" HEAD
test "$(git rev-list --count "$BASE"..HEAD)" -ge 3
test -z "$(git branch -r --contains HEAD)"
test -z "$(git config --get branch.claude/spike-table-port.remote || true)"
if git rev-parse -q --verify refs/remotes/origin/main >/dev/null; then ! git merge-base --is-ancestor HEAD refs/remotes/origin/main; fi
! git grep -q -E 'dyn +(Table(Port|Read|Write)|ColumnRead)' HEAD -- "$X"
! git grep -q -E -- '-> *&('"'"'[a-z_]+ +)?(\[|Vec)|Vec<Vec|\bcols\b|\braw\b' HEAD -- "$X/finish/table_port.rs"
test -z "$(git grep -n -E '\.cols\b' HEAD -- "$X/finish/antijoin_kernel.rs" || true)"
test -z "$(git diff "$BASE" HEAD -- "$X" | grep -E '^\+.*(sw!\(|env::var|as \*const)' || true)"
test -z "$(git diff --name-only "$BASE" HEAD -- . ':!crates/ryo/crates/executor/src/finish.rs' ':!crates/ryo/crates/executor/src/finish/table_port.rs' ':!crates/ryo/crates/executor/src/finish/table_store.rs' ':!crates/ryo/crates/executor/src/finish/view_cols.rs' ':!crates/ryo/crates/executor/src/finish/select_kernel.rs' ':!crates/ryo/crates/executor/src/finish/antijoin_kernel.rs' ':!crates/ryo/crates/sdk/src/resources/linter/lint/session/measure.rs')"
python3 - "$S" "$E" <<'PY'
import json, re, subprocess, sys
from pathlib import Path
S, E = Path(sys.argv[1]), Path(sys.argv[2])
t = S.read_text()
def one(pat):
    m = re.findall(pat, t, re.M)
    assert len(m) == 1, ("expected exactly one line", pat, len(m))
    return m[0]
ids = ["native_get", "cold_pred", "cold_word", "view_key", "view_probe", "fact_list", "population", "fact_meta", "lent_constructor", "kernel2", "raw_writes", "union", "interp_select", "tree"]
top = t.split("\n## ", 1)[0]
lines = top.splitlines()
first_result = next(i for i, l in enumerate(lines) if l.startswith("RESULT: "))
for i in ids:
    got = [n for n, l in enumerate(lines) if re.match(rf'^NOT TESTED {i}: .+; fastapi exclusive Ir (\d+ \(\d+\.\d\d%\)|n/a)$', l)]
    assert len(got) == 1, ("NOT TESTED line missing or repeated before the first heading", i, len(got))
    assert got[0] < first_result, ("NOT TESTED lines must come before RESULT", i)
assert len(re.findall(r'^NOT TESTED ', t, re.M)) == len(ids), "unexpected NOT TESTED lines"
result = one(r'^RESULT: (PASS|FAIL)$')
variant = one(r'^VARIANT: (A|B)$')
one(r'^BRANCH: (claude/spike-table-port)$')
one(r'^(BASE: /workspaces/idiomatic-target/ryo-redo-ts/landing-139/vec sha256 e7917459c05f1139e24f6b251d19e8ba34502f97b1773db91922484c0361d97d)$')
one(r'^CONTROL: /workspaces/idiomatic-target/ryo-spike-table/img-ctl/vec sha256 ([0-9a-f]{64}) rev 2a207464cebad3f30235abc5c9276de4bc6a1e9e$')
v = variant.lower()
_, rev = one(rf'^SPIKE: /workspaces/idiomatic-target/ryo-spike-table/img-{v}/vec sha256 ([0-9a-f]{{64}}) rev ([0-9a-f]{{40}})$')
subprocess.run(["git", "merge-base", "--is-ancestor", rev, "HEAD"], check=True)
L = "/workspaces/idiomatic-target/ryo-redo-ts/landing-139"
T = "/workspaces/idiomatic-target/ryo-spike-table"
adjudicated = (E / "dagster-ruff-adjudicated.findings.jsonl").read_bytes()
def ir(tag, bench, images):
    d = E / "price-line" / tag / bench / "entry"
    assert (d / "cg.verified").exists(), d
    with (d / "trace.cg").open(errors="replace") as fh:
        head = fh.read(8192).splitlines()
    cmd = next(l for l in head if l.startswith("cmd:"))
    assert f"{images}/vec " in cmd, (tag, bench, cmd)
    if bench == "fastapi":
        assert not (d / "cg.unequal-findings").exists(), (tag, "fastapi findings differ from the reference")
    else:
        assert (d / "cg.unequal-findings").read_bytes() == adjudicated, (tag, "dagster findings differ from the adjudicated reference")
    return int(next(l for l in head if l.startswith("summary:")).split()[1])
def check(label, a, b, stated):
    assert abs(round((b - a) / a * 100, 3) - float(stated)) < 0.0015, (label, a, b, stated)
r1, r2, d = one(r'^NOISE fastapi run1=(\d+) run2=(\d+) delta=([+-]\d+\.\d{3})%$')
assert int(r1) == ir("spk139a", "fastapi", L) and int(r2) == ir("spk139b", "fastapi", L)
check("noise", int(r1), int(r2), d); assert abs(float(d)) <= 0.05, "noise above 0.05%"
variants = ["A"] if variant == "A" else ["A", "B"]
deltas = {}
for bench in ("fastapi", "dagster"):
    base = ir("spk139a", bench, L)
    b, c, d = one(rf'^CALIBRATION {bench} base=(\d+) control=(\d+) delta=([+-]\d+\.\d{{3}})%$')
    assert int(b) == base and int(c) == ir("spk-ctl", bench, T + "/img-ctl")
    check("calibration", base, int(c), d); assert abs(float(d)) <= 0.05, "calibration above 0.05%"
    for var in variants:
        b, s, d = one(rf'^COUNT {var} {bench} base=(\d+) spike=(\d+) delta=([+-]\d+\.\d{{3}})%$')
        assert int(b) == base and int(s) == ir("spk-" + var.lower(), bench, T + "/img-" + var.lower())
        check("count", base, int(s), d)
        deltas[var, bench] = float(d)
    one(rf'^(CROSSED {bench} .*\bgather=\d+.*)$')
for var in variants:
    for bench in ("django", "pandas", "fastapi", "dagster", "cases"):
        g = json.loads((E / f"spk-{var.lower()}-{bench}-gate.json").read_text())
        assert g["equal"] is True and g["returncode"] == 0, (var, bench, "findings not equal")
if variant == "B":
    assert any(deltas["A", b] > 0.200 for b in ("fastapi", "dagster")), "variant B is built only when A fails"
passed = all(deltas[variant, b] <= 0.200 for b in ("fastapi", "dagster"))
assert (result == "PASS") == passed, ("RESULT does not follow from the counts", result, deltas)
print("ACCEPTED", result, variant, deltas)
PY
END ACCEPTANCE

---

## Decisions for the PM

1. **CPU pairs in a quiet window (PLAN section 7 step 0 mentions them).** Example: build 139's own gate row says
   `pairs_quiet: NO` and carries pandas 10.47/10.93 (+4.4%), so pairs cannot resolve 0.2%. A quiet window is
   `lane.sh window`, which only the coordinator runs. **Recommend: do not run them.** The acceptance is instructions,
   and pairs belong to the regroup's gate (`window-nine.sh`).
2. **Findings on the five Python workloads only.** This means building `vec` and not `vec_ts`, `vec_rust` or `tm`.
   The memory "all four families, every time" applies to changes that land. Here the bar and both priced benches are
   Python, and building four programs adds about 10 minutes per arm (images.sh comment) for a branch that is deleted.
   **Recommend: Python five.** The regroup's one gate checks all twelve workloads and the identity step.
3. **Build variant B automatically when A fails.** B is the two named changes D1 (column cursor) and D2 (prepared
   shape), with their exact lines in 2.3. The alternative is to stop after A's FAIL and re-scope, which costs a scope
   round and about an hour. **Recommend: automatic B.** Any other forced change still stops the builder.

## Not checked

- Run-to-run determinism of Callgrind on these images (step 5 measures it).
- Build 139's instruction totals, and dagster's Callgrind time (never counted).
- Whether vec-only builds take 10-20 min cold and 4-8 min warm, and how big the target folder is.
- Whether the `Column::at` call count (28.5 M) still holds at 139, and what share of it the routed copies make.
- How much of select's lent-filter work goes through the compiled filter [cold_pred] rather than `retain_original`
  (step 8's shares measure it).
- Whether a copy exists that `git grep` on these names misses, for example a reader that takes `&Table` and indexes
  `.cols` without naming a fact. The 188 `.cols` lines that remain outside the store are reported in spike.md, not
  swept one by one.
- Whether `ksel_one`'s `tab_width` equals `t.names.len()` on every path. The schema-sharing comment at finish.rs:142
  says so, and the findings gate would catch a change in effect.
- That landing-139 was built by this images.sh. Its revision file says 2a207464c and it holds the same four
  programs; the control arm checks the rest.

## PM decisions (28 Nov, the main session; the owner approved the spike: "table spike is fine")
1. No CPU pairs: instruction counts only. Accepted as recommended.
2. Findings checked on the five Python workloads only; build vec only. Accepted.
3. Build variant B automatically when A fails; any other forced change stops for a re-scope. Accepted.

## Scope review

Round 1 (ROUND=1). This is a throwaway spike, so the rows about landing (Tastemaker assets, `tm check` skeleton, published-API docs, integrator) are graded on whether the report says why they do not apply. I checked the code at `2a207464c` through `git show` only. I re-checked 17 of the report's `@2a207464c` quotes, the `.cols` census (206 lines in 35 files; antijoin 9, select 16, and the 7 select lines that stay), the pinned pack, the landing-139 `vec` sha256 and revision, `price.py`, `price-line-run.py`, `images.sh`, `lane.sh` and `wait-timing-window.sh`.

- S1 Worth it: PASS. The gain is named against PLAN 7 step 0 and api.txt:8 (UNPROVEN until spike.md records a pass). The owner approved it as a throwaway. It is ordered ahead of the sheets and the regroup, and its overlap with them is resolved. What it decides and what it does not is listed. That list must grow with the S2 fix.
- S2 Placement: FAIL. The Structure tree, the Interfaces and contracts block and the Logical flow are all present and match the cited code. But the claim "Every existing copy, swept" is false: several existing readers of lent fact columns and of the antijoin's computed table are missing from the sweep table. spike.md's "not exercised" lines and the sheet's marks are built from that table, so a PASS would mark the filter and key-read patterns as proven while their main copies skip the contract. Fix: add a row for each copy below, with its disposition. They are probably "not routed", and then they go in spike.md's first lines as not exercised.
  - `finish/cold_pred.rs:41-154` and `:313-381`: the compiled view filter. `Action::Fact`, `Pair` and `Rows` read `Columns::get(c).at(r)` directly. It runs before `retain_original` whenever a cold kernel is bound: `view_cols.rs:99` calls `cold_test::entry`, and `cold_test::retain` uses the kernel at `cold_test.rs:130-140`.
  - `finish/population.rs:420-563`: block gathers and `neighbors`/`probe` over lent columns. Also `contains_keys` (`:536-548`), which reads the antijoin's left `.cols` and is called with the raw `&Table` from `antijoin_kernel.rs:101`. That makes "every `.cols` and every table `.n` read goes through the reader" true of the antijoin file only.
  - `finish/view_key.rs:25`, `finish/view_probe.rs:187` and `finish.rs:1026` (`Index::build_column` over a lent column).
  - Qualify H1 as well: the 28.5 M `Column::at` calls (review-performance section 2(b), line 64) include these unrouted readers.
- S3 Contract: PASS. Variant A is api.txt:11-34 word for word, and variant B is exactly D1 and D2 written as signatures. All fields are private, there are no inherent methods except `From`, and the "never public" list is explicit. The boundary types are the nominal newtypes (`RowNumber`, `ColumnNumber`, `CellWord`, `Shape`, `ColumnName`). It is unsealed by design, and the report says why. The `tm check` skeleton is n/a because no crate is made, and the report says so.
- S4 One path: PASS. The routed functions are edited in place with no switch. The existing `VEC_OLD_*` switches are untouched, and that is stated. No second `knock`, `gather`, `ksel_one` or `filter` remains, and the acceptance greps added lines for `sw!(`, `env::var` and `as *const`.
- S5 Isolation: PASS. The replace list (one file, table_store.rs) and the add-a-kernel list (union_kernel.rs) are believable. The `.cols` census goes from 206 lines in 35 files to an expected 188 in 34. I verified the 18 removals line by line. No row rises.
- S6 Contract tests: PASS. The BDD block maps each scenario to an existing tool (the price.py gate, Callgrind through price-line-run.py, the acceptance greps), with no runner added. It is a pure refactor and the gate is named. Planted bugs are computed rather than built, which spike mode allows (decision 3). Unexercised methods are argued as unobservable.
- S7 Enforcement: PASS. The order is compiler (generics, private fields), then acceptance greps, then the reviewer only for "one store value per function", with its reason given. No Tastemaker asset, and the reason is given.
- S8 Criteria: PASS. Items 1-11 and 17-21 are met or argued. Items 12-16 are n/a because nothing ships. The panicking `FactRead::shape` is a deliberate throwaway stub, recorded as F1.
- S9 Buildable: PASS. The design compiles under the executor's `dead_code`, `private_interfaces` and `unnameable_types` denies: trait impls are not dead and all routed types are `pub`. `file_exclusion` takes an `FnOnce`, so the `gather` closure can borrow the store mutably. The `vec` image calls `print_measurement` (lint_image/main.rs:157), so the counter line appears. The callgrind `summary:` and `cmd:` lines sit in the trace header that the acceptance reads. The pack, landing-139 (sha256 `e7917459…`), the adjudicated reference and the scripts all exist. Each proof is runnable by the builder.
- S10 Evidence: PASS. Spot-checked: `finish.rs:148` `pub struct Table {`, `finish/fact_view.rs:150` `pub enum Fact<'a> {` and `finish.rs:1982` (the `VEC_OLD_ANTIJOIN` arm). All three hold, and so do 14 more. The false sweep claim is graded under S2.

### Notes for the builder (fix in the first commit of spike.md, or in the scope text)
1. "`dead_code = "deny"` is at Cargo.toml:59": the file is `crates/ryo/crates/executor/Cargo.toml:59`. There is no root Cargo.toml at 2a207464c.
2. "review-performance §4b" (H1): it is section 2, item (b), line 64.
3. `price.py` calls `wait4` at :94; CPU is computed at :97.
4. Free space: `df -h /workspaces` shows 479 GB free on 2026-11-28 at review time, not 88 GB. The stop rule is unchanged.
5. Flow 2.4 step 1.1: `view_cols::retain` tries `cold_test::entry` first (view_cols.rs:99). `retain_original`'s fact arm runs only when no compiled kernel takes the entry.
6. In the sweep, the "not routed" gather sites (finish.rs:1835, :1902, :2975, contribution.rs:371, select_kernel.rs:1091, view_pair.rs:95) still read the lent fact through `view_cols::gather` → `gather_ids`, whose fact arm is routed. Only their table writes stay unrouted. The same holds for every `view_cols::cell` caller (finish.rs:2509, :3425, :4026, view_input.rs:126, view_key.rs:40). Say so, so the crossing counts are read correctly.
7. Variant A's `FactRead::cell` goes through `Columns::cell` (fact_view.rs:82-98). That bumps the per-column read counter on every cell, with a translate threshold of 8× instead of `get`'s once-per-call 4×. Whole-column translation therefore fires at a different point. Record this in the attribution; it is part of what A costs.
8. `FactRead::from(fact)` is a table_store type named directly by view_cols: the lent reader is not obtained through `TablePort`. Add this as a finding for the sheet: who constructs a lent reader.
9. State the existing reds explicitly as "none at 2a207464c (landing-139 gate: five True)".
10. `price.py gate` prints `<workload> gate True <cpu> <peak_mib>`, not exactly `<workload> gate True`.

VERDICT: FAIL

## Scope review (round 2)

Round 2 (ROUND=2). I graded S2 again (it failed in round 1), plus the rows whose content the fix changed: S1 (section 1), S6 (H1 and the planted-bug table), S9 (the required spike.md lines and the acceptance block) and S10 (the new quotes). S3, S4, S5, S7 and S8 carry their round-1 PASS forward, because their content did not change. I checked the code at `2a207464c` through `git show` and `git grep` only. I re-ran the sweep greps from 2.1 and checked every hit. I also checked that the landing-139 `vec` carries symbols for step 8's attribution: `nm -C` lists 130 matching names.

- S1 Worth it: PASS (re-graded). The gain is still named against PLAN 7 step 0 and api.txt:8. The list of what the spike decides and what it does not now says that the filter and key-read patterns are only partly exercised. The sheet's marks become "partly exercised; NOT TESTED copies: <ids>", never "proven". The overlap with the sheets and the regroup is resolved.
- S2 Placement: PASS. Every hit of the re-run sweep (`.fact(`, `.columns()`, `.raw(`, `build_column`, `fact_view::Column`, `Columns<`, `contains_keys`) is either in the table or is a presence-only check (`is_some`/`is_none` at finish.rs:1702, :2441, keyed.rs:142, select_kernel.rs:479, lazy_view.rs:24, view_entry.rs:71, emptyprop.rs:61). The four copies found in round 1 now have rows with ids: cold_pred, cold_word, view_key, view_probe, fact_list and population. The population row says correctly that the antijoin file is behind the seam but its callees are not. Flow 2.4 step 1.1 now puts the compiled filter first. One more callee of the antijoin was missed (`file_exclusion`, note 1). It does not change what gets built or measured, and the marks are already "partly exercised". The "Not checked" section discloses this class ("a reader that takes `&Table` and indexes `.cols`"). So it is a note, not a failure. The Structure tree, the Interfaces block and the Logical flow are unchanged and still match the code.
- S3 Contract: PASS (carried forward from round 1; unchanged).
- S4 One path: PASS (carried forward from round 1; unchanged).
- S5 Isolation: PASS (carried forward from round 1; the census of 206 lines in 35 files, with antijoin at 9 and select at 16, re-verified).
- S6 Contract tests: PASS (re-graded). H1 is now an upper bound, and it names the unrouted readers inside the 28.5 M `Column::at` calls (review-performance.md line 64, verified). A new row in the planted-bug table argues why a NOT TESTED copy's cost cannot be observed. Each copy's exclusive share bounds what the number leaves out. The BDD block is unchanged and still maps to existing tools.
- S7 Enforcement: PASS (carried forward from round 1; unchanged).
- S8 Criteria: PASS (carried forward from round 1; unchanged).
- S9 Buildable: PASS (re-graded). The new acceptance block parses the 14 NOT TESTED lines. It needs exactly one line per id before RESULT and before the first `## ` heading, and no other line that starts with `NOT TESTED `. That matches the stated format, including `n/a`. Step 8's tools exist (`callgrind_annotate --inclusive=no --threshold=100`). The base binary is not stripped and has debug info. The rest of the acceptance is unchanged from round 1. The round-1 notes are applied: the Cargo.toml path, price.py :94/:97, df, flow 1.1, the callers whose reads are routed, `Columns::cell`'s threshold, F5, existing reds, and the gate output format.
- S10 Evidence: PASS (re-graded). I spot-checked three new citations, and all hold: population.rs:536-548 (`contains_keys` taking `left: &Table` and reading `left.cols[lk[key]][row]` and `left.cols[lc][row]`), view_key.rs:18 (`Src::Tab(table, _) => return Self::Table(&table.cols[col])`) with :24-25, and cold_test.rs:130-140 (`kernel.retain` when a cold kernel is bound). cold_pred.rs:7 and :381, population.rs:40 and :347, finish.rs:3847 and review-performance.md:64 also hold.

### Notes for the builder (round 2; fix in the first commit of spike.md, or the PM amends the scope text)
1. **A missed copy, file_exclusion.** `Exec::file_exclusion` (finish/view_input.rs:73-103) reads the antijoin's left table raw: `left.n` and `left.cols[*lc][*i as usize]` at view_input.rs:98. It is passed the raw `&Rc<Table>` at antijoin_kernel.rs:135. It is on the default antijoin path: landing-139's `vec` holds `file_exclusion<antijoin_kernel::over::{closure#3}>`. Make three changes:
   - Add a row to 2.1 for it: **NOT TESTED [file_exclusion]**. Name it beside `contains_keys` in the population row's caveat.
   - Add `file_exclusion` with pattern `file_exclusion` to step 8's table. Note that the pattern also matches the `cursor::<..file_exclusion..>` closure, which is fine.
   - Add the id to the required spike.md lines (15 lines) and to the acceptance's `ids` list.

   The acceptance rejects any extra `NOT TESTED ` line, so the PM must amend the `ids` list before launch. If it is not amended, the builder records the copy in the "what was not exercised" section, in a line that does not start with `NOT TESTED `.
2. The `view_cols::cell` caller list in 2.1 leaves out finish.rs:4029 (`view_cols::cell(f, run, *n, cur[si], col)`). It also leaves out view_cols's own `holds` (view_cols.rs:74-81) and the tree branch of `gather_ids` (:154). Their fact reads are routed through `cell`, the same as the listed callers. Count them when you read the crossing counts.
3. The old interpreter antijoin `Exec::antijoin` (finish.rs:2491-2550, behind `VEC_OLD_ANTIJOIN`) is a further copy of key hashing and row gathering. Section 2.5 already says it stays as it is. Give it a line in 2.1 too, as "untouched, behind the switch, never run by the measured image (share 0)". It needs no NOT TESTED id, because it does not run.
4. Step 8's shares count exclusive instructions per function symbol. In a fat-LTO, one-codegen-unit image, code inlined into a caller is charged to the caller, so each share is a lower bound. The pattern `cold_pred::` also matches `drop_glue` symbols for its types. Also, `callgrind_annotate` prints Ir with thousands separators, so strip them before adding. Say all of this beside the shares in spike.md.
5. No prose line in spike.md may start with `NOT TESTED ` outside the 14 (or 15) required lines. The acceptance counts them across the whole file.

VERDICT: PASS
