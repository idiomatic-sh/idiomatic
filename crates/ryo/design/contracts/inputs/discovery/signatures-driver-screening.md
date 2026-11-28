<!-- a read-only reader's final report, 27 to 28 Nov 2026; task a2acbbde901a38333; copied from the session transcript -->

I read everything through `git show` and `git grep` at `2a207464c`. Two corrections to your paths come first.

- There is no `X/worker.rs`. It is `X/check_session/worker.rs`.
- `Cache` and `Workers` are in `S/types.rs`, not in `domain`.
- Process breach: early on I ran `git show … | cat -n > /tmp/cs.txt`, a file write. I did not use that file afterwards, but it still exists. Please delete `/tmp/cs.txt`.

Abbreviations: X = `crates/ryo/crates/executor/src`, S = `crates/ryo/crates/sdk/src/resources/linter/lint`, D = `crates/ryo/crates/sdk/src/domain`, CS = `X/check_session`. `l` = `crate::languages`.

## 1. PREPARING RULES

`X/lower.rs` has no `pub fn lowered`. It imports `ryo_query_language::domain::plan_lowering::service::lowered` at line 4. That function is declared at `crates/ryo/crates/query_language/src/domain/plan_lowering/service.rs:25`.
```
service.rs:25 pub fn lowered(pack: &PackPrograms, stage: PlanningStage) -> FinishPlan
X/lower.rs:21 pub struct PackTexts { pub rules: Vec<(String, Option<String>)>, pub fragments: Vec<(String, String)> }
:26 pub fn parsed(texts: &PackTexts, context: RuleContext) -> PackPrograms
:56 pub struct ReadPack { pub typed: std::cell::RefCell<Option<std::sync::Arc<FinishPlan>>>, pub programs: PackPrograms }
:62 pub struct Producer { pub rule: Box<str>, pub text: Arc<str>, pub output: Box<str> }
:69 pub struct Observation { pub name: Box<str>, pub columns: Vec<ryo_values::domain::observations::types::ObservationColumn>, pub producers: Vec<Producer> }
:76 #[derive(Default)] pub struct SharedPlanData { pub rows: FxHashSet<Arc<RowsArguments>>, pub conditions: FxHashSet<Arc<PlanExpression>>, pub projections: FxHashSet<Arc<[SelectItem]>> }
:82 pub fn intern<T: ?Sized + Eq + std::hash::Hash>(set: &mut rustc_hash::FxHashSet<Arc<T>>, value: &mut Arc<T>)
:90 pub fn query_parts(rule: &str, text: &str, mut context: RuleContext, keep: bool, separator: Option<&str>, modules: ParsedModules, observations: &[Observation], literals: &mut SharedPlanData) -> Result<(super::finish::Plan, ReadPack), Refusal>
:120 pub fn query_batch(queries: &[(&str, &BTreeMap<Box<str>, ryo_values::domain::finish_plans::expressions::types::PlanConstant>)], separator: &str, modules: ParsedModules, literals: &mut SharedPlanData) -> Vec<Result<(super::finish::Plan, Arc<FinishPlan>), Refusal>>
:207 pub fn finish_plan(pack: &ReadPack, keep_for_first: bool, separator: Option<&str>, emit: Option<&str>, literals: Option<&mut SharedPlanData>) -> Result<super::finish::Plan, Refusal>
```
(The `SharedPlanData` field types are abbreviated above. At :77-79 they are written as full `ryo_values::domain::finish_plans::…` paths.)

`CS.rs` (`X/check_session.rs`):
```
:56 pub struct Program { pub typed: Arc<FinishPlan>, pub plan: Plan, pub accepted_only: bool, pub bases: Vec<String>, pub base_steps: Vec<(usize, Option<l::Relation>)>, pub renders: bool, pub requirements: std::sync::Mutex<Vec<(Option<usize>, std::sync::Arc<Requirements>)>>, pub unknown_kind: std::sync::OnceLock<Option<refusal::Refusal>> }
:70 pub type Requirements = (bool, Vec<Vec<String>>, Vec<Vec<String>>);
:88 Program::prepare(alias: &str, query: &Query, context: RuleContext, capture_separator: Option<&str>, modules: ParsedModules, literals: &mut lower::SharedPlanData) -> std::result::Result<Self, refusal::Refusal>
:102 Program::assemble(query: &Query, plan: Plan, typed: Arc<FinishPlan>) -> std::result::Result<Self, refusal::Refusal>
:130 Program::refuse_unknown_kind(&self, languages: impl FnOnce() -> Vec<(String, Vec<String>)>) -> Option<refusal::Refusal>
:134 Program::requirements(&self, reader: Option<usize>, make: impl FnOnce() -> Requirements) -> (std::sync::Arc<Requirements>, bool)
:601 pub fn bind<'p, L: FileSource>(registry: &mut finish::session::contribution::Registry, program: &'p Program, parser: Option<&L>, run: &mut Run, hosted: Tables, cut: Option<&finish::partition::Partition>, layout: Vec<(l::Relation, String)>, (requirements, test): (Rc<BoundRequirements>, usize)) -> Result<Bound<'p>>
:579 pub struct Bound<'p> { named: Vec<&'static source::FileRelation>, executable: bool, fact_mask: FactMask, requirements: Rc<BoundRequirements>, projects_words: bool, reads_fields: bool, empty_frontier: Option<frontier::EmptyFrontier>, plan: &'p Plan, slots: Rc<Vec<usize>>, test: usize, absent: Option<(u32, Vec<bool>)>, outputs: HashMap<usize,usize>, projections: BTreeMap<usize,finish::session::contribution::Projection>, facts: Vec<l::Relation>, filters: BTreeMap<usize,Vec<(usize,u64)>>, hosted: Tables, layout: Vec<(l::Relation, String)>, lines: bool }   (all fields pub)
:250 #[derive(Default)] pub struct Schedule { pub sources: Vec<Source>, pub source_ids: HashMap<Arc<str>, PathIds>, pub supplied_ids: HashMap<(Rc<str>, Arc<[u8]>, Permission, Option<Arc<str>>, Option<Arc<str>>), usize>, pub named_disk_ids: HashMap<(Arc<str>, Option<Arc<str>>, Option<Arc<str>>), PathIds>, pub pending: BTreeSet<usize>, pub programs: Vec<crate::check_session::refusal::Planned>, pub prepared: HashMap<(Query, Parameters), usize>, pub shared_plan_data: lower::SharedPlanData, pub module_catalogs: HashMap<query::Modules, Result<ParsedModules>>, pub pending_programs: Option<(Arc<SelectedFiles>, Vec<(usize, usize)>)>, pub pairs: usize }
:146 #[derive(Default)] pub struct Clock { pub prepare: f64, pub read_parse: f64, pub screen: f64, pub bound_proofs: f64, pub literal_positions: usize, pub exact_inputs: [usize; 8], pub requirement_programs: usize, pub requirement_bindings: usize, pub visit_projects: usize, pub node_runs: u64, pub nodes_made: u64, pub bulk_screen_files: usize, pub evaluate: f64, pub project: f64, pub assemble: f64, pub project_workers_cpu: f64, pub file_workers_cpu: f64, pub delivery_workers_cpu: f64, pub files: usize, pub reads: usize, pub skipped: usize, pub evaluated: usize, pub certified: usize, pub productive: usize, pub empty: usize, pub refused: usize }
:231 pub trait ReportSelection: FromIterator<usize> { fn assigned(slots: std::ops::Range<usize>) -> Self; }
:180 pub struct Source { pub environment: Option<Arc<project::Environment>>, pub permission: Permission, pub root: Arc<str>, pub path: String, pub reader: Option<Arc<str>>, pub bindings: FilePrograms, pub known: FilePrograms, pub project: bool, pub provided: Option<Arc<[u8]>>, pub seen: AtomicU8 }
```
`bind` returns `Result<Bound<'p>>`. This is the alias `Result<T> = std::result::Result<T, refusal::Refusal>` (:46). Other aliases: `Parameters = BTreeMap<Box<str>, PlanConstant>` (:43), `Answer = Result<Reported>` (:47), `Tables = Vec<(String, Rc<Table>)>` (:54).

Schedule pub methods (verbatim signatures, bodies omitted):
```
:267 delivered(&mut self, ids: &[usize], reusable: impl Fn(usize) -> bool)
:278 finish_preparation(&mut self)
:284 locate_closed(&self, root: &str, readers: &[ReaderState], report: &mut Reported, coordinates: &mut HashMap<usize, l::contract::FindingCoordinates>) -> Result<()>
:300 close(&mut self)
:313 prepare_many(&mut self, items: Vec<(Query, Parameters)>)
:344 prepare(&mut self, text: Query, parameters: Parameters, make: impl FnOnce(ParsedModules, &mut lower::SharedPlanData) -> crate::check_session::refusal::Planned) -> usize
:360 source_in(sources: &mut Vec<Source>, ids: &mut PathIds, root: &Arc<str>, path: &str) -> Result<usize>
:393 provide(&mut self, root: &Arc<str>, path: &str, bytes: &Arc<[u8]>, permission: Permission, reader: &Option<Arc<str>>, environment: &Option<Arc<project::Environment>>) -> Result<(usize, Arc<[u8]>)>
:426 reprovide(&mut self, root: &Arc<str>, path: &str, bytes: &Arc<[u8]>)
:434 listed(&mut self, root: &Arc<str>, directory: &Arc<str>, path: &str, reader: &Option<Arc<str>>, environment: &Option<Arc<project::Environment>>) -> Result<()>
:446 alias_source(&mut self, root: &Arc<str>, path: &str, id: usize) -> Result<()>
:458 project_inputs(&mut self, root: &str, paths: &[String]) -> Result<Vec<usize>>
:471 select_files(&mut self, root: &Arc<str>, files: &[String]) -> Result<Arc<SelectedFiles>>
:480 finish_selection(&mut self)
:490 admit_ids<T: ReportSelection>(&mut self, files: &Arc<SelectedFiles>, binding: usize, known_binding: bool) -> T
:510 slot(source: &mut Source, src: usize, binding: usize, known_binding: bool, pairs: &mut usize, pending: &mut BTreeSet<usize>) -> usize
```

Other pub items, one per line:
```
CS/shared.rs:7 pub type Aliases = HashMap<String, String>
:10 pub struct ReaderPlan { pub steps: SharedSteps, pub hosts: HashMap<Rc<Table>, String>, pub outputs: Vec<(String, usize, String, Vec<String>)> }
:17 add(&mut self, b: &mut Bound<'_>, run: &mut Run)   :42 append(&mut self, b: &mut Bound<'_>, given: &Aliases)   :65 plan(&mut self) -> Rc<Plan>   :75 finish(mut self) -> (Rc<Plan>, rustc_hash::FxHashMap<String, Rc<Table>>)
CS/kinds.rs:11 pub fn unknown(plan: &FinishPlan, languages: impl FnOnce() -> Vec<(String, Vec<String>)>) -> Option<Refusal>
CS/query.rs:17 pub struct Modules { pub sources: Arc<BTreeMap<Box<str>, Arc<str>>>, pub digest: [u8; 32] }   :23 Modules::new(sources: BTreeMap<Box<str>, Arc<str>>) -> Self
:40 pub struct Query { pub observations: Rc<[crate::lower::Observation]>, pub accepted_only: bool, pub every_row: bool, pub text: Rc<str>, pub digest: [u8; 32], pub modules: Option<Modules> }   :51 impl From<Rc<str>> for Query   :83 parse_modules(&self) -> Result<ParsedModules, String>
CS/file_programs.rs:5 pub type Job = (usize, usize);  :8 pub struct Selection { pub bases: Arc<[Job]>, pub ordinal: usize }  :16 pub struct FilePrograms { pub common: Option<Selection>, pub individual: Vec<Job> }
:24 common_id(&self) -> Option<usize>  :28 len(&self) -> usize  :31 is_empty(&self) -> bool  :33 iter(&self) -> impl Iterator<Item = Job> + Clone + '_  :39 materialize(&mut self) -> &mut Vec<Job>  :48 lookup(&self, binding: usize) -> Option<usize>  :57 admit(&mut self, binding: usize, new: impl FnOnce() -> usize) -> usize  :62 share(&mut self, bases: &Arc<[Job]>, ordinal: usize)  :72 clear(&mut self)  :77 keep(&mut self, mut f: impl FnMut(Job) -> bool)  :81 keep_programs(&mut self, eligible: &impl Fn(usize) -> bool, cache: &mut HashMap<usize, (Arc<[Job]>, Arc<[Job]>)>)  :98 append(&mut self, incoming: &mut Self)
CS/named.rs:8 pub struct Catalog(BTreeMap<&'static str, &'static FileRelation>)  :11 from_readers<L: FileSource>(readers: &[L]) -> Result<Self>  :33 declare_bases(&self, run: &mut Run, bases: &[String])  :41 declare(&self, run: &mut Run)
:49 pub fn declare(run: &mut Run, schema: &FileRelation)   :61 pub struct EmptyTables(FxHashMap<&'static str, Rc<Table>>)
:63 pub fn table(empty: &mut EmptyTables, mut columns: super::source::Columns, schema: &FileRelation, id: FileId, symbols: &l::Symbols, bytes: &[u8], run: &mut Run, unknown: &HashMap<u32, u32>, resolved: &mut FileSymbols, extent: super::refusal::Extent<'_>) -> Result<Rc<Table>>
```

`CS/replay/batch.rs`. Derives: `Input` is `#[derive(Deserialize)]` and `LiveAdmission` is `#[derive(Clone, Copy)]`. `Batch` and `Active` are `#[derive(Default)]`.
```
:5 pub struct Input { pub root: String, pub files: Rc<Vec<String>>, pub inventory: Option<Rc<Vec<String>>>, pub repository: Option<Authority>, pub hosted: Vec<Hosted>, pub parameters: Parameters }
:18 pub enum Work { Project(Rc<Answer>), Closed { root: Arc<str>, project: Option<usize>, pairs: Vec<usize>, binding: usize, inventory: Rc<Vec<String>>, files: Rc<Vec<String>> }, Pairs(PairSlots), Refused(Refusal), Invalid(Refusal) }
:82 pub struct LiveAdmission<'a> { pub version: &'a Arc<str>, pub selection: usize, pub host_tables: &'a [usize] }
:88 pub struct Admission { pub id: usize, pub contract: Arc<session::completeness::Contract>, pub project_bases: Vec<l::Relation>, pub closed: bool, pub file_cells: bool, pub sliced_host: bool, pub file_inventory: bool, pub layout: Option<(Vec<l::Relation>, Arc<session::completeness::Contract>)>, pub cut: Option<Arc<finish::partition::Partition>> }
:102 pub struct Batch { pub patterns: finish::Patterns, pub file_catalog: session::named::Catalog, pub closed_reuse: [usize; 5], pub workers: Option<NonZeroUsize>, pub schedule: session::Schedule, pub program_of: Vec<usize>, pub class_of: Vec<usize>, pub bindings: Vec<Arc<[Hosted]>>, pub host_rows: HashMap<Arc<[Arc<[super::HostCell]>]>, usize>, pub binding_ids: HashMap<(usize, Option<usize>, Vec<(usize, Option<String>)>), usize>, pub layout_of: Vec<Option<usize>>, pub layouts: Vec<Option<project::Layout>>, pub layout_ids: HashMap<(String, Authority, Rc<Vec<String>>, Vec<l::Relation>), Option<usize>>, pub work: Vec<Work>, pub projects: Vec<ProjectSpec>, pub cut_of: Vec<Option<Arc<finish::partition::Partition>>>, pub local_bindings: HashMap<usize, usize>, pub project_keys: HashMap<(String, Authority, Rc<Vec<String>>), usize>, pub admissions: HashMap<(usize, HostSchema), Rc<Admission>>, pub bound_contracts: Vec<Arc<session::completeness::Contract>>, pub reused_binding_entries: usize, pub host_binding_counts: [usize; 2], pub last_sources: Option<(Arc<str>, usize, Arc<session::SelectedFiles>)>, pub source_lookups_removed: usize, pub slot_lookups_removed: usize, pub clocks: Clock, pub empty_reader_epochs: usize, pub populations_built: usize, pub relations_built: usize, pub relations_rebuilt: usize }
:136 pub struct Active { pub paths: Arc<Vec<String>>, pub manifests: Option<BTreeSet<&'static str>>, pub readers: Vec<session::ReaderState>, pub world: session::World, pub answers: ReportSlots, pub aliases: session::worker::PairAliases, pub contributions: Vec<Option<Rc<project::Contribution>>>, pub project_unread: HashMap<usize, Vec<l::Refused>>, pub populations: HashMap<usize, (BTreeSet<l::Relation>, Result<Population, Refusal>)> }
Batch pub fns (all in impl Batch, :219):
:220 prepare_rules(&mut self, items: Vec<(Query, Parameters)>)
:225 prepared_rule(&mut self,query:Query,parameters:&Parameters)->usize
:239 register_query<L: session::source::Source>(&mut self, query: Query, mut req: Input, inventory_for: impl FnOnce(&str) -> Result<Option<Rc<Vec<String>>>, String>, sdk: Option<LiveAdmission<'_>>, layouts: Option<(&[L], Option<Arc<project::Environment>>)>) -> Result<Work, crate::check_session::refusal::Refusal>
:384 admit<T: session::ReportSelection>(&mut self, root: &str, files: &[String], binding: usize, known_binding: bool, sdk: Option<LiveAdmission<'_>>) -> Result<T, crate::check_session::refusal::Refusal>
:412 binding(&mut self, rule: usize, class: &Admission, mut hosted: Vec<Hosted>, canonical: Option<&[usize]>, layout: Option<usize>) -> usize
:453 run_sources<L: session::source::Source>(&mut self, readers: &[L]) -> ReportSlots
:465 release_admission(&mut self)
:472 epoch<L: session::source::Source>(&mut self,languages:&[L],state:&mut Active)
:475 epoch_with_store<L: session::source::Source>(&mut self, languages: &[L], state: &mut Active, mut store:Option<&mut stored::Store>)
:561 read_pending<L: session::source::Source>(&mut self, languages: &[L], state: &mut Active, pending: &[usize], release_files: bool)
:654 add_work(&mut self, work: Work, retained: Option<(&mut stored::Store, stored::Key)>) -> usize
:664 finish_projects<L: session::source::Source>(&mut self, languages: &[L], reader_states: &[session::ReaderState], contributions: &mut Vec<Option<Rc<project::Contribution>>>, world: &mut session::World, answers: &mut ReportSlots, populations: &mut HashMap<usize, (BTreeSet<l::Relation>, Result<Population, Refusal>)>, project_unread: &mut HashMap<usize, Vec<l::Refused>>, mut store:Option<&mut stored::Store>)
:888 answer<'a>(work: &'a Work, answers: &'a ReportSlots) -> Result<Receipt<'a>, std::borrow::Cow<'a, Refusal>>
```
Other types in the file:
- `PairSlots { first: Range<usize>, rest: Vec<usize> }` (:36), implementing `ReportSelection`.
- `ProjectSpec` (:72).
- `Receipt<'a>` (:909).

## 2. THE LANGUAGE HANDLE

`CS/source.rs:10` is the `Source` trait. It is not the same type as the struct `check_session::Source` at CS.rs:180. Doc comments are stripped.
```
pub trait Source: Clone + Sync {
    fn source_words(&self) -> Option<l::SourceWords> { None }
    fn uses_project(&self) -> bool { false }
    fn declarations(&self) -> &'static [FileRelation] { &[] }
    fn scoped(&self) -> &dyn l::NumberedVisit;
    fn numbered_projections(&self) -> &'static [l::Relation] { self.scoped().projections() }
    fn name(&self) -> &str;
    fn identity(&self) -> (&str, &str);
    fn reads(&self, path: &str) -> bool;
    fn manifests(&self) -> Vec<&'static str>;
    fn words(&self) -> Vec<l::UnknownName>;
    fn kinds(&self) -> Vec<&'static str>;
    fn fields(&self) -> l::FieldNames { l::FieldNames(Vec::new()) }
    fn expands(&self, rel: l::Relation) -> bool;
    fn available(&self, rel: l::Relation) -> bool;
    fn absent(&self, rel: l::Relation) -> bool;
    fn entries(&self, req: &l::ProjectRequest, out: &mut ProjectEntries) -> Result<()>;
    fn file_entries(&self, _req: &l::ProjectRequest, _file: usize, _notes: &l::Bytes, _out: &mut ProjectEntries) -> Result<()> { Ok(()) }
    fn file_rows(&self, _req: &l::ProjectRequest, _file: usize, _notes: &l::Bytes, _observed: &l::ObservedEntries, _symbols: &mut l::Symbols, _rows: &mut l::contract::Collect) -> Result<Option<l::Resolved>> { Ok(None) }
    fn project(&self, req: &l::ProjectRequest, symbols: &mut l::Symbols, rows: &mut l::contract::Collect) -> Result<l::Resolved>;
}
source.rs:7 pub use ...parse::named::FileRelation;  :8 pub type Columns = Vec<ryo_relations::domain::relations::types::FactColumn>;
:59 pub fn routed<'a, L: Source>(languages: &'a [L], reader: Option<&'a str>, path: &'a str) -> impl Iterator<Item = (usize, &'a L)>
:66 pub fn takes_environment<L: Source>(languages: &[L], reader: Option<&str>, path: &str) -> bool
```
`ProjectEntries` is `ryo_language::domain::files::types::ProjectEntries`.

`CS/linked.rs:14`:
```
#[derive(Clone)] pub struct Linked { name: Arc<str>, build: Arc<str>, parser: Arc<dyn ParseEngine>, walker: Arc<dyn WalkEngine>, projector: Arc<dyn ProjectEngine> }
:25 pub fn new(name: &str, build: &str, parser: Arc<dyn ParseEngine>, walker: Arc<dyn WalkEngine>, projector: Arc<dyn ProjectEngine>) -> Self
```
- `Linked` wraps three `ryo_language::inbound::use_cases` traits: `parse::types::ParseEngine`, `walk::types::WalkEngine` and `project::types::ProjectEngine`.
- `Source::scoped` goes through `parser.file_relations().numbered()`. That returns `l::NumberedVisit`, whose `open()`, `explicit()` and `visit()` are used at CS.rs:1618-1621.
- A `Fake` implementation exists under `cfg(any(test, feature="fixtures"))` (linked.rs:122).

`X/languages.rs` has only `pub use` lines (pattern `pub use <crate>::…::Name;`), plus `pub mod contract`. The `ryo_language` and `ryo_relations` crates are the only sources. One `pub use ... as facts;` and one `as node_table;` are module aliases.
- Types and constants:
  - Answers and cells: `AbsenceCause`, `UNAVAILABLE_CAPABILITY`, `ByteSlice`, `Bytes`, `CellKind`, `CellWord`, `ChildOrdinal`, `Count`, `EmbeddedMapping`, `EmbeddedText`, `FactAnswer`, `FieldCode`, `FieldNames`, `FieldRequest`.
  - Files and projects: `FileId`, `FilePath`, `FileProject`, `FileRelation`, `FileRows`, `FileWords`, `FillExtent`, `FilledColumn`, `FilledColumns`, `KindBits`, `ManifestFile`, `ManifestText`, `Missing`, `NO_FILE`, `NO_SYMBOL`, `NameText`, `NodeId`, `NodeTable`, `NoteRequest`.
  - Numbered and parse: `NumberedFacts`, `NumberedFile`, `NumberedVisit`, `ObservedEntries`, `ObservedEntryState`, `Offset`, `ParseError`, `ParseRequest`, `Position`, `ProjectRequest`, `Recognition`, `Refused`, `Relation`.
  - Render and symbols: `RenderContext`, `RenderOutcome`, `RenderRequest`, `Repository`, `Resolved`, `RootPath`, `SourcePath`, `SourceWords`, `Spelling`, `Symbol`, `Symbols`, `SyntaxAcceptance`, `SyntaxCompleteness`, `UnknownName`, `Wanted`.
- Services and modules: `facts`, `intern_derived`, `node_table`, `text_of`, `unknown_symbol`.
- `contract` re-exports `FindingCoordinates`, `name_count`, `Collect` and `ParseEngine`.

## 3. ONE FILE

```
CS.rs:864 pub fn visit<L: FileSource>(state: &mut ReaderState, parser: Option<&L>, reader_id: Option<usize>, manifests: &BTreeSet<&str>, contributions: &mut (impl Slots<Option<Rc<project::Contribution>>> + ?Sized), programs: &[crate::check_session::refusal::Planned], program_of: &[usize], contracts: &[Arc<completeness::Contract>], cut_of: &[Option<Arc<finish::partition::Partition>>], layouts: (&[Option<usize>], &[Option<project::Layout>]), world: &mut World, sources: &[Source], selected: &[usize], aliases: &mut (impl Slots<usize> + ?Sized), answers: &mut (impl Slots<Answer> + ?Sized), mut host: impl FnMut(usize, &mut Run) -> Result<Tables>, clocks: &mut Clock)
CS.rs:726 pub fn versions<'s>(group: &[usize], sources: &'s [Source], answers: &mut (impl Slots<Answer> + ?Sized), aliases: &mut (impl Slots<usize> + ?Sized), reads: &mut usize, retained: &mut HashMap<usize, Arc<[u8]>>, needed_later: impl Fn(&Source) -> bool) -> Vec<Version<'s>>
:707 pub struct Version<'s> { pub origin: usize, pub project_slots: Vec<usize>, pub data: Arc<[u8]>, pub jobs: Cow<'s, FilePrograms>, pub failed: Option<ReadFailed> }   :718 pub struct ReadFailed { pub reason: String, pub missing: bool }
:844 pub struct ReaderState { pub run: Run, pub screens: pairs::Retained, pub bytes: HashMap<usize, Arc<[u8]>> }
:851 #[derive(Default)] pub struct HeldFile { pub tree: l::NodeTable, pub contents: FileSummary, pub literals: RefCell<crate::pair_checks::LiteralPositions>, pub proof_atoms: crate::pair_checks::AtomAnswers }
```
`versions` (726-843) groups the sources that share a path. It reads the bytes (`src.provided`, else the disk), reads each hard-linked file once, writes into `answers` and `aliases`, and returns one `Version` per distinct content. I did not read lines 740-843 line by line.

Outline of `visit` (a per-reader batch over the files in `selected`; per-file work is inside the loop):
- **883-951, once per visit (per reader shard):**
  - `run.sym`, `run.links`, `reader::public_words`, and `kind_sym` built from `p.kinds()`.
  - `needed[binding]` from `selected` and `bindings.common_id()`.
  - `state.screens.merged(...)` (:907).
  - `reader_declaration` (:72).
  - `Bindings`, `contribution::Registry`, `FileDemand`, `Packet::new`, `EmptyTables`, `WitnessTables`.
  - `parser.map(FileSource::scoped)`, and `by_path`.
- **952-991, per binding:**
  - `host(binding,&mut run)`.
  - `reader_run::file_requirements` (:962).
  - `BoundSummary::bind` (:966).
  - `layout.declare`.
  - `bind(...)` (:979).
  - `merged.add` (:981).
  - Then `merged.finish()` and `Exec::new` (:989-990).
- **996-1650, per path group, and 1009-1646, per file version:**
  - Read bytes: `versions(...)` at 998-1006.
  - Per-file demand: 1020-1056.
  - Skip the file if it has no plan and no project slots: 1066.
  - Build the `ParseRequest`: 1079-1093.
- **1095-1612, the `consume` closure.** The language calls it exactly once per file, with the numbered file, or with `None` when no reader claims the file.
  - Unpack the loan and its coverage: 1096-1109.
  - Project contribution via `contributions.set`: 1115-1129.
  - Provider facts and words: 1130-1146.
  - Content screen, `contents.excludes`, with `populate` inside it: 1148-1201.
  - Fill facts: `scoped_facts.fill_numbered` (:1223), lazy fills (:1209-1266), partial coverage (:1231-1274), `packet.index` (:1275).
  - Bound-proof screen: `screen.exclude` (:1286), then `exact_roots` and `exact_locations`. This can certify an answer through `finish::session::publish_locations` (:1305). 1280-1314.
  - Named tables: `supplier.named` and `named::table`: 1321-1361.
  - Operator columns, facts view and renderer: 1363-1373.
  - Line-count table: 1375-1390.
  - Project layout tables: `layout.read_file` and `layout.slice`: 1391-1431.
  - Evaluate each active (binding, slot): 1451-1603.
    - Certified answer: 1534-1542.
    - `FileVersion::new` and `finish::session::frame`: 1544-1560.
    - Closed or project program: `shared.contribute(...)` (:1563), `reachable_partials`, then `answers.set`.
    - File-only program: `finish::session::pair(...)` (:1590), `covered_empty`, then `answers.set(slot, answer)` (:1602).
  - Cleanup and clocks: 1604-1611.
- **1613-1645, language call:**
  - `supplier.open()`, `reader.explicit(..)`, `reader.visit(request,&requested,&demand.named,env,&mut numbered_symbols,&mut |file,dict| consume(Some(..)))` (:1621-1634).
  - `None` calls `consume(None)`; an error answers every slot of the file.
- **1647-1653:** `world.boundaries.push`, then `exec.stats` goes into the clocks.

Hand-off rows go to the cross-file phase through `contribute` (:1563, closed programs, into `World`) and `contributions.set` (:1123, project notes). Per-pair answers go through `answers.set`.

```
CS/worker.rs:5 pub trait Slots<T> { fn previous(&self, _slot: usize) -> Option<&T> { None } fn set(&mut self, slot: usize, value: T); fn set_empty(&mut self, slot: usize, empty: impl FnOnce() -> T) { self.set(slot, empty()); } }
  impls: [T] (:12), Vec<(usize, T)> (:19), PairAliases for Slots<usize> (:44)
:28 pub struct PairAliases { pub pages: Vec<Option<Box<[usize; 256]>>>, pub writes: usize }  :33 lookup(&self, id: usize) -> usize  :38 is_empty(&self) -> bool  :40 counts(&self) -> (usize, usize)
:55 pub fn dispatch<L: FileSource>(request: Option<NonZeroUsize>, languages: &[L], paths: &Arc<Vec<String>>, state: &mut ReaderState, parser: Option<&L>, reader_id: Option<usize>, manifests: &BTreeSet<&str>, contributions: &mut [Option<Rc<project::Contribution>>], programs: &[Planned], rules: &[usize], contracts: &[Arc<completeness::Contract>], cuts: &[Option<Arc<finish::partition::Partition>>], layouts: (&[Option<usize>], &[Option<project::Layout>]), world: &mut World, sources: &[Source], selected: &[usize], aliases: &mut PairAliases, answers: &mut (impl Slots<Answer> + ?Sized), host: impl Fn(usize, &mut Run) -> Result<Tables> + Sync, clocks: &mut Clock)
CS/jobs.rs (full, 23 lines; comment lines 1-2 are the module doc):
use rayon::prelude::*;
pub fn run<I, T, F>(jobs: I, work: F) -> (Vec<T>, f64)
where I: IntoParallelIterator<Iter: IndexedParallelIterator>, T: Send, F: Fn(I::Item) -> T + Send + Sync,
{ let owner = std::thread::current().id();
  let results: Vec<_> = jobs.into_par_iter().map(|job| { let start = (std::thread::current().id() != owner).then(super::thread_cpu); let result = work(job); (result, start.map_or(0., |start| super::thread_cpu() - start)) }).collect();
  let mut child_cpu = 0.; let values = results.into_iter().map(|(value, cpu)| { child_cpu += cpu; value }).collect(); (values, child_cpu) }
```
`dispatch` (:77-105) runs a single `visit` inline when `count<=1` or every selected file has the same path. Otherwise (:107-237):
- it groups files by path and weights each by size times the number of bindings;
- it bins them greedily into `min(count, jobs)` shards;
- each shard runs `visit` over a fresh `ReaderState` and `World` through `jobs::run`, with `finish::PoolBudget::for_workers`;
- `answers`, `aliases`, `contributions`, `state.bytes` and `clocks` are merged back, and `world.receive(transfers, sources)` is called.

## 4. SCREENING

Public types and functions (full signatures only for the entry points; the rest are listed by name and line):
```
X/pair_checks.rs: Bits=Rc<Vec<u64>> :11; Nodes :13; SymbolColumn<'a> :14; SyntaxText<'a>{ids,symbols,patterns,dictionary,positions,distinct,column} :17 (fn matches :132, at :146); SymbolMatches :28; PatternAnswer :31; Predicates<'a>{cache:&SymbolMatches, scope:usize} :39 (with_predicate :96); FileSymbols{seen,words,next} :47 (clear,any,contains,words,next_distinct); ReservedWords<'a> :153; Reader<'a> :169; Table{read,answered,complete,cells,index} :172 (load :401); WitnessTables :181 (begin_file :184); LiteralPositions :202 (clear,scanned,head,positions); Answers :240; AtomAnswers{packed} :264; SharedAtoms<'a>{slots,answers} :285 (cached_roots :1165); Scratch :292; FactSymbol :301; FactSymbols<'a> :309; File<'a, const PROFILE: bool> :349 (complete,pass,pass_miss,literal_id,nodes,nodes_miss); fns insert :377, bits :384, each :391
  :1183 Program::selected_columns(&self, t: &NodeTable, src: &[u8], path: &[u8], syntax_text: Option<SyntaxText<'_>>, reserved: ReservedWords<'_>, tables: &mut WitnessTables, ...)   (signature continues; not fully read)
  :1212 Program::evaluate<const PROFILE: bool>(&self, t: &NodeTable, src: &[u8], path: Option<&[u8]>, syntax_text: Option<SyntaxText<'_>>, reserved: ReservedWords<'_>, tables: &mut WitnessTables, words: &mut FileWords, mut read: impl FnMut(Source, &mut FileWords, &mut FilledColumns) -> bool, alive: &mut [bool], mut exact: Option<&mut Vec<(usize, Bits)>>, known: &[(Id, bool)], selected: &[u64], ...)   (tail not read)
X/pair_checks/counts.rs: compare(left: i64, op: Comparator, right: i64) -> bool :5; under(...) :21.   witness.rs: any :6, contains :18, File::edge :27, File::path_exists :71
X/pair_summary.rs: pub mod compact,cost,exact,groups,patterns,paths,precheck,project,regexes; Program{scratch,meter,paths,file_only,domains,needs,exact,roots,regexes,path_globs,text_kinds,symbols,max_text,texts,facts,capped} :109 (the compiled screening program); Domain :41; Need :72; CellTest :88; Source :94; Fact :99; Attribute :129; Column :142; Summary<'a> :147; Builder<'a> :240 (domain, need, and, or, base, select, step, filter, ...)
  :26 pub fn inputs(s: &FinishStep) -> SmallVec<[StepNumber; 4]>   :1622 choices   :1655 numeric_comparison   :1681 conjuncts(step: &FinishStep) -> Conjuncts<'_>   :1691 terms
  :1701 pub fn prepare_plans(plans: &[&FinishPlan], targets: &[Vec<usize>], vocab: &Vocab, language: &Language, reader_rows: &[String], current_file: Option<(&'static str, &'static str)>, syntax_symbols: bool) -> Program
  :1793 pub fn prepare_roots(b: &mut Builder<'_>, plan: &FinishPlan, targets: &[usize], omit: u16, phase: &mut impl FnMut(&str))
  sub-modules (pub items by name): compact::{Compact,Reachable,compact}; cost::{Family,Price,SchedulePlan,Meter,Sample,arrange,family,omitted}; exact::{Value,Row,Certificate,LocationColumn,Grouped,Exclusion,prepare,select,...}; groups::compile; paths::{Edge,compile}; patterns::{gather,glob_terms,CapturePath}; precheck::{Early,schedule}; project::{origins,columns,common}; regexes::{Matcher,Pattern,glob_matcher,required_literal}
X/pair_skip.rs: Clause :7; Clauses :12; Skip<'a>{scan,shape,words} :48 (filter :126, with_terms :180, root(&self)->(bool,Vec<Vec<String>>,Vec<Vec<String>>) :235); Demand{kinds,fields,words} :252; Shape :259; BoundClause<T> :265; PrerequisiteClauses{shape,words} :273; BoundSummary{never,shape,words,demand} :308; Projection<'a>{kinds,fields,words} :373; FileSummary{asked,complete,answers,clauses,kinds,fields,words} :423 (reset :434, excludes :446, missing :460); clause_missing :504; project_fields :518
  :316 BoundSummary::bind((never, shape, words): &(bool, Vec<Vec<String>>, Vec<Vec<String>>), clauses: &mut PrerequisiteClauses, mut intern: impl FnMut(&[u8]) -> u32) -> Self
  :381 Projection::populate(mut self, tree: &NodeTable, run: &mut super::finish::Run, numbered: Demand, words: impl FnOnce(&mut FxHashSet<u32>)) -> Demand
  :446 FileSummary::excludes(&mut self, id: usize, bound: &BoundSummary, mut project: impl FnMut(Projection<'_>) -> Demand) -> bool
X/first.rs: Vocab{kinds,kind_names,fields,field_names} :7; WalkFact :15; Language{fact_family,walk_facts,facts,text_kinds,host_len,file_sets,spelled,is_identifier} :22; :35 declaration() -> Language; :48 span_bytes(table: &NodeTable, i: usize) -> Range<usize>
X/reader_run.rs: :12 pub fn file_requirements(plan: &FinishPlan, language: &first::Language) -> Vec<(String, (bool, Vec<Vec<String>>, Vec<Vec<String>>))>   :36 pub fn fact_family(name: &str) -> pair_summary::cost::Family   (pub mod packet (689 lines, pub items not read), pub mod pairs)
X/reader_run/pairs.rs:5 pub fn vocabulary(kinds: &[l::NameText<'_>], fields: l::FieldNames) -> first::Vocab
CS/pairs.rs: :13 pub struct Checks { program: pair_summary::Program, roots: Vec<Option<Vec<usize>>>, lefts: Vec<Vec<usize>>, exact_roots: Vec<(bool,bool)>, domains: Vec<Vec<(usize,usize)>>, live: Vec<bool>, selected: Vec<u64>, locatable: Vec<u64>, atoms: Vec<Option<usize>>, words: l::FileWords, row_guards: Vec<(usize,l::Relation)>, known: Vec<(usize,bool)>, exact: Vec<(usize,Bits)> } (all pub)
  :28 pub fn vocabulary<L: FileSource>(parser: Option<&L>) -> crate::first::Vocab
  :35 Checks::from_roots<L: FileSource>(parser: Option<&L>, vocab: &crate::first::Vocab, programs: &[Planned], program_of: &[usize], mut roots: Vec<Vec<usize>>, mut domains: Vec<Vec<(usize, usize)>>) -> Self
  :168 Checks::exclude(&mut self, positions: &[usize], active: &mut [(usize, usize, bool)], bound: &mut super::Bindings<'_>, run: &mut Run, tree: &l::NodeTable, text: Option<&[u32]>, distinct: &RefCell<FileSymbols>, literal_positions: &RefCell<LiteralPositions>, patterns: Predicates<'_>, answers: &mut AtomAnswers, bytes: &[u8], path: &str, id: FileId, symbols: &l::Symbols, local_words: Option<&mut l::FileWords>, lazy: Option<&reader_run::packet::Lazy<'_, '_>>, tables: &mut WitnessTables, requested: &[l::Relation], packet: &reader_run::packet::Packet, unknown: &HashMap<u32, u32>, cuts: &[Option<Arc<Partition>>], partials: &[ReadInPart], layouts: &[Option<super::project::Layout>])
  :353 asked_first(&self) -> Vec<l::Relation>   :371 exact_locations(&self, binding: usize) -> Option<impl Iterator<Item = (&[LocationColumn], &[u64])>>
  :393 host_executable(run: &Run, id: FileId, out: &mut l::FilledColumns) -> bool   :406 domain_steps(cut: Option<&Partition>) -> Vec<(usize, usize)>   :417 target_steps(program: &Program, cut: Option<&Partition>) -> Vec<usize>
  :427 pub struct Retained { vocabulary: Option<Vocab>, shapes: HashMap<(usize,Vec<usize>,Vec<(usize,usize)>),usize>, bindings: Vec<Option<usize>>, sets: HashMap<Vec<usize>,usize>, programs: Vec<Checks>, atoms: atoms::Atoms }  (derive Default)
  :437 Retained::merged<L: FileSource>(&mut self, parser: Option<&L>, programs: &[Planned], program_of: &[usize], needed: &[bool], cuts: &[Option<Arc<Partition>>]) -> (usize, &mut Checks, Vec<usize>)
CS/pairs/atoms.rs: Key :7; Atoms :14 (intern :17, bind(&mut self, program: &Program) -> Vec<Option<Id>> :22); Binding<'a> :36
```

**Driver into screening** (caller -> callee):
- `CS.rs:72-73` -> `first::declaration()`
- `CS.rs:846,854,855` (field types) -> `pairs::Retained`, `LiteralPositions`, `AtomAnswers`
- `CS.rs:907` -> `Retained::merged`, which calls `vocabulary`, `Checks::from_roots` and `Atoms::bind`, and through them `pair_summary::prepare_plans` (CS/pairs.rs:105)
- `CS.rs:913,935,941` -> `SymbolMatches::default`, `FileSymbols::default`, `WitnessTables::default`
- `CS.rs:357,962,966` -> `PrerequisiteClauses::default`, `reader_run::file_requirements`, `BoundSummary::bind`. The last two return values owned by the driver (`Program.requirements`, `Rc<BoundRequirements>`).
- `CS.rs:1175-1184` -> `FileSummary::excludes`, with a `Projection::populate` closure
- `CS.rs:1211` -> `Checks::asked_first`
- `CS.rs:1285` builds `Predicates { cache, scope }`
- `CS.rs:1286` -> `Checks::exclude`
- `CS.rs:1290,1302` -> `screen.exact_roots[..]` and `exact_locations`
- `replay/batch.rs:604` -> `worker::dispatch`, which calls `visit` at `worker.rs:85` and `:149`. These are the only paths into screening from the driver.
- `replay/batch.rs`, `worker.rs`: no direct reference to `pair_*`, `first` or `reader_run` (the `git grep` found none).
- SDK -> screening: not read.

**Screening back into the driver or evaluator** (caller -> callee):
- `CS/pairs.rs:4` imports `super::{FileId, FileSource, Program, Run, l, reader_run}`.
- `CS/pairs.rs:36,438` -> `check_session::refusal::Planned`
- `CS/pairs.rs:47-48,417-420` -> `Program.plan.emits`, `Program.typed.steps`
- `CS/pairs.rs:111` -> `crate::finish::host::CURRENT_FILE`
- `CS/pairs.rs:190,406,417,439` -> `finish::partition::Partition` (fields `inputs`, `domains`, `reach`)
- `CS/pairs.rs:172,315-346` -> `super::Bindings` and `Bound` fields `facts`, `absent`, `projections` (written at :336-346), plus `cut.inputs`
- `CS/pairs.rs:192,287-290` -> `super::project::Layout` (`relations`, `witness_columns`)
- `CS/pairs.rs:173,245-275,393-403` -> `Run` (`texts`, `ids`, `absent_word`, `supplied_permission`) and `crate::host_columns::fill`
- `pair_skip.rs:384,394-407` -> `finish::Run` (`kind_sym`, `field_sym`, `empty`)
- `first.rs:39` -> `finish::host::host_len`
- `pair_checks.rs:989,1105`, `counts.rs:8` -> `finish::NULL`
- `pair_summary.rs:1710` -> `super::thread_cpu` (crate root, from `bench_support`)
- `pair_skip.rs:2` -> `first::Language`
- `reader_run.rs:2` imports `finish`. It is not used in the lines I read.

**Evaluator into screening:**
- `finish/answer.rs:80,85` and `finish/session.rs:548,551` -> `pair_summary::LocationColumn`
- `finish/session.rs:557` -> `pair_checks::each`
- `CS.rs:1305` -> `finish::session::publish_locations(..)` takes the screen's masks and columns.

## 5. REFUSALS AND FAULTS (`CS/refusal.rs`)
```
:6 #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] pub enum Refusal { MissingParameter { name: String }, UnknownKind { word: String, languages: String }, UnknownRelation { name: String }, Provider { provider: String, fault: ProviderFault, relation: String }, Other(String) }
  impls: Display, Error, From<String>, From<&str>, From<Refusal> for String
:73 pub type Planned = Result<crate::check_session::Program, Refusal>;
:76 #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)] pub enum ProviderFault { SpanOutsideFile, NodeOutsideTable, NodeOfAnotherFile, FileNotVisited }
:101 #[derive(Clone, Copy, Debug)] pub struct Extent<'a> { pub provider: &'a str, pub home: u32, pub elements: u64, pub bytes: u64 }
:115 Extent::fault(&self, kind: CellKind, label: &str, cells: &[u64]) -> Option<ProviderFault>
```

## 6. THE SDK SIDE
```
S/entry.rs:18 pub fn new(languages: Languages, pool: WorkerPool, cache: Cache) -> Self      (#[must_use]) { Self::over(super::composition::build_engine(languages.0, pool, cache)) }
S/entry.rs:31 pub fn lint(&self, requests: Vec<LintRequest>) -> Vec<Result<LintReport, LintError>>      (#[must_use]) body at S/entry.rs:31-79, quoted above in outline
S/types.rs:129 pub struct Linter(LinterLintEngine);
S/types.rs:133 pub(crate) struct LinterLintEngine { pub scheduler: Arc<dyn SchedulerPort>, pub workers: WorkerCount, pub cache: Cache, pub languages: crate::domain::language::types::Languages, pub skipped: Vec<Box<str>>, pub session: usize }
S/composition.rs:17 pub(crate) fn build_engine(languages: Vec<Language>, pool: WorkerPool, cache: super::types::Cache) -> LinterLintEngine
```
`lint` does this in order:
1. Return early for an empty list.
2. `executor::in_use()` picks the executor.
3. `walk::listed(request, engine)` runs per request.
4. `engine.workers.0` is turned into a `NonZeroUsize`.
5. `engine.scheduler.on_owner(&mut job)` runs `execute(engine.session, requests, &engine.languages, &engine.cache, workers)`.
6. `walk::naming_unread` is applied to each report.

`Drop for Linter` calls `close(session)` on the owner thread.

`S/executor.rs` (59 lines, read in full) holds:
```
:14 pub(crate) type Execute = fn(usize, Vec<LintRequest>, &Languages, &Cache, NonZeroUsize) -> Vec<Result<LintReport, LintError>>;
:17 static LINKED: OnceLock<(Execute, fn(usize))>;  :20 pub(super) static NEXT_SESSION: AtomicUsize
:23 pub(super) fn in_use() -> (Execute, fn(usize))   (default: session::execute_round, session::release_session)
:33 #[cfg(test)] pub(super) fn link_executor(execute: Execute, close: fn(usize))
:39 report(findings: Vec<Finding>, unknowns: Vec<Unknown>, unread: Vec<Unread>) -> LintReport   :45 finding(rule: RuleId, subject: Slot, key: FindingKey, place: Place, columns: Vec<FindingColumn>) -> Finding   :51 unknown(rule: RuleId, path: RelativePath, message: Message) -> Unknown   :57 unread(path: RelativePath, reason: Unreadable) -> Unread
```
`S/session/entry.rs`:
```
:37 pub(crate) fn execute_round(id: usize, requests: Vec<sdk::LintRequest>, languages: &sdk::Languages, cache: &sdk::Cache, workers: std::num::NonZeroUsize) -> Vec<Reply>      (Reply = Result<sdk::LintReport, sdk::LintError>, session.rs:36)
:477 fn admit(&mut self, req: sdk::LintRequest, linked: &sdk::Languages, unbounded: bool) -> Result<RoundRequest, sdk::LintError>      (method of Live)
```
Calls into `ryo_executor`, in order (direct, then transitive through `Live`'s methods):
1. `:51` `link_readers` calls `check_session::named::Catalog::from_readers(&readers)` (:288). `natives(...)` builds the `Linked` values (session.rs:49).
2. `:51` `use_cache` calls `replay::stored::Store::directory(p, &self.languages)` (:297), only when `cache` is `At(..)`.
3. `:54` `session.batch.workers = Some(workers)` (a `Batch` field).
4. `:55` `lint` calls `epoch` (:374). Per request it calls `admit` (:382). Inside it:
   - `admit_version` -> `Environment::new` (:432), `schedule.reprovide` (:441), `schedule.provide` (:455), `store.bind_version` (:457), `schedule.listed` (:471);
   - `source::takes_environment` (:428);
   - `batch.prepare_rules(items)` (:605), only without a store;
   - `jobs::run` (:614);
   - `batch.prepared_rule` (:641,677);
   - `program.refuse_unknown_kind` (:679);
   - `batch.register_query(query, Input{..}, |_| Ok(None), Some(LiveAdmission{..}), layouts)` (:685-703);
   - `batch.add_work` (:715).
5. `:392`/`:394` `batch.epoch_with_store(&self.languages, &mut self.state, Some(store))` or `batch.epoch(&self.languages, &mut self.state)`, only if `batch.work` is non-empty.
6. `:400` `deliver` calls `Batch::answer(&batch.work[i], &state.answers)` (:736,766) and `jobs::run(chunks, render_chunk)` (:742). Rendering reads `finish::answer::Words` (:824).
7. `:407` `finish::operators::flush()`.
8. `:58` `session.publish()` calls `store.publish()`.
`batch.admit` is not called by the SDK directly. `register_query` calls it at batch.rs:303, 360 and 380.

Other `S/types.rs` types, with derives and field visibility:
```
:21 #[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord,Hash)] pub struct RuleId(pub(super) Box<str>);  :37 pub struct Glob(pub(super) Box<str>);  (same derives)  :54 FindingKey(pub Vec<Slot>)  :58 Message(pub Box<str>)  :62 ProviderName(pub Box<str>)  :72 #[derive(Clone,Copy,Debug,PartialEq,Eq)] pub struct Workers(pub NonZeroUsize);
:77 #[derive(Clone,Debug,PartialEq,Eq)] #[non_exhaustive] pub enum Cache { None, At(PathBuf) }
:86 pub enum RuleApplicability { #[default] Always, AcceptedSyntax }   :96 pub enum SkipReason { SyntaxRejected }
:103 #[derive(Clone,Debug,PartialEq,Eq)] pub struct Rule { pub observations: Vec<ObservationBinding>, pub query: RyoQlQuery, pub applicability: RuleApplicability, pub parameters: BTreeMap<ParameterName, ParameterValue>, pub include: Vec<Glob>, pub exclude: Vec<Glob> }
:169 pub struct LintRequest { pub source_language: SourceLanguage, pub root: PathBuf, pub project_search_scope: ProjectSearchScope, pub rules: BTreeMap<RuleId, Rule>, pub files: Vec<Glob>, pub hosted: Vec<HostRelation>, pub corpus: Corpus, pub beside: Vec<CaseSource> }
:191 #[non_exhaustive] pub enum Corpus { Walked, Listed(Vec<RelativePath>), Given(Vec<CaseSource>) }
:202 RowNumber(pub usize)  :206 SourceText(pub Box<str>)  :210 CaseSource { pub executable: ExecutablePermission, pub path: RelativePath, pub text: SourceText }
:222 #[non_exhaustive] pub struct Finding { pub rule: RuleId, pub subject: Slot, pub key: FindingKey, pub place: Place, pub columns: Vec<FindingColumn> }   :238 FindingColumn { pub name: ColumnName, pub value: Slot }
:249 #[non_exhaustive] pub struct Unknown { pub rule: RuleId, pub path: RelativePath, pub message: Message }   :261 #[non_exhaustive] pub enum Port { Corpus, Store, Facts, Language, Memory, Scheduler, Output }
:281 #[non_exhaustive] pub struct LintReport { pub findings: Vec<Finding>, pub unread: Vec<Unread>, pub unknowns: Vec<Unknown>, pub skipped: Vec<Skipped>, pub(crate) rows_total: usize }   :297 Skipped { pub rule: RuleId, pub path: RelativePath, pub reason: SkipReason }
:308 HostRelation { pub name: HostRelationName, pub columns: Vec<HostColumn>, pub rows: Vec<HostRow> }  :319 HostRow(pub Vec<Cell>)  :324 ObservationBinding { pub name: ObservationBindingName, pub columns: Vec<ObservationColumn>, pub producers: Vec<ObservationProducer> }  :334 pub enum ObservationProducer { Output { rule: RuleId, output: ObservationName }, NoQuery { rule: RuleId } }
```
`S/errors.rs:11` is `#[derive(Clone,Debug,PartialEq,Eq,thiserror::Error)] #[non_exhaustive] pub enum LintError`. Its variants are:
- `Execution{reason:String}`
- `ObservationExecution{reason:String}`
- `ObservationBindings{reason:String}`
- `SourceLanguage{name:String}`
- `Rule{id:RuleId, error:ValidateError}`
- `Unrunnable{id:RuleId, reason:String}`
- `MissingParameter{id:RuleId}`
- `TooLarge{size:u64, budget:u64}`
- `Provider{provider:ProviderName, code:ProviderCode}`
- `MemoryBudgetExceeded{used:u64, budget:u64, plans:Vec<RuleId>}`
- `DuplicateHostRelation{name:HostRelationName}`
- `HostRowWidth{relation:HostRelationName, row:RowNumber, expected:usize, given:usize}`
- `HostCellKind{relation:HostRelationName, row:RowNumber, column:HostColumnName, expected:HostKind}`
- `PathNotText{path:PathBuf}`
- `ProjectPath{path:RelativePath}`
- `Io{what:Port, reason:String}`

`:141 #[non_exhaustive] pub enum ProviderCode { SpanOutsideFile, NodeOutsideTable, NodeOfAnotherFile, FileNotVisited }`.

`D` (`domain`):
```
D/capacity/types.rs:11 #[derive(Clone, Debug)] pub struct WorkerPool(Arc<dyn SchedulerPort>, WorkerCount);  (pub(crate): scheduler(..), workers(&self)->WorkerCount, of(self)->Arc<dyn SchedulerPort>)
D/capacity/types.rs:34 #[derive(Clone,Copy,Debug,PartialEq,Eq)] #[non_exhaustive] pub enum MemoryBudget { Machine, Bytes(NonZeroU64) }
D/language/types.rs:18 LanguageName(pub Box<str>);  :22 pub enum SourceLanguage { #[default] Recognized, Named(LanguageName) }  :32 LanguageBuild(pub Box<str>)
D/language/types.rs:36 #[derive(Clone)] pub struct Language { name: LanguageName, build: LanguageBuild, engine: Arc<dyn ParseEngine>, project: Arc<dyn ProjectEngine>, walk: Option<Arc<dyn WalkEngine>> }  (all private)
  pub: name(&self)->&LanguageName, primary_role(&self, path: SourcePath<'_>) -> PrimaryFileRole, build(&self)->&LanguageBuild, configuration_identity(&self)->ParseConfigurationIdentity<'_>, skips(&self)->&'static [SkippedDirectory]; pub(crate): from_parts(name, build, engine, project), walking(self, walk), parse_use_case/walk_use_case/project_use_case -> Arc<dyn ...>; cfg(fixtures|bench-hooks) pub: parse_engine, walk_engine, project
D/language/types.rs:176 #[derive(Clone,Debug,Default)] pub struct Languages(pub Vec<Language>);   (doc(hidden) pub fns: skipped(&self) -> BTreeMap<SkippedDirectory, Vec<&LanguageName>>, reads(&self, path: &RelativePath) -> Recognition)
```
`Workers` and `Cache` are only in `S/types.rs` (above). There is no `Workers` or `Cache` file in `D`. The `From<Workers> for WorkerPool` impl is at `S/entry.rs:92`.

## FACTS
- Once per request: `walk::listed` expands the corpus, then `Live::admit`. That calls `prepare_rules`, `prepared_rule` and `register_query` (which calls `Batch::admit`) per rule, then `add_work`. The round ends with `epoch` and `deliver`, and `visit`'s per-reader set-up (`merged`, `bind`) runs once per reader group. Programs are lowered once per distinct `(Query, Parameters)` through `Schedule::prepared`.
- Once per session (`Linked` languages, `Catalog`, the store): `link_readers` and `use_cache`, on the first `execute_round` for that session id. `Linked` is a thin wrapper over three `Arc<dyn>` language-engine traits.
- Once per worker: `dispatch` makes a `World`, a `ReaderState` and a `Clock` per shard. Each shard `visit` rebuilds its `Run` symbols, `Bindings` and merged `Exec` plan. This costs one `merged`/`bind` pass per shard.
- Once per file: `versions`, one `reader.visit`, one `consume`, one `screen.exclude` run, one `FileSummary` reset, one fill of facts, and one `FileVersion`. Files that share a path share a visit.
- Once per (rule, file) pair: a `FileSummary::excludes` test via `b.test`, then (when not excluded) an `excluded`/`certified` check. Otherwise a `finish::session::pair` evaluation or, for closed programs, `contribute`. The result goes into `answers.set(slot, ..)`.
- The `.visit` language callback is invoked once per file. `visit` asserts that a file version is supplied exactly once.
- Screening has no inbound calls from `batch.rs` or `worker.rs`. Every call goes through `visit` in `check_session.rs`.
