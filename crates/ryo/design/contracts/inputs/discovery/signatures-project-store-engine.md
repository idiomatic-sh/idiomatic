<!-- a read-only reader's final report, 27 to 28 Nov 2026; task ae962dc9c5c7e8168; copied from the session transcript -->

# Interface contracts for the cross-file phase, the kept store and the designed engine

Revisions: T=2a207464c, O=f0219b3b3, B=ba657e79b. X = `crates/ryo/crates/executor/src`. I read everything below through `git show` and `git grep`.

Three things I did not read or quote in full:
- The bodies of `finish/session/demand.rs` and `contribution.rs`. I read only their pub item lists.
- Pub fn bodies in general. For long signatures I give the parameter types as read, and I mark where I abbreviate.
- Doc comments. I stripped them everywhere except Part 4, where you asked for them on the three traits and `EnginePort`.

## PART 1: cross-file phase at T

**X/check_session/frontier.rs**
```rust
:10  pub type ColumnDemand<'a> = BTreeMap<&'a str, Vec<bool>>;
:12  pub struct FileInputs<'a> { pub columns: &'a ColumnDemand<'a>, pub carried_bases: HashMap<String, Rc<Block>>, pub produced: HashMap<usize, Rc<Block>> }   // one field per line in source
:18  pub fn new(columns: &'a ColumnDemand<'a>) -> Self
:22  pub fn column_demand(active: &[(usize, usize, bool)], bound: &super::Bindings<'_>, programs: &'a [crate::check_session::refusal::Planned], program_of: &[usize], cut_of: &[Option<Arc<Partition>>]) -> ColumnDemand<'a>
:61  pub fn contribute(&mut self, b: &mut Bound<'_>, exec: &mut finish::Exec<'_>, frame: &mut finish::File<'_>, run: &mut Run, id: u32, facts_view: &mut dyn finish::Provider, cut: &Partition, local_coverage: &BTreeMap<String, Vec<Option<u32>>>, world: &mut World, carried_symbols: &mut rustc_hash::FxHashMap<u32, u32>, reader: Option<usize>, words: &HashMap<u32, u32>, slot: usize) -> Result<()>   // :61-76, one param per line in source
:186 pub type Part = (usize, Rc<Block>);
:187 pub type EmptyFrontier = Rc<[Part]>;
:188 pub enum FileFrontier { Rows(Box<[Part]>), Empty(EmptyFrontier) }
:193 pub fn parts(&self) -> &[Part]                      // on FileFrontier
:204 pub struct World {   // #[derive(Default)]
       pub run: Run,
       pub boundaries: Option<Vec<(usize, usize, usize)>>,
       pub project_words: Vec<HashMap<u32, u32>>,
       pub files: HashMap<String, u32>,
       pub frontiers: HashMap<usize, FileFrontier>,
       pub empty_blocks: HashMap<(usize, usize), Rc<finish::population::Block>>,
       pub unavailable: HashMap<usize, BTreeSet<(String, u32)>> }
:214 pub struct SharedFrontier<'a> { pub slots: HashMap<usize, Vec<(usize, usize)>>, pub blocks: Vec<finish::population::SharedBlock<'a>> }
:219 pub fn new(world: &'a World, pairs: impl Iterator<Item = usize>) -> Self
:240 pub fn joined(&self, slots: &[usize], cut: &[usize], local: &mut HashMap<usize, Rc<finish::population::WorkerBlock>>, schemas: &mut finish::population::LocalSchemas) -> Result<finish::population::Inputs>
:267 pub fn frontier_parts(&self, slot: usize) -> Option<&[Part]>        // impl World from here
:270 pub fn restore_frontier(&mut self, slot: usize, parts: Vec<Part>)
:274 pub fn file(&mut self, path: &str) -> u32
:284 pub fn empty_block(&mut self, t: &Table) -> Rc<Block>
:290 pub fn insert_empty(&mut self, slot: usize, path: &str, binding: &mut Bound<'_>, run: &mut Run, cut: &Partition)
:301 pub fn productive(&self, slot: usize) -> bool
:304 pub fn carry(&mut self, source: &Run, home: u32, t: &Table, needed: &[bool], syms: &mut rustc_hash::FxHashMap<u32, u32>, reader: Option<usize>, words: &HashMap<u32, u32>) -> Result<Rc<finish::population::Block>>
:407 pub fn joined(&self, slots: &[usize], cut: &[usize]) -> Result<finish::population::Inputs>
:429 pub fn release(&mut self, slot: usize)       // frontiers.remove + unavailable.remove
:437 pub struct Transfer { pub slots: Vec<(usize, Vec<(usize, usize)>)>, pub blocks: Vec<finish::population::CarriedBlock>, pub schemas: finish::population::CarriedSchemas, pub paths: Vec<String>, pub words: Vec<Vec<u8>>, pub prefix: usize, pub boundaries: Vec<(usize, usize, usize)>, pub unavailable: HashMap<usize, BTreeSet<(String, u32)>> }
:449 pub fn transfer(mut self, prefix: usize) -> Transfer
:498 pub fn receive(&mut self, parts: Vec<Transfer>, sources: &[super::Source])
:531 pub fn receive_blocks(&mut self, part: Transfer, files: &[u32], symbols: &[u32])
```

**X/check_session/project.rs** (`Population` here is NOT `finish::population::Population`)
```rust
:21  pub struct Layout { pub relations: Vec<l::Relation>, pub parts: Vec<Part>, pub words: Vec<Box<[u8]>>, pub visit: Option<Visit> }
:29  pub struct Part { pub columns: Vec<(String, finish::Ty)>, pub rows: std::collections::HashMap<String, Vec<u64>>, pub in_visit: bool }
:38  pub struct Visit { pub request: l::ProjectRequest, pub authority: Authority, pub environment: Option<std::sync::Arc<Environment>> }
:45  pub struct FileRead { pub tables: Vec<(String, Rc<Table>)>, pub partials: Vec<(l::Relation, String, ryo_relations::domain::relations::types::Missing)> }
:52  pub fn layout<L: super::FileSource>(readers: &[L], authority: &Authority, inventory: &[String], environment: Option<std::sync::Arc<Environment>>, relations: &[l::Relation]) -> Option<Layout>
:124 pub fn declare(&self, run: &mut finish::Run, prefix: &str) -> Vec<(l::Relation, String)>
:134 pub fn witness_columns(&self, part: usize, path: &str, file: l::FileId, words: &mut l::FileWords, out: &mut l::FilledColumns) -> bool
:159 pub fn slice(&self, run: &mut finish::Run, aliases: &[(l::Relation, String)], path: &str, file: l::FileId, read: Option<&FileRead>) -> Vec<(String, Rc<Table>)>
:185 pub fn reads_in_visit(&self) -> bool
:196 pub fn read_file<L: super::FileSource>(&self, reader: Option<&L>, public: &std::collections::HashMap<u32, u32>, run: &mut finish::Run, aliases: &[(l::Relation, String)], path: &str, file: l::FileId, notes: &l::Bytes) -> Result<FileRead>
:277 pub struct Contribution { pub reader: Option<usize>, pub notes: l::Bytes, pub manifest: Option<l::ManifestText> }   // Clone, Default
:284 pub enum Authority { Given, Disk(String), Scoped { disk: Option<String>, roots: Vec<String> } }   // Clone, Debug, PartialEq, Eq, Hash, Deserialize
:291 pub fn declare_words<L: super::FileSource>(&mut self, readers: &[L])    // impl super::World
:301 pub fn disk(&self) -> Option<&str>     :309 pub fn with_roots(self, roots: Vec<String>) -> Self
:313 pub fn observe(&self, files: &[l::FilePath], path: l::FilePath) -> EntryObservation
:342 pub struct Population { pub tables: BTreeMap<String, Rc<Table>>, pub partials: Vec<Partial>, pub refused: Vec<l::Refused>, pub consulted: Vec<l::FilePath>, pub observations: Vec<EntryObservation> }
:403 pub fn collect<L: super::FileSource>(world: &mut super::World, readers: &[L], authority: &Authority, inventory: &[String], environment: Option<&Environment>, contributions: &[Rc<Contribution>], wanted: &[l::Relation]) -> Result<Population>
:583 pub struct Environment { pub id: std::sync::Arc<str>, pub authority: Authority, pub version_scope: Option<[u8; 32]>, pub supplied: BTreeSet<String>, pub epoch: std::sync::Arc<std::sync::atomic::AtomicUsize>, pub observations: std::sync::Mutex<(usize, BTreeMap<std::path::PathBuf, l::ObservedEntryState>)> }
:595 pub fn new(id: std::sync::Arc<str>, authority: Authority, supplied: BTreeSet<String>, epoch: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> Self
:603 pub fn observed(&self, mut read: impl FnMut(&Path, l::ObservedEntryState))     :615 pub fn physical(&self, path: &Path) -> l::ObservedEntryState
```

**X/finish/population.rs** (`Names`, `Tys`, `R`, `Table` and `Ty` come from `super`, line 3. I did not read the definitions of `Names` and `Tys`.)
```rust
:12  pub struct SharedWords<'a>(Vec<(&'a [u8], bool)>);       :15 pub fn share_words(&self) -> SharedWords<'_>   // impl super::Run
:25  pub fn install(&self, run: &mut super::Run)
:40  pub enum Words { Constant(u64), Sequence(u64), Byte(u64, Box<[u8]>), Short(u64, Box<[u16]>), Int(u64, Box<[u32]>), Full(Box<[u64]>), Stored(StoredWords) }
:52  pub type BlockBytes = Arc<dyn AsRef<[u8]> + Send + Sync>;
:54  pub struct StoredValues { pub bytes: BlockBytes, pub dictionary: Vec<u32>, pub absent: Option<u32> }
:61  pub struct StoredWords { pub values: Arc<StoredValues>, pub start: u32, pub width: u8, pub base: u64, pub ty: Ty }
:69  pub fn at(&self, i: usize) -> u64   :86 pub fn translate(&self, ty: Ty, word: u64) -> u64
:104 pub fn pack(v: std::borrow::Cow<'_, [u64]>) -> Self   :127 pub fn at(&self, i: usize) -> u64   :139 pub fn append(&self, n: usize, to: &mut Vec<u64>)
:153 pub enum Posting { Ordered, Sorted(Box<[usize]>) }
:158 pub struct ColumnBlock<C> { pub names: Names, pub tys: Tys, pub n: usize, pub home: u32, pub columns: C, pub postings: RefCell<FxHashMap<usize, Posting>> }
:167 pub type Block = ColumnBlock<Vec<Words>>;      :169 pub type WorkerBlock = ColumnBlock<Arc<[Words]>>;
:172 pub struct SharedBlock<'a> { pub block: Arc<[Words]>, pub schema: (usize, usize), pub names: &'a [String], pub types: &'a [Ty], pub len: usize, pub home: u32 }
:181 pub fn local(&self, schemas: &mut LocalSchemas) -> Rc<WorkerBlock>          :197 pub struct LocalSchemas(FxHashMap<(usize, usize), (Names, Tys)>);
:199 pub enum FixedColumn { Constant(u64), Sequence(u64), Values { base: u64, width: u8 } }
:206 pub fn constant(&self, column: usize) -> Option<u64>     :212 pub fn share(&self) -> SharedBlock<'_>     :223 pub fn words(&self,col:usize)->Vec<u64>
:228 pub fn new(home: u32, names: Names, tys: Tys, n: usize, columns: Vec<Words>) -> Self
:240 pub fn append_fixed(&self, column: usize, out: &mut Vec<u8>) -> Option<FixedColumn>
:265 pub fn stored(home: u32, names: Names, tys: Tys, n: usize, columns: Vec<Result<StoredWords, u64>>) -> Self
:288 pub fn find(&self, col: usize, key: u64, mut hit: impl FnMut(usize) -> bool) -> bool   // impl<C: AsRef<[Words]>> ColumnBlock<C>
:341 pub enum Piece { File(Rc<Block>), Worker(Rc<WorkerBlock>) }                  :347 columns(&self) -> &[Words]   :353 file(&self) -> u32   :359 len(&self) -> usize   :365 find(&self, col: usize, key: u64, hit: impl FnMut(usize) -> bool) -> bool
:374 pub struct Population { pub schema: Option<(Names, Tys)>, pub pieces: Vec<Piece>, pub by_file: OnceCell<FxHashMap<u32, Vec<usize>>>, pub offsets: OnceCell<Vec<u32>>, pub n: usize, pub local_files: OnceCell<Vec<bool>>, pub materialized: OnceCell<Rc<Table>> }   // Default
:384 pub type Inputs = std::collections::BTreeMap<usize, Rc<Population>>;
:387 add(&mut self, block: &Rc<Block>) -> R<()>   :391 add_worker(&mut self, block: &Rc<WorkerBlock>) -> R<()>
:395 add_piece<C>(&mut self, table: &ColumnBlock<C>, piece: impl FnOnce() -> Piece) -> R<()>
:409 agrees(&self, names: &Names, tys: &Tys) -> bool   :415 full(&self, names: &Names, tys: &Tys) -> Rc<Table>   :430 same_rows(&self, other: &Self) -> bool   :444 cells(&self) -> usize
:449 matching_rows(&self, col: usize, key: u64, hits: &mut Vec<u32>)   :462 is_materialized(&self, t: &Rc<Table>) -> bool   :468 can_probe(&self, col: usize) -> bool
:490 find(&self, col: usize, key: u64, mut visit: impl FnMut(usize, &Piece, usize) -> bool) -> bool   :529 each_match(&self, col: usize, key: u64, mut each: impl FnMut(usize, &Piece, usize))
:536 contains_keys(&self, key: usize, left: &Table, row: usize, lk: &[usize], rk: &[usize]) -> bool   :551 neighbors(&self, by: usize, to: usize, key: u64, out: &mut Vec<u64>)   :556 probe(&self, names: &Names, tys: &Tys, col: usize, keys: &[u64]) -> Rc<Table>
:572 pub struct CarriedBlock { pub schema: usize, pub rows: usize, pub home: u32, pub columns: Vec<Words> }
:582 pub struct CarriedSchemas { pub ids: FxHashMap<u64, Vec<usize>>, pub schemas: Vec<(Vec<String>, Vec<Ty>)> }   // Default
:589 content_hash(names: &[String], types: &[Ty]) -> u64   :596 intern(&mut self, names: Names, types: Tys) -> usize   :610 receive(self) -> Vec<(Names, Tys)>
:618 pub fn carry(self, shapes: &mut CarriedSchemas) -> CarriedBlock   // impl Block
:629 pub fn receive(mut self, files: &[u32], symbols: &[u32], schemas: &[(Names, Tys)]) -> Block   // impl CarriedBlock
```

**X/finish/partition.rs** (all fns return `R<..>`; I read the heads only)
```rust
:8   pub fn with_file_slice(exec: &mut Exec<'_>, run: &mut Run, relations: &[&str], wide: impl Fn(&str) -> bool, ...)   // params after `wide` not read
:36  #[derive(Clone, Default)] pub struct Partition { pub domains: BTreeMap<usize, (usize, super::session::demand::Nodes)>, pub ancestor_domains: BTreeMap<usize, usize>, pub reach: BTreeMap<usize, super::session::demand::Nodes>, pub inputs: Vec<usize>, pub columns: BTreeMap<usize, Vec<bool>>, pub filters: BTreeMap<usize, Vec<(usize, Vec<u8>)>>, pub local_emit: bool, pub file_steps: Vec<bool> }
:54  pub fn file_steps(exec: &mut Exec<'_>, run: &mut Run, wide: impl Fn(&str) -> bool, sliced: impl Fn(&str) -> bool) -> R<Vec<bool>>
:63  pub fn local_steps(exec: &mut Exec<'_>, run: &mut Run, wide: impl Fn(&str) -> bool, sliced: impl Fn(&str) -> bool, anchor: impl Fn(Ty) -> bool) -> R<Vec<bool>>
:99  pub fn slice_readers_bind_file(exec: &mut Exec<'_>, run: &mut Run, base: usize, local: &[bool]) -> R<bool>
:154 pub fn file_steps_by(exec: &mut Exec<'_>, run: &mut Run, wide: &impl Fn(&str) -> bool, anchor: &impl Fn(Ty) -> bool, sliced: &impl Fn(usize, &str) -> bool) -> R<Vec<bool>>
:174 pub fn classify(exec: &mut Exec<'_>, run: &mut Run, wide: &impl Fn(&str) -> bool, anchor: &impl Fn(Ty) -> bool, sliced: &impl Fn(usize, &str) -> bool, local_class: Option<usize>) -> R<Vec<bool>>
:328 pub fn partition(exec: &mut Exec<'_>, run: &mut Run, local: Vec<bool>) -> R<Partition>
:506 pub fn mark_columns(expr: &E, alias: usize, used: &mut [bool])
```

**X/finish/session.rs and session/\*** (all in `finish::session`)
```rust
session.rs:12  pub fn scalar(expr: &E) -> R<()>
:33  pub fn evaluate_covered(exec: &mut Exec<'_>, run: &mut Run, provider: &mut dyn Provider, admitted: &[u32], admit_file_subject: impl Fn(u32) -> bool, carried: &Inputs, incomplete: &BTreeMap<String, Vec<Option<u32>>>, mut memo: Option<(&mut reuse::Reuse, [usize; 3])>) -> R<Vec<answer::Row>>
:285 pub fn keep_finding(seen: &mut BTreeMap<Vec<answer::Value>, answer::Row>, row: answer::Row)
:302 pub fn absence_input(step: &super::PNode) -> Option<(&'static str, &'static str)>
:311 pub fn restrict_uncertain(exec: &mut Exec<'_>, run: &mut Run, provider: &mut dyn Provider, frame: &mut File<'_>, demanded: &[bool], inputs: &BTreeMap<String, Vec<Option<u32>>>) -> R<()>
:398 pub fn frame<'a, 't>(version: &'a FileVersion<'a>, plan: &super::Plan, supplied: &'a super::FxHashMap<String, Rc<Table>>, loan: Option<&'a dyn super::fact_view::FileFacts>, local: impl IntoIterator<Item = &'t (String, Rc<Table>)>) -> File<'a>
:412 pub fn pair(exec: &mut Exec<'_>, frame: &mut File<'_>, run: &mut Run, facts: &mut dyn Provider, plan: &super::Plan, binding: usize, slots: &[usize], incomplete: &BTreeMap<String, Vec<Option<u32>>>) -> R<Vec<answer::Row>>
:442 pub enum CompleteBase   :447 covered_empty(..)   :457 reachable_partials(..)   :473 proven_empty(..)   // multi-line, all (exec, frame, run, facts: &mut dyn Provider, plan, slots, ...) -> bool / Option<..>
:523 pub fn publish_table(exec: &mut Exec<'_>, run: &mut Run, slot: (usize, usize), rule: &str, subject: &str, keys: &[String], table: &Table, file: Option<usize>, every_row: bool, rows: &mut Vec<answer::Row>, admit: impl Fn(usize, usize) -> R<bool>) -> R<()>
:546 pub fn publish_locations(.. columns: &[crate::pair_summary::LocationColumn], mask: &[u64], tree: &crate::languages::NodeTable, file: u32, every_row: bool, rows: &mut Vec<answer::Row>) -> R<()>
:573 pub fn gather(exec: &mut Exec<'_>, run: &mut Run, frame: &mut File<'_>, facts: &mut dyn Provider, slots: &[usize], cut: &[usize], ancestor_domains: &BTreeMap<usize, usize>, projections: &BTreeMap<usize,contribution::Projection>, incomplete: &BTreeMap<String, Vec<Option<u32>>>) -> R<Vec<(usize, Rc<Table>, bool)>>
certificate.rs:8 pub fn total(expr: &E) -> bool ; :27 pub fn empty_frontier(exec: &mut Exec<'_>, run: &mut Run, steps: &[usize]) -> R<Vec<(usize, Rc<Table>)>>
contribution.rs:13 pub enum Keys ; :18 pub struct Projection { demand: Option<Keys>, source: usize, tree: Rc<ViewSpec>, schema: Rc<Schema>, columns: Vec<(usize, Native, usize)>, rows: Rows, edge: Option<Native>, share_with_local_reader: bool }   // all pub; one field is on a line I did not print
:31 pub enum Rows ; :50 pub struct Output { rows: Rows, columns: Vec<(usize, Native, usize)>, names: Rc<Vec<String>>, types: Vec<u8>, mask: Vec<bool> } ; :59 pub struct Registry(HashMap<Output, usize>);
:62 identify(&mut self, plans: &BTreeMap<usize, Projection>, cut: Option<&super::super::partition::Partition>) -> HashMap<usize, usize>   :89 bind(exec, run, cut, tests) -> BTreeMap<usize, Self>   :118 prepare(..) -> Option<Self>   :321 gather(&self, slots: &[usize], file: &mut File<'_>, run: &mut Run, keys: Option<&[u32]>) -> Option<Table>
:379 pub fn demanded(plans, node, slots, exec, file, run, facts) -> R<Option<Vec<u32>>> ; :395 pub fn gather(plans, node, slots, file, run, keys) -> Option<Rc<Table>>
demand.rs: pub fn local_class:11, ancestor_domains:93, enum Nodes:140, union:149, steps:172, ancestor_direction:181, origin:191, reach_domains:299, walk_domains:358, reach_nodes:408, nodes_of:436, native_child:485, ancestor_edges:496, domains:541   // names and lines only, signatures not read
reuse.rs:5-8 pub const HOST/INVENTORY/VARIABLE/RESTORED: u8 = 1/2/4/8 ; :10 pub type Key = (usize, u8, [usize; 3]); :13 pub struct Reuse { pub enabled: bool, pub rows: FxHashMap<Key, Rc<Table>>, pub restored: usize, pub empty_emits: usize }
:21 new(requests: usize) -> Self ; :24 get(&mut self, node: usize, deps: u8, mut scope: [usize; 3]) -> Option<Rc<Table>> ; :31 remember(&mut self, deps: &[u8], scope: [usize; 3], tables: &[Option<Res>]) ; :47 pub fn empty(step: &PNode, zero: &[bool]) -> bool ; :67-69 posting(), capture(), take_counts() -> (usize, usize)
```

**X/check_session/replay/closed_workers.rs and readers.rs**
```rust
closed_workers.rs:5  pub enum Inputs { Withheld(session::Reported), Ready { carried: finish::population::Inputs, incomplete: BTreeMap<String, Vec<Option<u32>>>, unknowns: Vec<(String, String)>, skipped: Vec<String> } }
:18 pub fn absent_files(schedule: &session::Schedule, contract: &session::completeness::Contract, has_cut: bool, root: &Arc<str>, files: &[String]) -> BTreeSet<String>
:40 pub fn accepted_files<'a>(files: &'a [String], rejected: &[String]) -> std::borrow::Cow<'a, [String]>
:45 pub fn inputs(run: &finish::Run, contract: &session::completeness::Contract, unavailable: &HashMap<usize, BTreeSet<(String, u32)>>, pairs: &[usize], inventory: &[String], files: &[String], missing: &BTreeSet<String>, cut: Option<&finish::partition::Partition>, p: Option<&Population>, answers: &ReportSlots, frontier: impl FnOnce(&[usize], &[usize]) -> Result<finish::population::Inputs, crate::check_session::refusal::Refusal>) -> Result<Inputs, crate::check_session::refusal::Refusal>
:141 pub struct SharedTable<'a> { pub name: &'a str, pub names: &'a [String], pub types: &'a [finish::Ty], pub columns: &'a [Vec<u64>], pub rows: usize }
:149 pub struct SharedPopulation<'a> { pub tables: Vec<SharedTable<'a>>, pub partials: &'a [ryo_execution_domain::domain::coverage::types::Partial] }   :154 from(p: &'a Population) -> Self   :169 local(&self) -> Population
:188 pub struct Job<'a> { pub slot: usize, pub program: usize, pub project: Option<usize>, pub pairs: &'a [usize], pub inventory: &'a [String], pub inventory_key: usize, pub files: &'a [String], pub absent: BTreeSet<String>, pub hosted: &'a [Hosted], pub contract: &'a session::completeness::Contract, pub cut: Option<&'a finish::partition::Partition> }
:202 pub fn run(count: usize, batch: &mut Batch, world: &mut session::World, populations: &HashMap<usize, (BTreeSet<l::Relation>, Result<Population, crate::check_session::refusal::Refusal>)>, answers: &ReportSlots, groups: impl Iterator<Item = Vec<usize>>, readers: &mut readers::Closed, reader_states: &[session::ReaderState]) -> f64
readers.rs:11 pub struct PathLists<'a> ; :17 id(&mut self, paths: &'a Rc<Vec<String>>) -> usize
:25 pub struct Closed { pub scopes: HashMap<usize, [usize; 3]>, pub repeats: HashMap<[usize; 3], usize>, pub number_wall: f64, pub numbered: usize, pub numbers: Vec<Option<Vec<u32>>>, pub duplicate: Vec<Option<usize>>, pub reads: Vec<Option<usize>>, pub last: HashMap<usize, usize>, pub remaining: Vec<usize>, pub providers: HashMap<usize, usize>, pub shared: HashMap<usize, Inputs> }
:39 new(work: &[Work], cut_of: &[Option<Arc<Partition>>]) -> Self ; :116 scope(&self, request: usize) -> [usize; 3] ; :118 may_repeat(&self, scope: [usize; 3]) -> bool ; :122 inventory_id(&self, request: usize) -> usize ; :125 numbered_inventories(&self) -> &[Option<Vec<u32>>] ; :128 numbers(&mut self, request: usize, paths: &[String], world: &mut session::World) -> &[u32]
:142 inputs(&mut self, i: usize, make: impl FnOnce() -> Result<Inputs, crate::check_session::refusal::Refusal>) -> Result<Inputs, crate::check_session::refusal::Refusal>
:159 release(&mut self, i: usize, pairs: &[usize], project: Option<usize>, mut release_pair: impl FnMut(usize)) -> Option<usize>
:195 release_finished(&mut self, mut release_pair: impl FnMut(usize), mut release_population: impl FnMut(usize))
:249 pub struct Evaluated { pub entries: HashMap<[usize; 3], Vec<Evaluation>>, pub cells: usize, pub hits: usize } ; :255 pub struct Evaluation { pub project: Option<BTreeMap<String, Rc<finish::Table>>>, pub inputs: Inputs, pub answer: session::Reported }
:262 find(&mut self, scope: [usize; 3], project: Option<&super::Population>, inputs: &Inputs) -> Option<session::Reported> ; :280 remember(&mut self, scope: [usize; 3], project: Option<&super::Population>, inputs: &Inputs, answer: &session::Reported)
```

**Hand-off and release (one file to the cross-file phase)**

| What | Where |
|---|---|
| Per-file contribution into `world.frontiers.insert(slot, frontier)` | `X/check_session/frontier.rs:182`. It is called from `X/check_session.rs:1563` (`shared.contribute(b, &mut exec, current, &mut run, id.0, ...)`). |
| Carry of the file's tables into `World` | `World::carry` is called at `frontier.rs:139`. |
| Empty frontier inserted (file rejected, excluded or empty) | `X/check_session.rs:1166`, `:1526` -> `frontier.rs:299`. |
| Restored from the store instead of computed | `X/check_session/replay/stored.rs:1160` -> `world.restore_frontier`. |
| Worker thread to coordinator | `X/check_session/worker.rs:190` (`part.transfer(prefix)`) and `worker.rs:236` (`world.receive(transfers, sources)`), which ends at `frontier.rs:539`. |
| Cross-file input built from frontiers | `World::joined` at `frontier.rs:407`, called at `X/check_session/replay/batch.rs:808`. The worker path is `closed_workers.rs:286`, through `SharedFrontier::joined`. |
| Release, definition | `World::release` at `frontier.rs:429`. |
| Release, call sites | `X/check_session/replay/batch.rs:772-773` (`world.release(slot); answers.release(slot)`) and `batch.rs:864-866` (via `readers.release`). Also `batch.rs:~690` (`readers.release_finished`, closed workers). |
| Store consumes the frontier before release | `batch.rs:524/527/533` (`store.local` / `store.admit`) and `batch.rs:861` (`s.closed(...)`). The comment at `:863` says closed slots are consumed before release. |

## PART 2: answers and report at T

```rust
X/finish/answer.rs:12  pub use report::Reported;     :14 pub type Value = Option<Cell<Arc<str>>>;
:18  pub struct Words { pub symbols: FxHashMap<u32, Arc<str>>, pub paths: Vec<Arc<str>>, pub path_ids: FxHashMap<Arc<str>, u32> }   // Default
:24  pub struct Layout { pub rule: Box<str>, pub names: Box<[String]>, pub identity: Box<[usize]>, pub types: Box<[Ty]>, pub words: Arc<RwLock<Words>> }
:33  Layout::new(run: &mut Run, rule: &str, names: &[String], types: &[Ty], subject: &str, keys: &[String]) -> Result<Arc<Self>, crate::check_session::refusal::Refusal>
:60  answer_layout(&mut self, run: &mut Run, emit: (usize, usize), rule: &str, table: &Table, subject: &str, keys: &[String]) -> Result<Arc<Layout>, Refusal>   // Refusal = crate::check_session::refusal::Refusal
:78  pub fn location_layout(exec: &mut Exec<'_>, run: &mut Run, emit: (usize, usize), rule: &str, subject: &str, keys: &[String], columns: &[crate::pair_summary::LocationColumn], file: u32) -> Result<(Arc<Layout>, u32), Refusal>
:101 #[derive(Clone)] pub struct Row { pub layout: Arc<Layout>, pub words: Box<[u64]>, pub place: (u32, u32) }
:107 pub struct RestoredRows { pub dictionary: Arc<RwLock<Words>>, pub layouts: Vec<FxHashMap<Vec<Ty>, Arc<Layout>>> }
:112 RestoredRows::new(words: Vec<Box<str>>, schemas: usize) -> Self ; :123 row(&mut self, schema: usize, rule: &str, names: &[Box<str>], identity: &[usize], cells: impl Iterator<Item = (Ty, u64)>) -> Row
:138 Row::resolved(layout: &Arc<Layout>, words: Box<[u64]>) -> Self ; :142 decode<T: From<String> + From<&'static str>, D: std::ops::Deref<Target = Words>>(&self, index: usize, dictionary: impl FnOnce() -> D, text: impl FnOnce(&Arc<str>) -> T) -> Option<Cell<T>>
:168 byte_span(&self, column: &str) -> Result<Option<Span>, Refusal> ; :189 span_file(&self) -> Option<Arc<str>> ; :199 locate(&mut self, path: &str, tree: Option<&crate::languages::NodeTable>, bytes: &[u8], coordinates: &mut crate::languages::contract::FindingCoordinates) -> Result<(), Refusal>
:232 value(&self, index: usize) -> Value ; :236 cells_with<'a, T: From<String> + From<&'static str>>(&'a self, text: impl Fn(&Arc<str>) -> T + 'a) -> impl Iterator<Item = Option<Cell<T>>> + 'a ; :244 dictionary(&self) -> &RwLock<Words> ; :248 value_from(&self, index: usize, words: &Words) -> Value ; :252 width(&self) -> usize ; :256 cells(&self) -> impl Iterator<Item = Value> + '_
:261 pub fn unpack_span(word: u64) -> Span ; :269 symbol(&mut self, run: &Run, id: u32) -> Result<&Arc<str>, Refusal> ; :281 path(&mut self, text: &str) -> Result<u32, Refusal>
:293 pub fn typed(run: &Run, table: &Table, row: usize, file: Option<usize>, layout: &Arc<Layout>) -> Result<Row, Refusal>
X/finish/answer/report.rs:6  #[derive(Clone, Default)] pub struct Reported { pub rows: Box<[Row]>, pub issues: Option<Box<ReportIssues>> }   // Deref<Target = ReportIssues>
:12 pub struct ReportIssues { pub rejected_syntax: Vec<String>, pub unknowns: Vec<(String, String)>, pub unread: Vec<(String, String)> }
:33 Reported::new(rows: Vec<Row>, unknowns: Vec<(String, String)>, rejected_syntax: Vec<String>) -> Self ; :45 with_unread(mut self, unread: Vec<(String, String)>) -> Self ; :52 skipped(paths: &[String]) -> Self ; :58 extend_unknowns(&mut self, unknowns: Vec<(String, String)>)
X/check_session/report_slots.rs:8 pub static UNANSWERED_ANSWER: Answer ; :9 pub static EMPTY_ANSWER: LazyLock<Answer> ; :14 pub struct Page { pub empty: u64, pub payload: Option<Box<[u32; 64]>> } ; :20 pub struct ReportSlots { pub pages: Vec<Page>, pub len: usize, pub payloads: Vec<Answer>, pub unused: Vec<u32> }
:29 grow(&mut self, len: usize) ; :36 get(&self, slot: usize) -> Option<&Answer> ; :49 visit_pending<E>(&self, slots: std::ops::Range<usize>, mut visit: impl FnMut(usize) -> Result<(), E>) -> Result<(), E> ; :67 set(&mut self, slot: usize, answer: Answer) ; :107 release(&mut self, slot: usize) ; :111 clear(&mut self, slot: usize, marker: u32)
X/check_session/completeness.rs:16 pub struct Contract { pub cross: bool, pub frontier: bool, pub needs: Needs } ; :24 pub fn describe(program: &Program, local: &[bool]) -> Result<Contract> ; :65 pub fn report(&self, partials: &[Partial], path: &str, mut observe: impl FnMut(&'static str)) -> Vec<(String, String)>
```
- `set_empty` is called on `answers` at `check_session.rs:~1530`, but I did not read its definition.
- Answer rows reach output in these places: `Row::locate` at `check_session.rs:294` and `:1439`, `json_output::write_finding(dst: &mut Vec<u8>, row: &Row)` at `finish/json_output.rs:81`, and `ordered_rows` used at `replay.rs:125`. The step that turns `Row` into the SDK finding type is not read.

## PART 3: the kept store

**a. At T, `X/check_session/replay/stored.rs`**
```rust
:16 pub type Key = [u8; 32];     :35 pub fn key(value: impl Hash) -> Key     :17 pub type Res<T> = io::Result<T>;
:45 pub fn version_key(&self, effective: Key) -> Option<Key>   // impl session::Source
:367 pub struct Store {   // 55 fields, in groups
  location/build: dir, names, name_ids, names_digest, saved_names, previous, build
  tables:         files: Records<FileAnswers>, blocks: Records<EncodedBlock>, notes: Shards<Notes>, requests: Packets<Record>, groups: Shards<Group>, packets: Packets<Packet>, observed: Observations
  per-run maps:   work_inputs, pending_requests, updated_proofs, prior_groups, supplied_versions, versions, files_valid, notes_valid, pairs: Vec<Option<(usize, usize)>>, program_keys, program_inputs, bindings: Vec<Binding>, binding_table, binding_numbers, plans: Vec<Option<RequestPlan>>, observed_entries, environment_proofs, reused_context, keep_pairs, local_rows, project_proofs
  schema:         empty_schemas, schema_ids, empty_packet, empty_packet_reuses
  counters/CPU:   file_hits, closed_hits, evaluated_pairs, evaluated_closed, request_hits, absent_file, absent_local, absent_frontier, restored_blocks, frontier_cpu, packets_packed, packed_rows, rendered_packets, pack_cpu }
:54 pub struct Report { pub rows: Option<Vec<u8>>, pub issues: Option<Box<ReportIssues>> } ; :62 stored::ReportIssues { rejected_syntax, unknowns, unread }
:105 Notes { context: Option<Proof>, reader: Option<usize>, notes: Vec<u8>, manifest: Option<Box<str>> } ; :112 Local { report: Report, frontier: Vec<(usize, usize)>, unavailable: Vec<(String, String)> }
:139 pub enum BlockId { Empty(u32), Local(Key) } ; :156 pub struct Binding { pub key: Key, pub id: u32 } ; :185 pub struct FileAnswers { context: Option<Proof>, pairs: BTreeMap<Key, Local>, locations: BTreeMap<Key, Range<usize>>, mapped: Option<shards::MappedRecord>, blocks: Vec<BlockId>, known_blocks: HashMap<BlockId, usize>, complete: Vec<u8>, closed: Vec<u8> }   // all pub
:338 pub struct Snapshot { names, build, files, notes, requests, groups, packets, packet_layers, blocks, stamps, empty_schemas, request_layers, program_inputs, bindings }
:354 pub struct PriorClosed { pub work: usize, pub source: usize, pub version: Key, pub record: Record } ; :361 pub struct RequestPlan { pub proof: Vec<Proof>, pub unread: Vec<(String, String)>, pub environments: BTreeSet<Arc<str>> }
:333 pub use request_answers::{InputScope, Packet};
```
`Store` pub fns, signature only. **Bold** means the parameter is an engine type:
```rust
:442 open(root: &Path, readers: &[impl session::source::Source]) -> Res<Self>      :445 directory(dir: PathBuf, readers: &[impl session::source::Source]) -> Res<Self>
:541 publish(&mut self) -> Res<usize>     :611 program_inputs(&self, program: Key) -> Option<&[String]>     :615 remember_program_inputs(&mut self, program: Key, names: &[String])
:619 request_inputs(&mut self, root: &Path, inventory: Key, peers: Key, supplied: BTreeMap<String, Key>) -> InputScope
:629 bind_work(&mut self, work: usize, scope: InputScope)    :633 offer(&mut self, work: usize, id: Key)    :637 truncate_work(&mut self, len: usize)    :643 bind_version(&mut self, source: usize, version: Key)
:646 inventory(&mut self, i: usize, path: PathBuf, expected: Option<Key>)    :657 contract_file(&mut self, p: &Path) -> Option<Key>
:660 lookup(&mut self, id: Key) -> Option<(Key, &[(String, String)])>   :664 lookup_in(&mut self, scope: InputScope, id: Key) -> ...   :668 lookup_for(&mut self, scope: Option<InputScope>, id: Key) -> ...
:682 packet(&self, id: &Key) -> &Packet
:685 **save(&mut self, id: Key, idx: usize, r: &super::Receipt<'_>) -> Option<Key>**          // Receipt (engine type)
:717 **decline(&mut self, id: Key, reason: &crate::check_session::refusal::Refusal)**
:720 put(&mut self, id: Key, mut packet: Packet, proof: Vec<Proof>, unread: Vec<(String, String)>, render: impl FnOnce() -> String) -> Key
:738 record(&mut self, id: Key, packet: Key, proof: Vec<Proof>, unread: Vec<(String, String)>)    :749 group(&mut self, mut paths: Vec<(PathBuf, Probe)>, scope: Option<InputScope>) -> Option<Proof>
:780 **prepare(&mut self, b: &Batch)**                                                          // &Batch
:837 changed_record(&mut self, id: Key, scope: Option<InputScope>) -> Option<((PathBuf, Key), Record)>
:865 **probe(&mut self, b: &Batch) -> (Vec<usize>, Vec<PriorClosed>)**                         // &Batch
:919 **reuse_closed(&mut self, b: &mut Batch, candidates: Vec<PriorClosed>)**                  // &mut Batch
:955 same_frontier(&mut self, before: Key, after: Key, binding: Binding) -> bool
:982 **admit(&mut self, b: &mut Batch, world: &mut session::World, answers: &mut super::ReportSlots, contributions: &mut [Option<Rc<project::Contribution>>], pre_read: &[usize])**   // &mut Batch, &mut World
:1238 **local(&mut self, b: &Batch, world: &session::World, answers: &super::ReportSlots, aliases: &session::worker::PairAliases, contributions: &[Option<Rc<project::Contribution>>], pending: &[usize])**   // &Batch, &World
:1371 **closed(&mut self, idx: usize, value: &Answer, pop: Option<&Population>, project: Option<(usize, &Authority)>)**   // Answer, Population, Authority
:1409 **reuse_context(&mut self, environment: Option<&project::Environment>, proof: Option<Proof>) -> bool**
:1440 **remember_entries(&mut self, environment: &project::Environment)**   :1454 environment_proof(&mut self, environment: Arc<str>) -> Option<Proof>   :1472 **population_proof(&mut self, p: &Population, root: &str) -> Option<Proof>**
:1495 pub fn merge_entry(entries: &mut BTreeMap<PathBuf, Option<u8>>, path: PathBuf, state: Option<u8>)   :1502 pub fn entry_kind(state: l::ObservedEntryState) -> Option<u8>
```
Other pub types in the store files:
```rust
request_answers.rs:18 pub enum Probe { Source, Bytes, Kind, Effective, Inventory } ; :25 type Group = Vec<(u32, Probe)> ; :26 pub type Proof = (Key, Key);
:29 pub struct Stamp { pub device: u64, pub inode: u64, pub length: u64, pub mode: u32, pub modified: (i64, i64), pub changed: (i64, i64) } ; :56 SavedStamp { pub stamp: Stamp, pub bytes: Key } ; :61 FileObservation { pub bytes: Key, pub executable: u32, pub source: Option<Key>, pub stamp: Stamp }
:81 pub struct InputScope { pub overlay: usize, pub inventory: usize } ; :86 RequestInputs { pub root: PathBuf, pub supplied: BTreeMap<String, Key> } ; :91 pub struct Observations (fields not printed)
:390 pub struct Record { pub proof: Vec<Proof>, pub packet: Key, pub unread: Vec<(String, String)> }
:397 Schema { rule: Box<str>, columns: Vec<Box<str>>, identity: Vec<usize> } ; :403 PackedRow { place: (u32, u32), schema: usize, values: Vec<Option<PackedValue>> } ; :409 enum PackedValue { File(u32), Text(u32), Number(i64), Span(u32, u32) } ; :416 Rows<Word = Box<str>> { words, schemas, rows }
:423 pub struct Packet { pub rejected_syntax: Vec<String>, pub refusal: Option<crate::check_session::refusal::Refusal>, pub digest: String, pub unknowns: Vec<(String, String)>, pub data: Vec<u8> }
:432 declined(reason: &Refusal) -> Self ; :441 typed_bytes(&self) -> &[u8] ; :444 save(receipt: &Receipt<'_>) -> Self ; :453 report(&self) -> session::Reported
:458 pub fn pack_rows<'a>(input: impl IntoIterator<Item = &'a finish::answer::Row>) -> Vec<u8> ; :509 pub fn unpack_rows(data: &[u8]) -> Vec<finish::answer::Row>
:121 Observations::new(dir: PathBuf, index: super::Index) -> Self ; :178 observe_in(&mut self, p: &Path, mode: Probe, overlay: Option<usize>) -> Option<Key> ; :192 observe_kind(&mut self, path: &Path) -> Option<u8> ; :202 observe(&mut self, p: &Path, mode: Probe) -> Option<Key> ; :295 still_current(&self) -> bool ; :312 leaf(..) ; :332 prior_leaf(..) ; :365 group(..)
shards.rs: :12 pub type Index = BTreeMap<u8, Key>; Shards<T> :14 (new, coarse, shard, open, get, insert, dirty, flush(&mut self, dir: &Path, obsolete: &mut Vec<PathBuf>) -> Res<usize>) ; Packets<T> :103 (same plus index(&self) -> &Index) ; :207 Location = (u64, usize, Key); :208 Directory = BTreeMap<Key, Location>; :210 Segment; :216 Segments = BTreeMap<u8, Segment>; Records<T> :217 (new, part, open, open_mapped, mapped_record, get, change(&mut self, k: &Key) -> &mut T, dirty, flush) ; :410 pub struct MappedRecord { pub map: Arc<memmap2::Mmap>, pub range: Range<usize> }
stored_block.rs:9 pub enum Column { Constant(u64), Sequence(u64), Values { base: u64, width: u8 } } ; :16 Word<'a> { Path(&'a str), Symbol(&'a [u8]) } ; :24 WordId { Path(u32), Symbol(u32) } ; :30 Header<'a, Names = Vec<String>, Types = Vec<finish::Ty>> { home: &'a str, names, types, count: usize, words: Vec<Word<'a>>, columns: Vec<Column> }
:41 pub fn encode(block: &Block, world: &World) -> Vec<u8> ; :108 pub fn pack_column(column: &[u64], bytes: &mut Vec<u8>) -> Column ; :139 pub fn restore(bytes: BlockBytes, world: &mut World) -> Option<Rc<Block>>
```

How a rerun decides a kept answer is still valid:
- **Build key.** `stored.rs:449-466`. The key is a blake3 hash of the executable's build id (or of the whole executable if there is no build id), combined with each reader's `(name, identity())`. If `Snapshot.build` differs, the whole snapshot is dropped.
- **File stat and hash.** `request_answers.rs:202-293` (`Observations::observe`). A saved `Stamp` (device, inode, length, mode, mtime, ctime) is compared with the file's current stat at `:233`. A hit needs the stamps to match and the file to be older than the run start (`older_than(self.started)`, `:51`). Otherwise the file is read and hashed with blake3 at `:265`. The executable bit is folded into the `Probe::Source` digest (`:68-76`).
- **File version key.** `stored.rs:786-797`. It is `key((path, reader, permission, blake3 of the bytes))`. `version_key` at `:45` adds `environment.version_scope`.
- **Rule text.** `stored.rs:803-805`. `program_keys[idx] = key((text, params))`. `Binding.key` at `:813-821` is `key((1u8, rule, host))`, or `key((0u8, rule, shape))` when there is a cut.
- **Per-file validity.** A file's answers are looked up by version key plus binding key (`FileAnswers::read(binding)` at `:237`). The environment context proof must match (`context` compared at `:963`).
- **Request validity.** `Record.proof: Vec<Proof>`, where each `Proof` is `(Key, Key)`, is re-checked through `Observations::group` (`request_answers.rs:365`).
- **Refusal.** `Store::save` (`:686-689`) refuses to keep an answer whose `unread` list is non-empty.

**b. At B**
- `git merge-base 2a207464c ba657e79b` = `043698327d6e3729eb387479353857c3fe2775c9`. B is not a descendant of T.
- `git diff --stat T B -- crates/ryo`: 4839 files changed, +393256 / -10867. The top 25 lines are mostly `...lock` files (one-line additions), then `.config/nextest.toml +6`, `Cargo.lock +476`, `Cargo.toml`, `asymptote.toml`, `cli/Cargo.toml +13`, `cli/build.rs`, `cli/src/edge.rs`, `cli/src/languages.rs`, `languages_test.rs` (67 lines), `cli/src/lib.rs -3`, `outcome/remedy.rs`, `tastemaker.toml +65`.
- Store files that differ (`replay/stored.rs` 1665 -> 1748 lines, `stored/shards.rs` +12) are not smaller at B. `request_answers.rs` and `stored_block.rs` are identical.
- In `stored.rs`, B has the same `Store` fields and the same pub fns as T, with three differences:
  - B drops `unread` from `stored::ReportIssues` and from `Report::save` and `read`.
  - B drops the early `return None` in `Store::save` for non-empty `unread`.
  - B adds `pub fn costs(&self)` and `eprintln!` timing lines in `publish` and `reuse_closed`.
- In `shards.rs`, B adds this on `Shards<T>` or `Packets<T>`; I did not check which:
  ```rust
  pub fn change(&mut self, key: &Key) -> &mut T where T: Default
  ```
- In `check_session/project.rs`, B lacks `Visit`, `FileRead`, `Layout.visit`, `Part.in_visit`, `reads_in_visit` and `read_file`. B's `slice` is `slice(&self, run: &mut finish::Run, aliases: &[(l::Relation, String)], path: &str, file: l::FileId) -> Vec<(String, Rc<Table>)>`.
- I did not find a separate smaller store at B in `crates/ryo/crates/executor`. I did not search other crates.

**c. SDK cache**
```rust
crates/ryo/crates/sdk/src/resources/linter/lint/types.rs:77  #[non_exhaustive] pub enum Cache { None, At(PathBuf) }
```
- `lint/session/entry.rs:47-50` maps `Cache::At(path)` to `Some(path.as_path())` and everything else to `None`. That is passed to `session.use_cache(directory)` (`:296-299`).
- `use_cache` calls `replay::stored::Store::directory(p.to_owned(), &self.languages)` and stores it in `self.store: Option<replay::stored::Store>` (`entry.rs:97`).
- `session.publish()` runs at `entry.rs:58` after each round, which calls `Store::publish` (`:113-114`).
- `lint/walk.rs:32` also skips the store directory during the corpus walk when it is `Cache::At`.
- `Store::open` (`replay.rs:152`) is the other constructor, used by the CLI replay path.

## PART 4: designed engine contracts at O

The three traits, verbatim with doc comments (module `//!` headers omitted). Each is in `crates/ryo/crates/engine/src/inbound/use_cases/<load|scan|project>/interface.rs`.
```rust
load/interface.rs  (trait at :~23, fn at :~29)
/// Makes a rule set's programs.
/// # Examples  (doctest, 11 lines: `LoadUseCaseImpl::new().load(LoadSource::Plans(SelectedRules(&[]), StatisticsBytes(&[]), LoadGiven::default()), &kinds).is_ok()`)
pub trait LoadUseCase: Send + Sync {
    /// Lowers `Plans` as one pack, answering the programs of the rules it accepted and each rule it refused with the reason, or
    /// decodes `Pack`.
    ///
    /// # Errors
    ///
    /// `PackVersion`, `PackDigest` or `PackCorrupt` for a pack. A rule that cannot be loaded is not an error: it is in `refused`.
    fn load(
        &self,
        source: LoadSource<'_>,
        kinds: &KindVocabulary,
    ) -> Result<LoadedRules, LoadError>;
}
scan/interface.rs
/// Runs a rule set's programs on one file.
/// # Examples  (doctest: `ScanUseCaseImpl::new(std::sync::Arc::new(MatchingUseCaseImpl))`)
pub trait ScanUseCase: Send + Sync {
    /// Runs every enabled program over the file's node table and facts. Which file of the run it is, is the lent file's own
    /// `file`: a program writes it as a `File` cell and as the file half of a node's id.
    ///
    /// # Errors
    ///
    /// `Refused` or `Partial` when the language refuses or partly fills a relation a candidate needs: a fault of the rules that
    /// read it, never empty rows.
    fn scan(
        &self,
        programs: &Programs,
        file: FileFacts<'_>,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ScanError>;
}
project/interface.rs
/// Runs the cross-file rules once per run.
/// # Examples  (doctest: `ProjectUseCaseImpl::new()`)
pub trait ProjectUseCase: Send + Sync {
    /// Runs the project programs; their made tables are filled from `frontier` and `project`.
    ///
    /// # Errors
    ///
    /// `Refused` when the project refuses a relation a program needs.
    fn project(
        &self,
        programs: &Programs,
        frontier: &dyn FrontierSource,
        project: ProjectFacts<'_>,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ProjectPassError>;
}
```
The doc comments inside the types below are stripped.
```rust
engine/src/inbound/use_cases/load/types.rs:24  pub enum LoadSource<'a> { Plans(SelectedRules<'a>, StatisticsBytes<'a>, LoadGiven<'a>), Pack(PackBytes<'a>, PackDigest) }
:47  pub struct LoadedRules { pub programs: Programs, pub refused: Vec<RefusedRule>, pub matched: MatchedStatistics }
engine/src/domain/lowering/types.rs:52  #[derive(Clone, Copy, Debug)] pub struct SelectedRules<'a>(pub &'a [SelectedRule<'a>]);
:68  pub struct LoadGiven<'a> { pub hosted: &'a [ryo_values::domain::hosted::types::HostRelation], pub observed: &'a [ObservedBinding] }
:150 pub struct RefusedRule { pub rule: SelectedRuleOrdinal, pub refusal: LoadRefusal }
engine/src/domain/programs/types.rs:27  pub struct Programs(pub(crate) ProgramsEngine);
engine/src/domain/scratch/types.rs:41   pub struct ScanScratch(pub(crate) ScratchEngine);
engine/src/domain/findings/types.rs:30  pub trait FoundRows { fn found(&mut self, target: FoundTarget, row: FoundCells<'_>, words: &FileWords); fn noticed(&mut self, notice: WorkNotice); }
:52  pub enum FoundTarget { Rule(SelectedRuleOrdinal), Frontier(SelectedRuleOrdinal, InputNumber) }
:70  pub struct FoundCells<'a>(pub &'a [CellWord]);
:88  (line approx.) ColumnKinds(pub Vec<CellKind>) ; RuleColumns { pub emit: ColumnKinds, pub frontier: Vec<ColumnKinds> }
:121 pub trait FrontierSource { fn rows(&self, rule: SelectedRuleOrdinal, input: InputNumber, each: &mut dyn FnMut(FrontierCells<'_>)); }
:142 pub struct FrontierCells<'a>(pub &'a [FrontierCell<'a>]);   // enum FrontierCell<'a> { Text(NameText<'a>), Number(CellWord) }
:172 pub struct ScanCounters { pub steps: InstructionCount, pub candidates: InstructionCount, pub made_rows: InstructionCount, pub fill_rows: InstructionCount, pub visited: InstructionCount }   // InstructionCount(pub u64)
:219 pub struct WorkNotice { pub rule: SelectedRuleOrdinal, pub verdict: WorkVerdict, pub work: InstructionCount, pub allowance: InstructionCount, pub step: StepId, pub heaviest: HeaviestWork }   // WorkVerdict { Over, Oversized(Relation) }
load/errors.rs:30  pub enum LoadError { PackVersion, PackDigest, PackCorrupt, Vocabulary(FixedNamesError) }
scan/errors.rs:22  pub enum ScanError { Refused { rule: SelectedRuleOrdinal, relation: Relation, cause: AbsenceCause }, Partial { rule: SelectedRuleOrdinal, relation: Relation, cause: PartialCause, missing: Missing } }
project/errors.rs:21  pub enum ProjectPassError { Refused { rule: SelectedRuleOrdinal, relation: Relation, cause: AbsenceCause } }
language/src/domain/facts/types.rs:35  pub struct FileFacts<'f> { pub table: &'f NodeTable, pub source: ByteSlice<'f>, pub file: FileId, pub path: SourcePath<'f>, pub facts: &'f mut dyn FactSource }
:69  pub struct ProjectFacts<'p> { pub facts: &'p mut dyn FactSource }       :223 #[derive(Debug, Default)] pub struct FileWords(pub(crate) WordsEngine);
```

`EnginePort`, at `crates/ryo/crates/execution_runtime/src/outbound/engine/interface.rs:33`. Doc comments stripped except the trait doc; full docs are in the file.
```rust
/// What every `engine` adapter implements.
pub trait EnginePort: Send + Sync + std::fmt::Debug {
    fn load(&self, source: LoadSource<'_>, kinds: &KindVocabulary) -> Result<LoadedRules, LoadError>;
    fn accepted(&self, programs: &Programs) -> Vec<SelectedRuleOrdinal>;
    fn columns(&self, programs: &Programs) -> Vec<RuleColumns>;
    fn encoded(&self, programs: &Programs, digest: PackDigest) -> EncodedPack;
    fn state_for(&self, programs: &Programs) -> ScanScratch;
    fn measuring_state_for(&self, programs: &Programs) -> ScanScratch;
    fn ledger(&self, programs: &Programs, workers: &[&ScanScratch]) -> WorkLedger;
    fn scan(&self, programs: &Programs, file: FileFacts<'_>, scratch: &mut ScanScratch, found: &mut dyn FoundRows) -> Result<ScanCounters, ScanError>;
    fn project(&self, programs: &Programs, frontier: &dyn FrontierSource, project: ProjectFacts<'_>, scratch: &mut ScanScratch, found: &mut dyn FoundRows) -> Result<ScanCounters, ProjectPassError>;
}
```
(`load`, `scan` and `project` are written one parameter per line in the source.)

Core, in `crates/ryo/crates/core/src` at O, doc comments stripped. `StoreKeeping` is in `outbound/lint_store/types.rs:13` and the rest in `outbound/lint_store/interface.rs`. `Stored`, `KeptRows`, `ReadHanded`, `BucketsWanted` and `StoredPack` are in `ryo_values::domain::stores::types`; `OldWords` is there too. I did not read those.
```rust
types.rs:13  #[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum StoreKeeping { Keeps, Discards }
:13  pub trait OldRowBatch: Send { fn read(&mut self, words: &ryo_values::domain::stores::types::OldWords, into: &mut ryo_values::domain::stored_facts::types::PackedWords) -> Result<(), LintStoreError>; }
:27  pub trait LintStorePort: Send + Sync { fn keeps(&self) -> StoreKeeping; fn stored(&self) -> Result<Arc<dyn ReadStore>, LintStoreError>; }
:42  pub trait ReadStore: Send + Sync + std::fmt::Debug {
         fn tables(&self) -> Stored;
         fn handed(&self, wanted: &BucketsWanted) -> Result<ReadHanded, LintStoreError>;
         fn packs(&self) -> Result<Vec<ryo_values::domain::stores::types::StoredPack>, LintStoreError>;
         fn read_old(&self, _words: &ryo_values::domain::stores::types::OldWords, _into: &mut ryo_values::domain::stored_facts::types::PackedWords) -> Result<(), LintStoreError>;
         fn begin_old_rows(&self, _wanted: &BucketsWanted) -> Result<Option<Box<dyn OldRowBatch>>, LintStoreError>;
         fn stage_words(&self, words: ryo_values::domain::stored_facts::types::PackedWords) -> Result<ryo_values::domain::records::types::RecordWords, LintStoreError>;
         fn read_staged(&self, _words: &ryo_values::domain::records::types::StagedWords, _into: &mut ryo_values::domain::stored_facts::types::PackedWords) -> Result<(), LintStoreError>;
         fn keep(&self, stored: Stored, kept: &KeptRows) -> Result<(), LintStoreError>; }
inbound/use_cases/lint/interface.rs:7   pub trait LintUseCase: Send + Sync { fn lint(&self, request: LintRequest) -> Result<LintReport, LintError>; }
inbound/use_cases/open/interface.rs:7   pub trait OpenUseCase: Send + Sync { fn open(&self, request: OpenRequest) -> Result<OpenReport, OpenError>; }
inbound/use_cases/lint/use_case.rs:112  pub fn new(
        file_analysis: FileAnalysis,
        cross_file: CrossFile,
        corpus: Arc<dyn CorpusPort>,
        facts: Arc<dyn FactsPort>,
        language: Arc<dyn LanguagePort>,
        matching: Arc<dyn MatchingPort>,
        query_language: Arc<dyn QueryLanguagePort>,
        memory: Arc<dyn MemoryPort>,
        operators: Arc<dyn OperatorsPort>,
        scheduler: Arc<dyn SchedulerPort>,
        workers: WorkerCount,
        rows: Arc<dyn RowSinkPort>,
        digest: Arc<dyn DigestPort>,
        store: Arc<dyn LintStorePort>,
    ) -> Self
```
- `LintUseCaseImpl` (struct at `lint/use_case.rs:67`) stores `store: Arc<dyn LintStorePort>` at `:93` and `observer: Option<fn() -> Arc<dyn ObserverPort>>` at `:96`. `new` does not take the observer.
- The `ReadStore` docs say `tables()` is what the last run kept except the "handed" rows (rows the files handed to the cross-file steps) and the packs, and `keep` replaces the kept tables atomically.
- `OpenUseCase` exists at O, quoted above.

FACTS
- Per file, `Store::local` (`stored.rs:1238`) writes a `Notes` entry (project notes, manifest, context proof) under the file's version key. It also writes a `FileAnswers` record holding, per binding, `Local { report, frontier: Vec<(node, block index)>, unavailable }`, plus `complete`/`closed` bitsets, encoded frontier `Block`s (`stored_block::encode`), and a `Stamp` plus content hash for each observed path.
- Per request, `Store::save` (`:685`) writes a `Packet` (packed rows, unknowns, rejected_syntax, refusal) and a `Record { proof: Vec<Proof>, packet, unread }`. `Store::closed` (`:1371`) adds the project's `population_proof` and the environment proofs to the request plan.
- Until the end of a run, the cross-file phase holds `World.frontiers` (`Rc<Block>` per slot), `empty_blocks`, `unavailable`, the interned file and symbol tables in `World.run`, `readers::Closed.shared` inputs, and the `populations` map. These are released slot by slot at `batch.rs:772` and `:864-866` once the closed request is answered and the store has consumed it.
- The store is only written at `Store::publish` (`stored.rs:541`), called by the SDK after each round (`entry.rs:58`) and when the cache directory changes (`entry.rs:296`).
- Store methods that take engine types and cannot be separated from the driver as written: `prepare`, `probe`, `reuse_closed`, `admit`, `local`, `save`, `decline`, `closed`, `reuse_context`, `remember_entries`, `population_proof`. Their parameters are `Batch`, `World`, `ReportSlots`, `Receipt`, `Answer`, `Population`, `Environment`, `Authority`, `Refusal` and `PairAliases`. `admit` and `reuse_closed` mutate the `Batch`, and `admit` also mutates the `World`.
- Store methods with no engine types in their signatures: `open`, `directory` (take a `session::source::Source` reader trait), `publish`, `request_inputs`, `bind_work`, `offer`, `truncate_work`, `bind_version`, `inventory`, `contract_file`, `lookup*`, `packet`, `put`, `record`, `group`, `changed_record`, `same_frontier`, `program_inputs`, `remember_program_inputs`.
- The store's validity inputs are the build key, file `Stamp` and blake3 content hash, the file version key (path, reader, permission, bytes hash, environment scope), the rule `(text, params)` key and the binding key. Nothing else is compared.
- At O, the engine contract (`EnginePort::scan`/`project`) takes `FileFacts` and `FrontierSource` and emits rows through `FoundRows::found`. `LintStorePort::stored` and `ReadStore::keep` use `Stored`, `KeptRows` and `ReadHanded` from `ryo_values`. None of the three O store/engine ports mention `Batch`, `World` or `Receipt`.
