<!-- a read-only reader's final report, 27 to 28 Nov 2026; task aef258b22f32bd3a2; copied from the session transcript -->

All items below were read at T=2a207464c or O=f0219b3b3 through `git show` and `git grep`. `E` abbreviates `crates/ryo/crates/executor/src`, `L` abbreviates `crates/ryo/crates/language/src`, and `RL` abbreviates `crates/ryo/crates/relations/src`. I stripped doc comments except where you asked for them. Counts from `git grep -c` are matching lines, not occurrences.

# PART 1 (T) evaluator

**E/finish.rs: cell and table types**
```
E/finish.rs:101  pub enum Ty { Node, Sym, File, Span, Num, Flag }   // derives Clone,Copy,PartialEq,Eq,PartialOrd,Ord,Hash,Debug,Serialize,Deserialize
E/finish.rs:110  pub fn ty_of(k: CellKind) -> Ty
E/finish.rs:123  pub struct V(Ty2, u64);   // Copy; pub enum Ty2 { T(u8) } at :125
E/finish.rs:129  pub fn new(t: Ty, v: u64) -> Self        // impl V
E/finish.rs:132  pub fn ty(self) -> Ty
E/finish.rs:138  pub const NULL: u64 = u64::MAX;
E/finish.rs:140  pub const NULL_V: V = V(Ty2::T(5), NULL);
E/finish.rs:143  pub type Names = Rc<Vec<String>>;
E/finish.rs:144  pub type Tys = Rc<Vec<Ty>>;
E/finish.rs:148  pub struct Table { pub names: Names, pub tys: Tys, pub cols: Vec<Vec<u64>>, pub n: usize }   // Default,Clone,Debug,PartialEq,Eq,Hash
```

**`Table` methods (`impl Table`, finish.rs:217)**
```
:218 pub fn retained(names: Names, tys: Tys, rows: usize) -> Self
:223 pub fn with(names: impl Into<Names>, tys: impl Into<Tys>) -> Self
:227 pub fn with_rows(names: impl Into<Names>, tys: impl Into<Tys>, n: usize) -> Self
:239 pub fn copy(&self) -> Table
:252 pub fn push_const_col(&mut self, x: u64, n: usize)
:258 pub fn release(&mut self)
:276 pub fn col(&self, name: &str) -> Result<usize, crate::check_session::refusal::Refusal>
:279 pub fn push_row(&mut self, row: &[u64])
:288 pub fn row(&self, r: usize, out: &mut Vec<u64>)
:209 impl Drop for Table   // release() + recycle::give_cols
```

**Column pool and recycling**
```
E/finish.rs:163 pub const POOL_CELLS: usize = 1 << 22;
E/finish.rs:172 pub struct PoolBudget(usize);
E/finish.rs:175 pub fn for_workers(workers: usize) -> Self   // impl PoolBudget; Drop restores the limit
E/finish.rs:186 pub const CLASSES: usize = 20;
E/finish.rs:192 pub fn take_col(p: &mut ([Vec<Vec<u64>>; CLASSES], usize), n: usize) -> Vec<u64>
E/finish.rs:714 pub fn give_tables<K>(m: &mut FxHashMap<K, Rc<Table>>)
```
The pool (`POOL`) and the limit (`POOL_LIMIT`) are `thread_local!`, private to each worker thread.

**`recycle.rs` pub items**
```
:12 pub const KEEP: usize = 1 << 14;
:22 pub const ROW_CELLS: usize = 1 << 20;
:26 pub fn rows_with(fill: impl FnOnce(&mut Vec<u32>)) -> Rc<Vec<u32>>
:52 pub fn recycle_rows(mut r: Rc<Vec<u32>>)
:70 pub fn no_rows() -> Rc<Vec<u32>>
:76 pub fn take_cols(k: usize) -> Vec<Vec<u64>>
:89 pub fn give_cols(v: Vec<Vec<u64>>)
:104 pub fn rc(t: Table) -> Rc<Table>
:123 pub fn recycle(mut r: Rc<Table>)
```

**Provider and scope (finish.rs:785-803)**
```rust
pub enum Scope<'s> { File(u32), Files(&'s [u32]) }   // Clone,Copy,Debug
pub trait Provider {
    fn fill(&mut self, run: &mut Run, rel: &str, scope: Scope<'_>) -> Result<Table, crate::check_session::refusal::Refusal>;
    fn shared_fill(&mut self, run: &mut Run, rel: &str, scope: Scope<'_>) -> Result<Rc<Table>, crate::check_session::refusal::Refusal> { self.fill(run,rel,scope).map(recycle::rc) }
    fn fill_len(&mut self, _run: &mut Run, _rel: &str, _scope: Scope<'_>) -> Option<usize> { None }
    fn render(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String>;
}
```

**`File<'a>` (finish.rs:661)**

Fields, in declaration order:
```
keyed_sizes: FxHashMap<usize, usize>
owed_fills: Vec<bool>
version: &'a FileVersion<'a>
cold_text: cold_text::Dictionary
after_last_file: bool
project_inputs: Option<&'a population::Inputs>
bound_inputs: Option<&'a FxHashMap<String, Rc<Table>>>
loan: Option<&'a dyn fact_view::FileFacts>
borrowed: Vec<Option<fact_view::Fact<'a>>>
memo: Vec<Option<Res>>
vrows: cold_entries::Rows
vmat: FxHashMap<usize, Rc<Table>>
cursor_views: FxHashSet<usize>
index: FxHashMap<usize, inline_key::List<(inline_key::Key, Rc<Index>)>>
fills: FxHashMap<String, Rc<Table>>
lazies: FxHashMap<usize, Rc<Lazy>>
facts: Vec<Option<Rc<Table>>>
fact_ix: RefCell<FxHashMap<(u16, usize), Rc<Index>>>
known: Vec<u8>
touched: Vec<u32>
fact_read: Vec<Cell<bool>>
file_tests: RefCell<FxHashMap<(usize, filter_order::List), bool>>
```
`File` derefs to `FileVersion` (`E/finish.rs:805`). `FileVersion` (`E/finish/file_version.rs:6`) holds `table: &NodeTable, src: &[u8], file: u32, n: usize, own: usize, kind: &[u16], start: &[u32], end: &[u32], parent: &[u32], field: &[u16]`, plus lazy caches. Its constructor is `FileVersion::new(table: &'a NodeTable, src: &'a [u8], file: u32, text: Option<Vec<u32>>) -> Self`.

`File` methods (signatures only):
```
:721 pub fn fact(&self, slot: usize) -> Option<fact_view::Fact<'_>>
:728 pub fn filled(&self, relation: &str) -> Option<&Rc<Table>>
:812 pub fn population(&self,node:usize) -> Option<&Rc<population::Population>>
:822 pub fn remember(&mut self, node: usize, answer: Res)
:828 pub fn new<'a>(version: &'a FileVersion<'a>, nodes: usize) -> File<'a>
:862 pub fn has_text(run: &Run, k: u16) -> bool
:866 pub fn text_bytes(&self, p: u32) -> &[u8]
:872 pub fn elem_text(&self, run: &mut Run, p: u32) -> u32
:888 pub fn has_parent(&self, p: u32) -> bool
:891 pub fn toks(&self, run: &Run) -> Rc<Tokens>
:920 pub fn native_get(&self, run: &mut Run, nat: Native, row: u32, col: usize) -> u64
:964 pub fn token_row(t: &Tokens, src: u32, k: u32) -> bool
:973 pub fn native_rows_into(&self, run: &Run, nat: Native, out: &mut Vec<u32>)
:991 pub fn native_rows(&self, run: &Run, nat: Native) -> Vec<u32>
:1013 pub fn mark_read(&self, i: u16)
:1019 pub fn fact_list(&self, i: u16, col: usize) -> Rc<Index>
:1035 pub fn probes(nat: Native, col: usize) -> bool
:1039 pub fn native_probe(&self, run: &Run, nat: Native, col: usize, v: u64, out: &mut Vec<u32>)
```

**`Exec<'p>` (finish.rs:1220)**

Fields:
```
answer_layouts: FxHashMap<(usize, usize), Arc<answer::Layout>>
plan: &'p Plan
stats: Stats
made: Vec<bool>
schemas: Vec<Option<Rc<Schema>>>
views: Vec<Option<Option<Rc<ViewSpec>>>>
sels: Vec<Option<Rc<SelPlan>>>
select_preps: Vec<Option<Rc<select_kernel::Prep>>>
union_preps: Vec<Option<Rc<union_kernel::Prep>>>
antijoin_preps: Vec<Option<Rc<antijoin_kernel::Prep>>>
subject_scope: subject_scope::Scope
keyed_forms: Vec<Option<keyed::Form>>
band_shapes: FxHashMap<(usize, usize), Rc<band::Shape>>
leaves: emptyprop::Leaves
inner: f64
constant: Vec<Option<bool>>
run_memo: Vec<Option<Rc<Table>>>
closed_checked: Option<(Vec<usize>, Vec<bool>)>
emitted: FxHashSet<usize>
tally: band::Tally
u32_pool: Vec<Vec<u32>>
needs: Vec<Option<Rc<known::Need>>>
flat: known::Flat
empties: Vec<Option<Rc<Table>>>
scratch_rows: Vec<u32>
timing: bool
cold_compiler: cold_test::Compiler
```
`pub type R<T> = Result<T, crate::check_session::refusal::Refusal>;` is at `:1258`.

Every `pub fn` on `impl<'p> Exec<'p>` (starts at `:1278`). `File<'_>`, `Run`, `dyn Provider` and `node` are the common parameters:
```
:1279 pub fn new(plan: &'p Plan) -> Self
:1312 pub fn release_request(&mut self)
:1316 pub fn closed_inputs(&mut self)
:1322 pub fn is_constant(&mut self, run: &Run, node: usize) -> bool
:1341 pub fn arg_rel(&self, node: usize, name: &str) -> R<usize>
:1348 pub fn schema(&mut self, run: &mut Run, node: usize) -> R<Rc<Schema>>
:1587 pub fn sel(&mut self, run: &mut Run, node: usize) -> R<Rc<SelPlan>>
:1632 pub fn view(&mut self, run: &mut Run, node: usize) -> R<Option<Rc<ViewSpec>>>
:1641 pub fn view_slow(&mut self, run: &mut Run, node: usize) -> R<Option<Rc<ViewSpec>>>
:1685 pub fn res(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Res>
:1696 pub fn res_body(...same params) -> R<Res>
:1716 pub fn make(...same params) -> R<Res>
:1723 pub fn make_(...same params) -> R<Res>
:1767 pub fn timed_body(...same params) -> R<Rc<Table>>
:1785 pub fn probe(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<Rc<Table>>
:1796 pub fn probe_body(...same as probe) -> R<Rc<Table>>
:1855 pub fn closure_at(...same as probe) -> R<Option<Table>>
:1876 pub fn eval(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>>
:1887 pub fn eval_body(...same params) -> R<Rc<Table>>
:1924 pub fn take_u32(&mut self) -> Vec<u32>
:1928 pub fn give_u32(&mut self, mut v: Vec<u32>)
:1937 pub fn empty_rc(&mut self, run: &mut Run, node: usize) -> R<Rc<Table>>
:1947 pub fn empty(&mut self, run: &mut Run, node: usize) -> R<Table>
:1952 pub fn empty_rows(&mut self, run: &mut Run, node: usize, n: usize) -> R<Table>
:1957 pub fn lazy(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Lazy>>
:1978 pub fn body(...same params) -> R<Rc<Table>>
:2003 pub fn counted(...same params) -> R<Option<Table>>
:2041 pub fn step(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table>
:2438 pub fn load_fact(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, nat: Native) -> R<()>
:2468 pub fn base(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: &str) -> R<Table>
:2491 pub fn antijoin(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &Args) -> R<Table>
:2545 pub fn src_of(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Src>
:2559 pub fn select(...same as step) -> R<Table>
:3007 pub fn axis(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &Args) -> R<Table>
:3013 pub fn hier(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: usize) -> R<Hier>
:3040 pub fn dom(...same as step) -> R<Dom>
:3055 pub fn admit(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, dom: &Dom, xs: &[u64]) -> R<FxHashSet<u64>>
:3075 pub fn axis_over(&mut self, f, run, p, node: usize, a: &Args, from: &Table) -> R<Table>
:3205 pub fn ordered(&mut self, f, run, p, node: usize, a: &Args, as_of: bool, incomplete: Option<&BTreeSet<Option<u32>>>) -> R<Table>
:3316 pub fn source_matches(&mut self, f, run, p, node: usize, a: &Args) -> R<Table>
```
`...same params` means `(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize)`.

**Plan form, schema, view, stats**
```
E/finish.rs:485  pub struct PNode { pub args: std::sync::OnceLock<Args>, pub inputs: plan_load::StepInputs }
E/finish.rs:490  pub struct Plan { pub frames: OnceLock<Vec<frame::Frame>>, pub consumers: OnceLock<Vec<Vec<usize>>>, pub nodes: Vec<PNode>, pub emits: Vec<(String, usize, String, Vec<String>)>, pub every_row: bool }
E/finish.rs:500  pub fn frame(&self, node: usize) -> frame::Frame        // impl Plan
E/finish.rs:504  pub fn consumers(&self, node: usize) -> &[usize]
E/finish.rs:1126 pub enum E { Col(usize, usize, Ty), Lit(V), Text(u32), Num(i64), List(Vec<E>), Call(String, Vec<E>), Cmp(Op, Box<E>, Box<E>), In(Box<E>, Box<E>, bool), And(Box<E>, Box<E>), Or(Box<E>, Box<E>), Not(Box<E>), TextTest(Box<E>, TextTest) }
E/finish.rs:1144 pub enum Op { Eq, Ne, Lt, Le, Gt, Ge }       // comparison operator, not the plan's operator name
E/finish.rs:1153 pub struct Schema { pub names: Names, pub tys: Tys }
E/finish.rs:1160 pub struct ViewSpec { pub base: Native, pub map: Vec<usize>, pub filter: Vec<E>, pub tests: Vec<view_cols::Test>, pub cold_test: OnceCell<Rc<cold_test::Kernel>>, pub cold_bucket: OnceCell<cold_list::Handoff>, pub cold_file_test: bool }
E/finish.rs:1172 pub fn new(run: &Run, base: Native, map: Vec<usize>, filter: Vec<E>) -> Self    // impl ViewSpec
E/finish.rs:1118 pub struct Stats { pub total_runs: u64, pub total_rows: u64 }
E/finish.rs:1181 pub enum Res { Tab(Rc<Table>), View(Rc<ViewSpec>) }
E/finish.rs:1195 pub enum Src { Tab(Rc<Table>, usize), View(Rc<ViewSpec>, usize), Base(Native), Lazy(Rc<Lazy>) }
E/finish.rs:540  pub enum Native { Tree(Extent), Child(Extent), Location, Token, Fact(u16) }
E/finish.rs:531  pub enum Extent { Own, Whole }
E/finish/plan_args.rs:9  pub enum Param { Missing, Text(String), Number(PlanNumber), List(Vec<Param>), Named(BTreeMap<String, Param>), CaptureColumns(Vec<Option<usize>>) }
E/finish/plan_args.rs:17 pub type Args = Param;
E/finish/plan_load.rs:26 pub struct StepInputs { pub graph: Arc<IndexedSteps>, pub slot: usize }
E/finish/plan_load.rs:19 pub struct IndexedSteps { pub plan: Arc<FinishPlan>, pub offsets: Vec<usize>, pub operands: Vec<usize> }
```
`Param` accessors (`plan_args.rs`): `text`, `strings`, `list`, `as_str`, `as_array`, `as_object`, `as_i64`, `as_u64`, `get(&self, k: &str) -> Option<&Self>`, `from_constant`, and `Index<&str>` and `Index<usize>` impls. Free function: `s(v: &Param) -> R<&str>` at `:90`.

`PNode` accessors (`plan_load.rs:214-273`):
```
op(&self) -> &str
relation_input(&self, name: &str) -> Option<usize>
summary_text(&self) -> String
step(&self) -> &FinishStep
base(&self) -> Option<&str>
semi(&self) -> Option<usize>
distinct_count(&self) -> bool
counted(&self) -> bool
tested(&self) -> bool
exists(&self) -> bool
args(&self) -> &Args
```
`args()` builds `Args` from `parameters(step: &FinishStep) -> Args` (`:275`) on first use, memoised in a `OnceLock`.

**1b. `E/finish/fact_view.rs`**
```
:9   pub struct ColumnCache { pub cells: Vec<OnceCell<Vec<u64>>>, pub reads: Vec<Cell<u32>> }   // new(columns: usize) -> Self
:21  pub enum Column<'a> { Owned(&'a [u64]), Numbers(&'a [CellWord]), Nodes(&'a [CellWord]), Words(&'a [CellWord], &'a [u32], &'a FxHashMap<u32,u32>, u32) }   // Clone,Copy
:30  pub fn at(self, row: usize) -> u64
:46  pub fn extend_into(self, out: &mut Vec<u64>, n: usize)
:59  pub fn translated(self) -> Vec<u64>
:65  pub fn len(self) -> usize
:74  pub enum Columns<'a> { Owned(&'a [Vec<u64>]), Lent(&'a [FilledColumn], &'a [CellKind], &'a [u32], &'a FxHashMap<u32,u32>, u32, &'a ColumnCache) }
:82  pub fn cell(self, column: usize, row: usize) -> u64
:102 pub fn raw(self, column: usize) -> Column<'a>
:117 pub fn get(self, column: usize) -> Column<'a>
:133 pub fn cached<'a>(cache: &'a ColumnCache, column: usize, lent: Column<'a>) -> Column<'a>
:150 pub enum Fact<'a> { Owned(&'a Table), Lent { rows: usize, columns: Columns<'a>, kinds: &'a [CellKind] } }
:156 pub fn len(self) -> usize
:160 pub fn cell(self, column: usize, row: usize) -> u64
:163 pub fn columns(self) -> Columns<'a>
:166 pub fn ty(self, column: usize) -> Ty
:172 pub trait FileFacts {
         fn bind(&self, relation: crate::languages::Relation, run: &mut super::Run) -> Result<Option<Fact<'_>>, crate::check_session::refusal::Refusal>;
     }
```
A lent cell is translated into the executor's words on read, in `Column::at`. `Columns::cell` and `Columns::get` switch to a cached translated column once reads reach about `rows/8` and `rows/4` respectively.

**1c. `E/reader_run/packet.rs`**
```
:76 pub struct OperatorColumns<'file, 'd> { pub file: FileColumns<'file>, pub lazy: Option<&'file Lazy<'file, 'd>>, pub dictionary: Option<&'file Dictionary<'d>>, pub caches: Vec<OnceCell<finish::fact_view::ColumnCache>>, pub own: RefCell<Vec<u32>>, pub symbols: Vec<OnceCell<Vec<u32>>>, pub public: Rc<FxHashMap<u32,u32>>, pub absent: u32 }
:89 impl finish::fact_view::FileFacts for OperatorColumns<'_, '_>   // bind(): answers Fact::Lent only when the answer is FactAnswer::Answered and wanted == Wanted::Whole, else Ok(None)
:459 pub struct View<'a, 'words, 'd> { pub renderer: Option<&'a mut dyn FnMut(l::RenderRequest) -> Result<l::RenderOutcome, String>>, pub packet: &'a Packet, pub local_words: Option<&'a Dictionary<'d>>, pub lazy: Option<&'a Lazy<'a, 'd>>, pub local_symbols: FxHashMap<u64, u32>, pub parsed: &'a mut reader::ReaderFacts<'words>, pub symbols: &'a l::Symbols, pub bytes: &'a [u8], pub resolved: FileSymbols, pub materialized: FxHashMap<l::Relation, Rc<finish::Table>> }
:471 impl finish::Provider for View<'_, '_, '_>
```
`View` defines `render` at `:472`, `fill_len` at `:487`, `fill` at `:504` and `shared_fill` at `:513`.
- `fill` is `self.shared_fill(..).map(|t| t.copy())`.
- `shared_fill` calls `layout()` and then `transpose()` once per relation, and caches the result in `materialized`.
- `View` also has the inherent methods `layout` (`:530`) and `transpose` (`:556`).

**1d. `from_typed`**
```
E/finish/plan_load.rs:51  pub fn from_typed(plan: Arc<FinishPlan>) -> Plan
```
It is re-exported at `finish.rs:20`. `FinishPlan` is `ryo_values::domain::finish_plans::types::FinishPlan { pub steps: Vec<FinishStep>, pub rules: Vec<PlannedRule>, pub unplanned: Vec<UnplannedRule> }` (`crates/ryo/crates/values/src/domain/finish_plans/types.rs:33`). `from_typed` fills `Plan.nodes` (one `PNode` per step), `emits` and `every_row`. `recycle.rs` pub items are listed above.

**1e. Counts in `finish.rs` plus `finish/` (T, `git grep -c`, summed matching lines)**
```
.cols[              139
.cols.               36
push_row(            41
.row(                22
.col(                52
.copy()               8
push_const_col(       2
extend_from_slice(   26
.cols (any, \.cols\b) 179
```
Five files with the most `.cols` lines:
- `finish.rs`: 75
- `finish/select_kernel.rs`: 16
- `finish/ordered_dfs.rs`: 9
- `finish/antijoin_kernel.rs`: 9
- `finish/worklist.rs`: 8

`finish/session/demand.rs` follows with 6. The counts include test files such as `cold_test.rs`.

**1f. Kernels that read a `NodeTable` or a lent column directly**
- **`native_get`** (`finish.rs:920-962`): one cell per call.
  - Tree and location columns come from `self.kind`, `self.start`, `self.end` and `self.parent`.
  - `ordinal_of` and `field_of` are read straight from `self.table`.
  - `Native::Fact(i)` reads `self.fact(i).cell(c, r)`, which is a lent provider column when the fact is `Fact::Lent`.
  - It also reads `run.kind_sym`, `run.field_sym` and `toks()`.
- **`finish.rs:1019` `fact_list`**: builds an `Index` from `t.columns().get(col)` via `Index::build_column`.
- **`finish.rs:988` and `:1009`**: `native_rows_into` and `native_rows` take `fact(i).len()`.
- **`finish/view_cols.rs`**:
  - `cell` (`:55`) reads `f.fact(i).cell`, `f.kind[row]` and `f.parent[row]`.
  - `retain_original` (`:102`, uses at `:108-124`) reads `col.at(r)`.
  - `gather_ids` (`:141`, uses at `:144-156`) reads `col.at` and `f.kind[r]`.
- **`finish/cold_bucket.rs`**: `Bucket::rows` (`:16`) calls `ids_of_field` and `ids_of_kind` on the `NodeTable`. `Part::rows` (`:30`), `Merge::write` (`:42`) and `Merge::read` (`:60`) merge those bucket slices. `select` (`:171`) takes `f.cut(bucket.rows(f.table), extent)`.
- **`finish/cold_list.rs:208-210`** and **`finish/emptyprop.rs:291`**: `ids_of_field` and `ids_of_kind` on `f.table`.
- **`finish/cold_pred.rs`**: `f.kind[...]` at `:22` and `:49`, `f.field[...]` at `:25` and `:51`.
- **`finish/select_kernel.rs:204`**: `f.kind[row]`.
- **`finish/walk.rs`**: `walk_axis` (`:18`) reads `node_count(f.table)` at `:45` and `subtree_end_of(f.table, ..)` at `:62`, `:83` and `:169`.
- **`finish/answer.rs:216-218`**: `own_end`, `implied_of` and `span_of`.
- **`finish/view_choice.rs:84`**: `ordinal_of`.
- **`finish/view_entry.rs:63`**: `ids_of_kind`.
- **`finish/session.rs:554-555`**: `starts` and `ends`.
- **`finish/fact_view.rs`**: `Column::at` (`:30`) and `Columns::cell` (`:82`) are the lent-column reads.

# PART 2 (T) what a language hands over

**`L/domain/facts/types.rs`**
```rust
:109 pub trait FactSource {
    fn fill(&mut self, relation: Relation, wanted: Wanted, words: &mut FileWords, out: &mut FilledColumns) -> FactAnswer;
    fn render(&mut self, request: RenderRequest) -> RenderOutcome;
    fn completeness(&self) -> FileCompleteness { FileCompleteness { syntax: SyntaxCompleteness::Complete, lexical: SyntaxCompleteness::Complete } }
    fn fills_are_complete(&mut self) -> bool { false }
    fn notes(&mut self, _table: &NodeTable, _source: ByteSlice<'_>, _out: &mut Bytes) {}
    fn named(&mut self, request: NamedRequest<'_, '_>) -> Vec<Result<FileRows, ParseError>> { request.relations.iter().map(|_| Err(ParseError::Unsupported)).collect() }
}
:354 pub enum Wanted { Whole, Key(Column, CellWord) }   // Clone,Copy,Debug,PartialEq,Eq
:301 pub enum FactAnswer { Answered(FillExtent), Partial(PartialRows), Refused(AbsenceCause) }
:371 pub enum FillExtent { Asked, Whole }
:322 pub struct PartialRows { pub cause: FillerSentence, pub missing: Missing }
:265 pub struct FilledColumns(pub Vec<FilledColumn>);   // Debug,Default
:277 pub struct FilledColumn(pub Vec<CellWord>);
:289 pub struct CellWord(pub u64);
:223 pub struct FileWords(pub(crate) WordsEngine);        // fields private: base: BaseWords, own: WordSet
:193 pub struct BaseWords(pub(crate) Arc<BaseSet>);       // BaseSet { words: WordSet, fixed: Count }; Clone,Debug,Default
:35  pub struct FileFacts<'f> { pub table: &'f NodeTable, pub source: ByteSlice<'f>, pub file: FileId, pub path: SourcePath<'f>, pub facts: &'f mut dyn FactSource }
:69  pub struct ProjectFacts<'p> { pub facts: &'p mut dyn FactSource }
:145 pub struct NamedRequest<'a, 'w> { pub table: &'a NodeTable, pub relations: &'a [&'a FileRelation], pub request: ParseRequest<'a>, pub project: Option<&'a dyn FileProject>, pub words: &'w mut FileWords, pub symbols: &'w mut Symbols }
```
I did not read the method signatures of `FileWords` and `BaseWords`. Their service functions are in `L/domain/facts/service.rs` (not read). The call sites in `packet.rs` use `l::facts::text(words, CellWord) -> text`, `base_count(words)`, `base_changed(words, &mut ..)` and `words_end(words)`.

**`L/domain/node_table/types.rs`**
- `:31 pub struct NodeTable(pub(crate) NodeTableEngine);` is opaque.
- Engine fields (all `pub(crate)`):
  ```
  visible: Option<usize>
  kind: Vec<u16>
  parent: Vec<u32>
  subtree_end: Vec<Position>
  start: Vec<u32>
  end: Vec<u32>
  field: Vec<u16>
  ordinal: Vec<ChildOrdinal>
  child_range: Vec<ChildRange>
  children: Vec<Position>
  by_kind: Vec<Vec<Position>>
  by_field: Vec<Vec<Position>>
  present: KindsPresent
  nodes: Count
  decoded: Vec<DecodedProperty>
  properties: Vec<DecodedProperty>
  texts: Bytes
  diagnostics: Diagnostics
  acceptance: SyntaxAcceptance
  embedded: Vec<EmbeddedSegment>
  implied: Vec<Position>
  sets: Vec<ElementSets>
  enclosing_sets: Vec<KindSet>
  enclosing: Vec<Vec<u32>>
  ```
- `:287 pub struct NodeTableBuilder<'a>(pub(crate) BuilderState<'a>);`
- Small types: `FieldCode(pub u16)`, `ByteSpan { pub start: Offset, pub end: Offset }`, `Implied { No, Yes }`, `EmbeddedRoot(pub Position)`, `EmbeddedText<'a> { pub text: NameText<'a>, pub mapping: EmbeddedMapping }`, `EmbeddedSpan { pub host: Position, pub root: Position, pub end: Position }`, `KindSet(pub [u64; 4])`.
- Layout rule from the module doc: syntax nodes at ids `0..nodes` in pre-order, then tokens in source order, then embedded segments.

**`L/domain/node_table/service.rs`**: free functions, not methods.

Read side (`table: &NodeTable`):
```
:945  kind_of(table, id: Position) -> KindBits
:965  parent_of(table, id: Position) -> Position
:989  span_of(table, id: Position) -> ByteSpan
:1021 field_of(table, id: Position) -> FieldCode
:1045 ordinal_of(table, id: Position) -> ChildOrdinal
:1070 subtree_end_of(table, id: Position) -> Position
:1095 children_of(table, id: Position) -> &[Position]
:1125 ids_of_kind(table, kind: KindBits) -> &[Position]
:1155 ids_of_field(table, field: FieldCode) -> &[Position]
:1183 kind_codes(table) -> &[u16]
:1203 field_codes(table) -> &[u16]
:1225 parent_ids(table) -> &[u32]
:1245 starts(table) -> &[u32]
:1265 ends(table) -> &[u32]
:1285 kinds_present(table) -> &KindsPresent
:1306 node_count(table) -> Count
:1327 element_count(table) -> Count
:1360 decoded_of(table, id: Position) -> Option<NameText<'_>>
:1384 property_of(table, id: Position, property: PropertyId) -> Option<NameText<'_>>
:1413 implied_of(table, id: Position) -> Implied
:1434 acceptance_of(table) -> SyntaxAcceptance
:1451 syntax_error_count(table) -> Count
:1481 own_end(table) -> Position
:1513 is_token(table, id: Position) -> bool
:1550 embedded_segments(table) -> impl Iterator<Item = EmbeddedSpan> + '_
```
Write side (`builder: &mut NodeTableBuilder<'_>`):
```
:43   builder(table: &mut NodeTable, kinds: Count) -> NodeTableBuilder<'_>
:97   structured<'t>(table: &'t mut NodeTable, kinds: Count, enclosing: &[KindSet]) -> NodeTableBuilder<'t>
:295  enter_node(builder, kind: KindBits, field: FieldCode, start: Offset)
:344  enter_implied_node(builder, kind: KindBits, field: FieldCode)
:353  enter_unlocated_node(builder, kind: KindBits, field: FieldCode)
:381  leave_node(builder, end: Offset)
:496  token(builder, kind: KindBits, field: FieldCode, start: Offset, end: Offset)
:523  decoded_token(builder, kind, field, start, end, decoded: NameText<'_>)
:619  node_property(builder, property: PropertyId, value: NameText<'_>)
:653  node_property_fmt(builder, property: PropertyId, value: std::fmt::Arguments<'_>) -> std::fmt::Result
:705  diagnose(builder, start: Offset, end: Offset, message: NameText<'_>)
:743  diagnose_with_metadata(builder, start, end, message, metadata: DiagnosticMetadata)
:774  set_syntax_acceptance(builder, acceptance: SyntaxAcceptance)
:828  embed(builder, host: Position, source: EmbeddedText<'_>) -> EmbeddedRoot
:862  finished<'b>(builder: &'b mut NodeTableBuilder<'_>) -> &'b NodeTable
:914  classify_written_fields(builder, classify: impl Fn(&NodeTable, Position) -> FieldCode)
:1560 classify_fields(table: &mut NodeTable, classify: impl Fn(&NodeTable, Position) -> FieldCode)
```
`L/inbound/use_cases/parse/named.rs`
```rust
:37  pub trait FileProject { fn roots(&self) -> Option<&[String]>; fn entry(&self, path: &std::path::Path) -> crate::domain::project_context::types::ObservedEntryState; }
:45  pub struct FileRows { pub columns: Vec<FactColumn>, pub partial: Option<crate::domain::reasons::types::ToolMessage> }
:29  pub struct FileRelation { pub name: &'static str, pub columns: &'static [(&'static str, CellKind)] }
:53  pub struct NumberedFile<'a> { pub tree: &'a NodeTable, pub words: &'a [Symbol], pub fact_words: &'a mut FileWords, pub fields: &'a FieldNames, pub syntax: SyntaxCompleteness, pub lexical: SyntaxCompleteness, pub notes: &'a Bytes, pub facts: &'a mut dyn NumberedFacts }
:74  pub trait NumberedFacts {
         fn fill_columns(&mut self, relation: Relation, wanted: Wanted, words: &mut FileWords, columns: &mut FilledColumns) -> FactAnswer;
         fn project_structure(&mut self, relations: &[Relation], symbols: &mut Symbols, out: &mut dyn RowWriter) -> Result<(), super::errors::ParseError>;
         fn named(&mut self, relations: &[&FileRelation], symbols: &mut Symbols) -> Vec<Result<FileRows, super::errors::ParseError>>;
         fn fills_are_complete(&mut self) -> bool { false }
         fn render(&mut self, request: RenderRequest) -> Result<RenderOutcome, super::errors::ParseError>;
     }
:99  pub trait FileRelations: Send + Sync {
         fn uses_project(&self) -> bool { false }
         fn numbered(&self) -> &dyn NumberedVisit;
         fn declarations(&self) -> &'static [FileRelation];
         fn read_covered(&self, relations: &[&FileRelation], request: super::types::ParseRequest<'_>, elements: &Elements, tree: &NodeTable, symbols: &mut Symbols, project: Option<&dyn FileProject>) -> Vec<Result<FileRows, super::errors::ParseError>>   // default body wraps read()
         fn read(&self, relations: &[&FileRelation], request: super::types::ParseRequest<'_>, elements: &Elements, tree: &NodeTable, symbols: &mut Symbols, project: Option<&dyn FileProject>) -> Vec<Result<Vec<FactColumn>, super::errors::ParseError>>;
     }
:139 pub trait NumberedVisit: Send + Sync {
         fn fields(&self) -> Option<&FieldNames> { None }
         fn projections(&self) -> &'static [Relation] { &[] }
         fn open(&self) -> Box<dyn NumberedReader + '_>;
     }
:155 pub trait NumberedReader {
         fn explicit(&mut self, _yes: bool) {}
         fn visit(&mut self, request: super::types::ParseRequest<'_>, catalog: &[Relation], named: &[&FileRelation], project: Option<&dyn FileProject>, symbols: &mut Symbols, consume: &mut dyn FnMut(NumberedFile<'_>, &mut Symbols)) -> Result<(), super::errors::ParseError>;
     }
```
In the quoted lines I shortened the full paths of the types in these signatures.

The task listed `NumberedVisit`, `NumberedReader`, `NumberedFacts`, `FileRelations` and `FileProject`. All five are above.

**`L/inbound/use_cases/walk/types.rs`**
```rust
:30  pub trait FileVisit { fn visit(&mut self, file: FileFacts<'_>); }
:49  pub struct WalkRequest<'a> { pub path: SourcePath<'a>, pub file: FileId, pub source: ByteSlice<'a>, pub explicit: bool }   // Clone,Copy,Debug
:71  pub struct RelationNeeds<'a>(pub &'a [Relation]);
:98  pub trait WalkEngine: Send + Sync {
        fn kind_count(&self) -> Count;  fn vocabulary(&self) -> WalkVocabulary;
        fn recognizes(&self, path: SourcePath<'_>, explicit: bool) -> Recognition;
        fn skips(&self) -> &'static [SkippedDirectory];  fn configuration_identity(&self) -> ParseConfigurationIdentity<'_>;
        fn capabilities(&self) -> Vec<OptionalCapability>;  fn unknown_reasons(&self) -> &'static [UnknownName];
        fn source_words(&self) -> Option<SourceWords>;  fn includes_expansions(&self, relation: Relation) -> bool;
        fn named_declarations(&self) -> &'static [FileRelation];  fn uses_project(&self) -> bool;
        fn spells(&self, kind: KindBits) -> bool;  fn absence_of(&self, relation: Relation) -> Option<AbsenceCause>;
        fn buffers(&self) -> Box<dyn std::any::Any + Send>;
        fn walk_file(&self, request: WalkRequest<'_>, needs: RelationNeeds<'_>, table: &mut NodeTable, buffers: &mut (dyn std::any::Any + Send), visit: &mut dyn FileVisit) -> Result<(), WalkError>;
    }
L/inbound/use_cases/parse/types.rs:123  pub enum FieldRequest { Wanted, Unwanted }   // Clone,Copy,Debug,PartialEq,Eq; it is a field of ParseRequest (:26)
```
**`RL/domain/relations/types.rs`**
```rust
:177 pub enum Value { Node(NodeId), Symbol(Symbol), Name(Name), File(FileId), Span(Span), Number(Integer), Flag(Flag) }   // Copy; :195 const-asserts size_of::<Value>() == 16
:220 pub enum CellKind { Node, Symbol, File, Span, Number, Flag }
:277 pub trait Rows<Row> { fn write(&mut self, row: Row); fn partial(&mut self, cause: ReportText<'_>, missing: Missing); fn partial_at(&mut self, file: FileId, cause: ReportText<'_>, missing: Missing); }
:261 pub trait RowWriter: $( $( Rows<$crate::...::relations::$module::item::$row> + )* )* {}   // generated
:237 macro_rules! catalog_types, expanded by crate::each_relation!(catalog_types) at :273; it also generates:
:243   pub enum Relation { $( $( $variant, )* )* }
:252   pub enum Column { $( $( $variant($crate::domain::relations::collections::relations::$module::item::$column), )* )* }
:266   impl<Writer> RowWriter for Writer where Writer: <all the Rows<..> bounds>
:358 pub struct FactColumn(pub Vec<u64>);
:363 pub struct ColumnsView<'a> { pub kinds: &'static [CellKind], pub columns: &'a [FactColumn] }   // Clone,Copy,Debug
:401 pub enum FactForm { NodeTable, Filled(FilledKey) }
:420 pub enum FilledKey { Unique(Column), Many(Column), Whole }
:431 pub enum NodeColumn { Id, Kind, Parent, Field, Ordinal, Start, End, Span, File, Text }
:466 pub enum NodeExtent { Own, Embedded }
:485 pub enum RelationSize { PerFile, PerElement, PerPair }
:292 pub enum Missing { Any, Kind(RowKind) }
```
The macro names `Relation` and `Column` per variant, but the variant list comes from `each_relation!`, which I did not read.

# PART 3 (O=f0219b3b3) designed operator contract

All paths below are under `crates/ryo/crates/operators/src`.

**`inbound/use_cases/plan/interface.rs:39`**
```rust
/// The physical plan of a whole program: each kind's one lowering, the rewrites its consumers' demand allows, pipelines broken
/// only at blocking kernels and shared nodes.
/// (doctests omitted) A kernel is private to operators: the engine cannot name one. Implementations must supply the planner.
pub trait PlanUseCase: Sync {
    /// The physical plan of `program`: every node lowered once.
    ///
    /// # Errors
    ///
    /// The shape fault of a step that is not a valid operation: `WrongOperation` for a node that reads a later node.
    fn plan(&self, program: &LogicalProgram<'_>) -> Result<PhysicalPlan, PortableOperatorFault>;
}
```
**`inbound/use_cases/run/interface.rs:32`**
```rust
/// Makes one node's rows for the open file, running only the pipelines it needs; its rows are then in `buffers`.
/// (doctests omitted) A kernel, its row map and what it borrows for a run are private to operators: the engine cannot name `text_map`.
pub trait RunUseCase: Sync {
    /// Makes `node`'s rows for the open file.
    ///
    /// # Errors
    ///
    /// The first fault in the interpreter's order (D15): the earliest input row that fails at any stage, at its first failing
    /// stage; a materialized input's pending fault after the rows before it.
    fn run(
        &self,
        plan: &PhysicalPlan,
        node: PlanNodeId,
        buffers: &mut FileBuffers,
        inputs: &dyn PlanInputs,
        resources: &mut dyn Resources,
    ) -> Result<(), PortableOperatorFault>;
}
```
**`domain/physical_plan/types.rs`**
```rust
:128 /// The rows a run reads besides its own nodes', which the engine holds for the open file.
pub trait PlanInputs {
    fn facts(&self, relation: Relation) -> FactRows<'_>;
    fn roots(&self, root: PlanNodeId) -> Candidates<'_>;
    fn frontier(&self, place: FrontierPlace) -> HeldRows<'_>;
    fn observation(&self, place: BoundObservationPlace) -> HeldRows<'_>;
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_>;
}
:142 /// What a kernel borrows from the engine besides rows: the complete inventory.
pub trait Resources {
    fn texts(&self) -> Texts<'_>;
    fn intern(&mut self, text: CellText<'_>) -> Value;
    fn mint(&mut self, text: CellText<'_>) -> Value;
    fn source(&self) -> OpenFileBytes<'_>;
    fn tree(&self) -> &Elements;
    fn render(&mut self, requests: RenderRequests<'_>) -> Result<RenderOutcomes, PortableOperatorFault>;
}
:45  pub struct PlanNodeId(pub u32);                       // Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord,Hash
:117 pub struct OpenFileBytes<'a>(pub &'a [u8]);
:121 pub struct RenderRequests<'a>(pub &'a [ryo_language::domain::semantics::types::RenderRequest]);
:125 pub struct RenderOutcomes(pub Vec<ryo_language::domain::semantics::types::RenderOutcome>);   // Clone,Debug,Default,PartialEq,Eq
:60  pub struct NodeFed(pub [RowCount; 3]);
:64  pub enum PlanInput { Node(PlanNodeId), Relation(Relation), Frontier(FrontierPlace), Observation(BoundObservationPlace) }
:86  pub struct PlanInputList<'p>(pub &'p [PlanInput]);   // :90 pub struct NodeBindings<'p>(pub &'p [ParameterBinding]);
:95  pub struct LogicalNode<'p> { pub operation: &'p Operation, pub inputs: PlanInputList<'p>, pub bindings: NodeBindings<'p>, pub scope: StepScope }
:108 pub enum NodeRows<'b> { Materialized(&'b Table), Absent }
```
**Named types**
```
.../physical_plan/types/fact_rows/value.rs:7        pub enum FactRows<'a> { Lent(ColumnsView<'a>), Refused }   // Clone,Copy,Debug; pub const fn lent(self) -> Option<ColumnsView<'a>>
.../held_rows/value.rs:8                            pub enum HeldRows<'a> { Held(&'a Table), Withheld(WithheldInput) }   // pub const fn table(self) -> Result<&'a Table, WithheldInput>
.../candidates/value.rs:7                           pub struct Candidates<'a>(pub &'a [NodeId]);   // pub const fn nodes(self) -> &'a [NodeId]
.../frontier_place/value.rs:10                      pub struct FrontierPlace { pub rule: RuleOrdinal, pub input: InputNumber }   // const fn of(rule, input)
.../bound_observation_place/value.rs:8              pub struct BoundObservationPlace { pub rule: RuleOrdinal, pub input: ObservationBindingId }   // const fn of
crates/ryo/crates/values/src/domain/plan_parts/types.rs:148   pub struct HostedNumber(pub u32);
crates/ryo/crates/values/src/domain/row_texts/types.rs:36    pub struct CellText<'a>(pub &'a str);
domain/table_texts/types.rs:45                      pub struct Texts<'a> { pub symbols: &'a Symbols, pub source: SourceView<'a>, pub paths: &'a [FilePath], pub arenas: &'a [ArenaBytes] }   // Clone,Copy,Debug
```
`WithheldInput(pub u32)` is at `domain/contract/types.rs:6`.

`PhysicalPlan` (`.../physical_plan/value.rs:16`), all fields `pub(crate)`. Public methods are `with_layouts(self, Vec<Option<Vec<CellKind>>>) -> Self`, `node_count(&self) -> NodeTotal` and `node_kind(&self, PlanNodeId) -> Option<NodeKind>`.
```
entity_queries: Arc<QueryPlan>
nodes: Vec<PhysicalNode>
indexes: Vec<KeyedIndex>
layouts: Vec<Option<Vec<CellKind>>>
guards: Vec<Box<[u32]>>
partitions: SelectPartitions
```
`FileBuffers` (`.../file_buffers/value.rs:31`, `Debug, Default`). All fields are `pub(crate)`. I read lines 30-115, and the struct continues after `origin`.
```
entity_queries: EntityFileSession
nodes: Vec<Option<ryo_values::domain::rows::types::Table>>
pending: Vec<Option<PortableOperatorFault>>
fed: Vec<NodeFed>
remaining: Vec<NodeReaders>
engine_left: Vec<EngineUses>
done: ConsumersDone
pool: KeptPool<Table>
pools: IndexPools
decisions: Vec<NodeDecisions>
index: KeyedRows
row: Vec<Value>
written: Vec<Value>
values: Vec<Value>
scratch: RowScratch
places: RangeScratch
hashes: GroupHashes
second: IndexRows
walk: WalkScratch
cycles: CycleScratch
repeats: RowSet
found: Vec<NodeId>
shared: SharedIndexes
touched: TouchedNodes
stream: StreamScratch
worklist: WorklistScratch
counters: Option<NodeCounters>
known_empty: Vec<u32>
known_full: Vec<u32>
generation: u32
partitions: FilePartitions
origin: Vec<u32>
```
`LogicalProgram<'p>` (`.../logical_program/value.rs:18`, private fields) has `nodes: Vec<LogicalNode<'p>>`, `emits: Vec<Emitted>` and `holds: Vec<PlanNodeId>`.
- Public methods are `new() -> Self`, `node(&mut self, LogicalNode<'p>) -> PlanNodeId`, `emit(&mut self, RuleOrdinal, PlanNodeId)`, `hold(&mut self, PlanNodeId)` and `nodes(&self) -> &[LogicalNode<'p>]`.
- `check_shape` is `pub(crate)`. It fails with `WrongOperation` if a node reads a node that is not earlier.

`PortableOperatorFault` (`domain/contract/types.rs:26`, `Clone, Debug, PartialEq, Eq`):
```
Withheld(WithheldInput)
Interrupted
NoSuchColumn { index: ColumnIndex }
ShapeMismatch
DuplicateOrder
NotALattice
InvalidSpanBounds
InvalidSpan
InvalidSourceMatchSpan
MissingCapture { capture: LiteralText }
WrongOperation
UnboundKey
UnsupportedRender
WrongBinding
Unbound
NotRun
WrongTest
Derived(ryo_derived::domain::fault::types::DerivedFault)
RenderRefused { refusal: RenderRefusal }
Pattern(ryo_matching::inbound::use_cases::matching::errors::MatchingError)
```
Values at O (`crates/ryo/crates/values/src/domain/rows/types.rs`):
```
:4 pub struct RowIndex(pub usize);
:7 pub struct RowCount(pub usize);
:10 pub struct Table { pub kinds: Vec<CellKind>, pub cells: Vec<Value>, pub count: RowCount }   // Clone,Debug,Default,PartialEq,Eq
```
`rows/service.rs` free functions: `column_in`, `cell`, `row_in(&Table, RowIndex) -> Option<&[Value]>`, `reset(&mut Table, &[CellKind])`, `push_row(&mut Table, &[Value])` and `positions(RowCount)`.

# FACTS a contract must respect

1. At T a table is column-major `Vec<Vec<u64>>` plus `n`. A cell is a bare `u64` typed by a parallel `Vec<Ty>`, and `Ty::Sym` cells are run-level symbol ids. At O a table is row-major `Vec<Value>` (16 bytes) plus `kinds`, so the two layouts differ.
2. Per step, `Exec::step(f, run, p, node)` returns an owned `Table`. `eval` and `eval_body` return `Rc<Table>`, memoised per file in `File::memo` (`Res::Tab` or `Res::View`). `Provider::fill` is called per (relation, scope) and cached in `File::fills`, and `shared_fill` is cached in `View::materialized`.
3. Per file, `File::new(version, nodes)` reuses thread-local spare buffers. A file's tables return to a per-thread pool of at most `POOL_CELLS` cells (`PoolBudget::for_workers` splits it), so tables must be released on the worker thread that made them.
4. Per cell, `native_get` and `Fact::cell` or `Column::at` are called one cell at a time. `Columns::cell` promotes a lent column to a translated `Vec<u64>` after about `rows/8` reads. Lent `Words` cells translate through a per-file word map, and lent `Nodes` cells map `NULL` to `NULL`.
5. Per row, tree and view kernels index `f.kind[r]`, `f.parent[r]`, `f.start[r]`, `f.end[r]` and `f.field[r]` directly, and pull `ids_of_kind`, `ids_of_field`, `subtree_end_of` and `own_end` from the `NodeTable`, without building a `Table`.
6. The language side is pull by relation per file: `FactSource::fill(relation, Wanted::{Whole|Key(Column, CellWord)}, &mut FileWords, &mut FilledColumns) -> FactAnswer`. A lent `Fact` is bound only if the answer is `Answered` and `wanted == Whole` (`OperatorColumns::bind`). The walk is push-style: `WalkEngine::walk_file` calls `FileVisit::visit(FileFacts)` once per file with `&mut dyn FactSource`.
7. At O `RunUseCase::run` is per (plan, node, file): it takes `&mut FileBuffers` and returns rows into `buffers`, with the earliest-failing-row fault order (D15) and a `pending` fault after earlier rows. `PlanInputs` is read-only and returns `Refused` or `Withheld(token)` instead of failing. `Resources` is `&mut` and is the only route to intern, mint, render and source bytes.
8. At O `PlanUseCase::plan` is once per program and fails only with `WrongOperation` when a node reads a node that is not earlier. `Plan` at T is built once per request by `from_typed(Arc<FinishPlan>)`, with `args()` memoised per node in a `OnceLock`.
