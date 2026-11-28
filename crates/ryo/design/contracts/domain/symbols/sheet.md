# domain/symbols
STATUS: drafted (28 Nov; not reviewed, not agreed). Bead ryo-fmx3.1.13.23.4.4. The crate's name is decision 1.

Revisions: MAIN = f4ae3bd802, TIP = 2a207464c, B133 = f0219b3b3. Parked: b4210de05c, fae1469dd7, 89d13a19ee (24 Nov).
Every quote is one line, two spaces, `@rev:path:line`; all were re-checked with `git show "REV:PATH" | sed -n 'Np'`
(section 11 gives the count). "X" below is `crates/ryo/crates/executor/src` at TIP.

## 1. Owns
The run's text dictionary: each distinct text gets one dense number, and a number gives its text back.
(One thing. Case folding, the language-code tables, the reserved words' bookkeeping, file paths and the answers'
words are other crates' and leave the dictionary; see 5, "What leaves".)

## 2. On main (f4ae3bd802)
**There is no run dictionary on main.** A name is a number only inside one file, and crosses files by hash.
- Within a file: `ryo_language::domain::symbols` (the per-file name table; 21 public items, 16 functions and 5 types,
  `service.rs` and `types.rs`). A symbol is a place in the file's bytes plus a hash, or a "derived" text kept beside
  the file's names:
  ```
  pub struct Symbols(SymbolsEngine);  @f4ae3bd802:crates/ryo/crates/language/src/domain/symbols/types.rs:31
  pub fn intern_derived(symbols: &mut Symbols, source: ByteSlice<'_>, text: NameText<'_>) -> Symbol {  @f4ae3bd802:crates/ryo/crates/language/src/domain/symbols/service.rs:256
  pub fn find_symbol(  @f4ae3bd802:crates/ryo/crates/language/src/domain/symbols/service.rs:149
  pub fn vocabulary_symbol(  @f4ae3bd802:crates/ryo/crates/language/src/domain/symbols/service.rs:313
  pub struct Symbol(pub u32);  @f4ae3bd802:crates/ryo/crates/relations/src/domain/terms/types.rs:76
  pub struct ByteSlice<'a>(pub &'a [u8]);  @f4ae3bd802:crates/ryo/crates/language/src/domain/symbols/types.rs:142
  ```
- Across files: a name is `Name { hash, place }`, equal by hash alone (after a spelling-collision check), its text in
  the source file's bytes or in the arena of the worker that made it:
  ```
  pub struct Name {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:118
  pub struct WorkerNumber(pub u32);  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:78
  pub enum Origin {  @f4ae3bd802:crates/ryo/crates/relations/src/domain/relations/types.rs:85
  pub(crate) fn check_spellings<'a>(  @f4ae3bd802:crates/ryo/crates/execution_domain/src/domain/spelling/service.rs:20
  ```
- What operators were handed: three text methods on `Resources` (identical at B133: `git diff --stat f4ae3bd802
  f0219b3b3` over the two files is empty), and the borrowed `Texts`:
  ```
  pub trait Resources {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:142
      fn texts(&self) -> Texts<'_>;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:144
      fn intern(&mut self, text: CellText<'_>) -> Value;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:146
      fn mint(&mut self, text: CellText<'_>) -> Value;  @f4ae3bd802:crates/ryo/crates/operators/src/domain/physical_plan/types.rs:148
  pub struct Texts<'a> {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/table_texts/types.rs:45
  pub fn symbol_text<'a>(texts: &Texts<'a>, symbol: Symbol) -> Option<TextView<'a>> {  @f4ae3bd802:crates/ryo/crates/operators/src/domain/table_texts/service.rs:16
  ```
- Implemented by three types, and in all three `intern` and `mint` do the same thing (the fixture only records which
  was called):
  ```
  impl Resources for PassResources<'_> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/service.rs:296
          self.mint(text)  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/file_pass/service.rs:306
  impl Resources for PhaseResources<'_> {  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service/inputs/service.rs:67
          self.mint(text)  @f4ae3bd802:crates/ryo/crates/execution_runtime/src/application/file_dag/project_pass/service/inputs/service.rs:77
  impl Resources for TextResources {  @f4ae3bd802:crates/ryo/crates/operators/src/inbound/use_cases/run/fixtures.rs:268
  ```
- Callers: 9 kernel call sites in operators (`extend`, `ordered_walk`, `render`, `scan_bound`, `text_map`; by
  `git grep -n "\.mint(\|resources\.intern("`); `Texts` is named in 59 files (operators 32, execution_domain 7,
  execution_runtime 6, query_language 6, execution 3, values 2, core 1, sdk 1, cli tests 1).
- Proof: `table_texts` has 4 `#[test]` (`service_test.rs` 1 over a case table; `derived_texts/value_test.rs` 3:
  `a_name_reads_as_its_text`, `a_file_reads_as_its_path`, `a_number_has_no_text`). The per-file table has 2 `#[test]`
  over 13 case functions (`service_test.rs`, `service_test/storage_test.rs`); they are the language's, not this crate's.
- Tastemaker: no entry names a run dictionary. The SDK spec re-exports the per-file items (`Symbols`,
  `intern_derived`, `vocabulary_symbol`, `text_of`, `ByteSlice`) at `crates/ryo/tastemaker.toml:134`.

## 3. On the perf branch (2a207464c)
The run dictionary exists, as fields and methods of the executor's `Run` (one struct, 35 public fields), per worker,
built on `Rc` and therefore not sendable:
```
pub struct Run {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:301
    pub texts: Vec<Rc<[u8]>>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:308
    pub ids: FxHashMap<Rc<[u8]>, u32>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:309
    pub lower_symbols: FxHashMap<u32, u32>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:313
    pub kind_sym: Vec<u32>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:317
    pub field_sym: Vec<u32>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:318
    pub paths: std::sync::Arc<Vec<String>>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:321
    pub answer_words: Option<std::sync::Arc<std::sync::RwLock<answer::Words>>>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:322
    pub empty: u32,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:323
    pub absent_word: Option<u32>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:325
    pub unresolved_words: Vec<u32>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:327
    pub link_sym: [u32; 3],  @2a207464c:crates/ryo/crates/executor/src/finish.rs:335
    pub fact_word_base: Option<ryo_language::domain::facts::types::BaseWords>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:351
    pub base_word_sym: Vec<u32>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:352
    pub public_word_ids: Rc<FxHashMap<u32, u32>>,  @2a207464c:crates/ryo/crates/executor/src/finish.rs:353
    pub fn links(&mut self) {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:357
    pub fn sym(&mut self, t: &[u8]) -> u32 {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:360
    pub fn lower_symbol(&mut self, symbol: u32) -> u32 {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:370
    pub fn text(&self, s: u32) -> &[u8] {  @2a207464c:crates/ryo/crates/executor/src/finish.rs:379
```
Beside it, in three files:
```
pub struct SharedWords<'a>(Vec<(&'a [u8], bool)>);  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:12
    pub fn share_words(&self) -> SharedWords<'_> {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:15
    pub fn install(&self, run: &mut super::Run) {  @2a207464c:crates/ryo/crates/executor/src/finish/population.rs:25
pub struct Words {  @2a207464c:crates/ryo/crates/executor/src/finish/answer.rs:18
    pub fn symbol(&mut self, run: &Run, id: u32) -> Result<&Arc<str>, crate::check_session::refusal::Refusal> {  @2a207464c:crates/ryo/crates/executor/src/finish/answer.rs:269
    pub fn path(&mut self, text: &str) -> Result<u32, crate::check_session::refusal::Refusal> {  @2a207464c:crates/ryo/crates/executor/src/finish/answer.rs:281
pub fn public_words(run: &mut Run, reasons: &[l::UnknownName]) -> HashMap<u32, u32> {  @2a207464c:crates/ryo/crates/executor/src/reader.rs:31
pub fn symbol(run: &mut Run, unknown: &HashMap<u32, u32>, symbols: &l::Symbols, bytes: &[u8], word: l::Symbol) -> u32 {  @2a207464c:crates/ryo/crates/executor/src/reader.rs:52
```
That is **25 public items** for the concept in the executor (13 fields, 4 methods of `Run`; 3 in `population.rs`;
`answer::Words` and its 2 methods; 2 functions in `reader.rs`).

**A second dictionary of the same shape, in the language crate** (new on the tip; absent on main), already frozen and
shared: the run's base words behind an `Arc`, and one file's words coded after them.
```
pub struct BaseWords(pub(crate) Arc<BaseSet>);  @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:193
pub struct FileWords(pub(crate) WordsEngine);  @2a207464c:crates/ryo/crates/language/src/domain/facts/types.rs:223
pub fn base_words(fixed: CheckedNames<'_>, texts: &[NameText<'_>]) -> BaseWords {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:56
pub fn fixed(words: &FileWords, word: FixedWord) -> CellWord {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:110
pub fn file_words(base: &BaseWords) -> FileWords {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:133
pub fn clear_words(words: &mut FileWords) {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:154
pub fn base_count(words: &FileWords) -> CellWord {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:175
pub fn base_changed(words: &FileWords, previous: &mut Option<BaseWords>) -> bool {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:193
pub fn words_end(words: &FileWords) -> CellWord {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:217
pub fn text(words: &FileWords, cell: CellWord) -> NameText<'_> {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:240
pub fn intern(words: &mut FileWords, text: NameText<'_>) -> CellWord {  @2a207464c:crates/ryo/crates/language/src/domain/facts/service.rs:274
```
11 public items. `FileWords` is re-exported by the SDK to language authors (`pub use ryo::languages::FileWords`,
tip `sdk/api.txt:132`); every language's fills take `&mut FileWords`.

**How a name travels today, per file to run-wide.** (1) A language fills a column with `CellWord` codes from its
`FileWords`: codes below `base_count` are the shared base, the rest the file's own. (2) When the executor binds a
symbol column of that file, it translates the base once per `BaseWords` (identity by `Arc::ptr_eq` in
`base_changed`) and the file's own words one by one, each through `Run::sym`:
```
                    if l::facts::base_changed(words, &mut run.fact_word_base) {  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:108
                            .map(|id| run.sym(l::facts::text(words, CellWord(id)).0.as_bytes()))  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:110
                    own.extend((next..end).map(|id| run.sym(l::facts::text(words, CellWord(id)).0.as_bytes())));  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:116
```
(3) A name read through the per-file `Symbols` goes through `reader::symbol` (text, then `Run::sym`; a name with no
text gets a reserved word). (4) Kernels intern computed texts directly, per row, for example
```
                        row.push(u64::from(run.sym(&t)));  @2a207464c:crates/ryo/crates/executor/src/finish.rs:2361
```
(5) Words cross workers in one place, the cross-file ("closed") workers, by copying every word into each worker:
```
    let words = world.run.share_words();  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/closed_workers.rs:249
        words.install(&mut run);  @2a207464c:crates/ryo/crates/executor/src/check_session/replay/closed_workers.rs:267
```
(6) Words that outlive the worker go through the lock-guarded `answer::Words` (only the words a kept answer names).

- Callers (by `git grep`, TIP, all under `X` unless said): `.sym(` on 37 lines in 14 files; `.text(` of the run on
  24 lines; `share_words` 1 caller, `install` 1; `absent_word` 20 lines in 10 files; `.kind_sym` 13 lines in 9 files;
  `.field_sym` 9 in 8; `.link_sym` 5 in 3; `public_word_ids` 4 in 3; `base_word_sym` 2 in 1. `.texts` matches 74
  lines in 33 files but that pattern also hits other structs' fields (not separated).
- Proof on the tip: 2 `#[test]` over 10 case rows in `language/src/domain/facts/service_test.rs` (the layered
  dictionary: `a_base_text_keeps_its_base_code`, `a_file_text_is_coded_after_the_base_words_once`,
  `a_file_word_reads_back_as_its_text`, `the_last_file_words_are_forgotten`, `word_zero_is_the_empty_text`,
  `a_fixed_word_is_its_place_after_the_empty_text`, `a_fixed_word_is_the_same_cell_in_every_file_of_the_run`,
  `a_fixed_word_is_the_word_its_text_interns_to`, `the_base_texts_follow_the_fixed_names`,
  `a_handle_past_the_fixed_names_fails_the_assertion_of_a_test_build`); in the executor 3 tests touch the run
  dictionary:
  ```
      fn resolving_published_terms_leaves_room_only_for_file_symbols() {  @2a207464c:crates/ryo/crates/executor/src/reader_run/packet.rs:600
      fn absent_symbol_matches_the_empty_literal_and_keeps_its_identity() {  @2a207464c:crates/ryo/crates/executor/src/pair_checks.rs:1348
      fn reserved_cells_are_not_file_dictionary_offsets() {  @2a207464c:crates/ryo/crates/executor/src/finish/fact_view.rs:181
  ```
  `share_words`/`install` have no test.
- Tastemaker: none names the run dictionary (same SDK re-export entry, tip `crates/ryo/tastemaker.toml:124`).

## 4. What changed between them
- **Added on the tip:** the run dictionary itself (main had none: a per-file number inside a file, a hash across
  files). It arrived with the vector executor, which lived in `crates/engine/benches/vec` and was moved to
  `crates/executor/src` by `f2869d1f74` (26 Nov, "renames only"). Steps found by `git log -S` on the tip:
  `90d6196bbe` (22 Nov, "fill text cells through a word cache": perf row 39), `28819d0c2a` (23 Nov, "Bind shared
  text predicates to immutable reader symbols": rows 21/22), `67fc9724bc` (24 Nov, "Preserve absent and unresolved
  symbol ordinals in the closed worker seed": `share_words` with its `bool`), `fcac638588` (25 Nov, "Translate
  invariant provider words once per shared base": `base_word_sym`). `BaseWords` first appears in `3097109506`
  (20 Nov, "the engine contract beside today's engine").
- **Removed:** `Resources::{texts, intern, mint}` and `Texts` went with the `operators` crate (absent on the tip).
  Main's cross-file `Name { hash, place }` is still declared on the tip
  (`@2a207464c:crates/ryo/crates/relations/src/domain/relations/types.rs:119`); whether anything on the tip still
  compares names by hash is not checked.
- **Changed meaning, same word:** main's `Symbol` is a per-file number; on the tip the executor's "symbol" (`Ty::Sym`,
  `sym`, `lower_symbol`, `kind_sym`) is the run number, a bare `u32`, while `relations::terms::Symbol` keeps the
  per-file meaning. Two concepts share one word.
- **The parked many-worker work (24 Nov, on no branch; merge base with the tip `263836c564` for all three).** The
  frozen form was **a snapshot copied into every worker, not one shared copy**: the dictionary became one byte buffer
  plus offsets, and a hash table holding only numbers; the run prepared a `Settings` once and each worker's `open`
  cloned both into its own `Run`. Literal numbers inside the shared prepared plan stay valid because every worker
  starts from the same snapshot ("These literal IDs precede the dictionary snapshot used by every child"):
  ```
  pub(crate) struct Texts {  @b4210de05c:crates/ryo/crates/engine/benches/vec/finish.rs:367
      bytes: Vec<u8>,  @b4210de05c:crates/ryo/crates/engine/benches/vec/finish.rs:368
      offsets: Vec<usize>,  @b4210de05c:crates/ryo/crates/engine/benches/vec/finish.rs:369
      pub(super) ids: hashbrown::HashTable<u32>,  @b4210de05c:crates/ryo/crates/engine/benches/vec/finish.rs:412
      pub(super) fn lookup_word(&self, text: &[u8]) -> Option<u32> {  @b4210de05c:crates/ryo/crates/engine/benches/vec/finish.rs:462
      text: finish::Texts,  @b4210de05c:crates/ryo/crates/engine/benches/vec/finisher/worker.rs:21
          run.texts.clone_from(&self.text);  @b4210de05c:crates/ryo/crates/engine/benches/vec/finisher/worker.rs:102
          run.ids.clone_from(&self.ids);  @b4210de05c:crates/ryo/crates/engine/benches/vec/finisher/worker.rs:103
          // These literal IDs precede the dictionary snapshot used by every child.  @b4210de05c:crates/ryo/crates/engine/benches/vec/finish/worker_schemas.rs:136
      pub fn fresh(&self) -> Self {  @b4210de05c:crates/ryo/crates/language/src/domain/facts/types.rs:171
  ```
  The same `Texts` / `HashTable<u32>` / clone-per-worker form is at `fae1469dd7` and `89d13a19ee` (same lines in
  `finish.rs:411-412` and `finisher/worker.rs:17,21,64,76,102,103`); those two commits themselves change shared view
  shapes, not the dictionary. The plan's row 13 says "symbol tables frozen once and shared"; the code shows frozen
  once and copied.
- **Lost that should not have been:** main's per-file `find_symbol` (a lookup that does not add) has no run
  counterpart on the tip; the parked `lookup_word` was its run form and never landed. The two `Texts`-era tests
  (`a_name_reads_as_its_text` and siblings) went with `operators`; they belong to the operators sheet.

## 5. The drafted public interface        (explains every line of api.txt that is changed or new)
`api.txt` beside this sheet: **15 public items** (2 traits, 5 methods, 5 types, 3 inherent functions) plus 5 impl
lines. Working names as in the plan's Appendix A §3.2; decision 1 renames them. Corrections to Appendix A and to
the earlier draft of `api.txt` that this pass replaced:
- `freeze(self) -> FrozenSymbols` on the use case and `FrozenSymbols::worker() -> impl SymbolsUseCase + '_` become a
  concrete pair, `SymbolLayer::freeze` and `FrozenSymbols::layer`, because the tip already has exactly that pair live
  in the language (`base_words`/`file_words`), owned rather than borrowed (a layer outlives a scoped borrow when it
  is handed to answers).
- Dropped from the earlier draft: `lowered` (case folding is the `lower` function's cache: operators, by the owner's
  "functions in operators"); `hash` and `TextHash` (no run caller on the tip; the per-file hash stays in language);
  the `FixedWords` trait, `WordSet`, `SetMember`, `NameText`, `NO_SYMBOL` (kind/field/link tables and
  empty/absent/unresolved are built by interning and kept by their caller; one call, `reserve`, is what the
  dictionary must add for them).
- The earlier draft tagged `Symbol(pub u32)` `[main]` and said the language would import `Symbol` from this crate.
  Wrong: main's `Symbol` is the per-file number. The run number is a new type with a colliding name (decision 1).

**The calls.**
```
pub trait SymbolsUseCase: sealed::Sealed {                       @new
    fn count(&self) -> SymbolCount;
    fn find(&self, text: SymbolBytes<'_>) -> Option<Symbol>;
    fn text(&self, symbol: Symbol) -> SymbolBytes<'_>;
}
pub trait InternUseCase: SymbolsUseCase {                         @new
    fn intern(&mut self, text: SymbolBytes<'_>) -> Symbol;
    fn reserve(&mut self, shown: SymbolBytes<'_>) -> Symbol;
}
```
- `intern` is the tip's `Run::sym` (`finish.rs:360`) and the language's `facts::intern` (`service.rs:274`); main's
  `Resources::intern` and `Resources::mint` are one call because every implementation answers them alike (section 2).
  Bytes, not `&str`: the tip's dictionary keys `&[u8]`, and main's `text_of` returns no text for non-UTF-8 names.
- `reserve` gives a number whose text reads back but which `intern` never finds: the tip does this by hand for the
  absent word and each unresolved name, outside the hash index:
  ```
          run.texts.push(Default::default());  @2a207464c:crates/ryo/crates/executor/src/reader.rs:35
          run.texts.push(format!("unknown: {}", reason.text.0).into_bytes().into());  @2a207464c:crates/ryo/crates/executor/src/reader.rs:40
  ```
  and `share_words` carries a `bool` per word only to keep that distinction across workers (`67fc9724bc`).
- `find` is the parked `lookup_word` (`b4210de05c` above): screening and a shared plan ask "is this text a constant
  of any program" without growing a worker's dictionary.
- `text` is `Run::text` (`finish.rs:379`) and `facts::text` (`service.rs:240`). `count` is `run.texts.len()`
  (`reader.rs:34`) and `facts::words_end` (`service.rs:217`).

**How it is built.**
```
pub struct FrozenSymbols;          // Send + Sync; Clone shares (an Arc inside), Default is empty      @new
impl FrozenSymbols { pub fn layer(&self) -> SymbolLayer; }
pub struct SymbolLayer;            // Send, not Sync: a frozen base plus additions numbered from its count
impl SymbolLayer { pub fn clear(&mut self); pub fn freeze(self) -> FrozenSymbols; }
```
At load (`programs`), the edge interns every program constant, every language's fixed names and the reserved words
into a layer over `FrozenSymbols::default()` and freezes it. Each worker takes `layer()`: no copy, numbers below the
frozen count identical in every worker, so a shared prepared plan can hold `Symbol` constants (perf row 13). A
language's file layer is `layer()` again, `clear()`ed per file (today's `clear_words`). At a worker's end, `freeze`
turns its layer into a dictionary handed to answers (decision 5). This is the tip's `BaseWords`/`FileWords` pair made
the run's (decision 2, option B).

**The types.** `Symbol(pub u32)` is the tip's bare run number (`Run::sym -> u32`) given a type, guaranteeing what
gains 21/22 rely on: dense from zero and never reused in a run (the answer pages index `id / 512`):
```
    pub fn answer(&self, id: u32, test: impl FnOnce() -> bool) -> bool {  @2a207464c:crates/ryo/crates/executor/src/finish/symbol_predicate.rs:43
```
`SymbolBytes<'a>(pub &'a [u8])` is main's `ByteSlice` (section 2) under a word that says what the bytes are.
`SymbolCount(pub u32)` is new. No error type: see api.txt "failures".

**What leaves** (all listed by item in api.txt `# NO LONGER PUBLIC`): `lower_symbols`/`lower_symbol` to operators;
`kind_sym`, `field_sym`, `link_sym`, `links`, `base_word_sym`, `public_word_ids`, `empty`, `absent_word`,
`unresolved_words`, `reader::{public_words, symbol}` to application/execute (its run state, built with `intern` and
`reserve`); `paths` and `answer::Words::path` to whichever crate numbers files; `answer_words`/`answer::Words` replaced
by a `FrozenSymbols`.

| Side | Public items in the concept | After the draft |
|---|---|---|
| main | 0 run-dictionary items; 5 that serve a cell's text (`Resources::{texts, intern, mint}`, `Texts`, `symbol_text`) | leave or go |
| tip, executor | 25 | 9 map onto the draft, 16 leave for other crates |
| tip, language `facts` | 11 | 10 map onto the draft if decision 3 is taken; `fixed` stays in language |
| draft | **15** (+5 impl lines) | |

## 6. Proof it brings
| Proof | Source | Fate |
|---|---|---|
| the 10 case rows of the layered dictionary | `language/src/domain/facts/service_test.rs` (tip; rows at lines 47-101) | **restated** against `SymbolLayer`/`FrozenSymbols` (base code kept, additions after the base once, read back, `clear` forgets, word zero is the empty text); the 4 fixed-word rows stay in language with `fixed` |
| `absent_symbol_matches_the_empty_literal_and_keeps_its_identity` | `X/pair_checks.rs:1348` | **restated** as a dictionary case: `reserve(b"") != intern(b"")`, same text |
| `resolving_published_terms_leaves_room_only_for_file_symbols` | `X/reader_run/packet.rs:600` | goes with `reader` to application/execute; it uses the dictionary only through `intern`/`reserve` |
| `reserved_cells_are_not_file_dictionary_offsets` | `X/finish/fact_view.rs:181` | the store's/execute's (cell namespace), not this crate's |
| main's `table_texts` 4 tests | `operators/src/domain/table_texts/**` (main) | operators sheet (`datum`) |
| main/tip per-file `symbols` tests (13 cases) | `language/src/domain/symbols/service_test*` | stay in language |
| numbers below a frozen count are the same in every layer; a reserved word stays reserved across `freeze`/`layer` (the `67fc9724bc` bug) | none | **MISSING** on both sides: `share_words`/`install` have no test |
| `FrozenSymbols: Send + Sync`, `SymbolLayer: Send` | none | **MISSING**: a compile-time assertion |
| a `compile_fail` doctest that the layout cannot be reached | none | **MISSING** (precedent: `BaseWords`'s `compile_fail,E0603`, tip `facts/types.rs:179`) |

## 7. Perf gains it carries
From plan §10.3 and `discovery-inputs/perf-history.md` (figures not re-measured here):
- **Row 13** (parked; ten cores fastapi 3.62 -> 1.38 CPU-s, 838 -> 387 MiB): needs a dictionary that is built once,
  read by every worker, and numbers below its count identical in every worker. Interface: `FrozenSymbols` (Send +
  Sync, sharing `Clone`) and `layer()`. The parked code met this by copying; the draft meets it without the copy.
- **Row 21** (literal glob compiled once, answer reused per interned text; 57.2 -> 36.2) and **row 22** (symbol
  tests bound once; regex calls 325.3 M -> 8.7 M): the code is operators' (functions), but both key their answer
  cache by the run number (`symbol_predicate.rs:43`). Interface: `Symbol` dense from zero, stable for the run, one
  text one number.
- **Row 39** (text cells to symbol ids through a word cache; Django 16.29 -> 13.93) and its follow-up `fcac638588`
  (base translated once per shared base): the translation lives in execute today. Interface: a frozen base whose
  numbers are fixed, so translating it is once per run (or, with decision 3, no translation at all).
- Lost and to make room for: the shared prepared plan of row 13 (constants as `Symbol`s in an `Arc`ed plan) needs
  exactly `FrozenSymbols` + `find`.

## 8. Benchmarks that cover it
- No benchmark names the run dictionary. It is exercised end to end by the lint images (`vec`, `vec_ts`,
  `vec_rust`, `tm`, `crates/ryo/crates/executor/benches/lint_image`), which call the SDK and name nothing here; the
  ten-worker wall column of the bar table is the gate for row 13.
- The Python fillers bench calls the language's word calls by name and must follow decision 3:
  ```
          languages::facts::clear_words(&mut words);  @2a207464c:crates/ryo/languages/python/benchmarks/provider/performance/fillers/bench.rs:61
  ```
  (and `walk_words` from `ryo_python::fixtures::walking`); after the draft it calls `SymbolLayer::clear`, measuring
  the same thing.
- Nothing benchmarks `intern`/`text` alone; a micro-bench is not proposed (the images are the measure).

## 9. Checks
IMPORTS: none
CROSSED: per cell
- Imports: no Ryo crate; outside, `hashbrown` (the tip's language `WordSet` and the parked `HashTable<u32>`) and
  `rustc-hash` (`FxBuildHasher`, used by both). Relations, operators, the store, execute and every language may
  import it (domain base, beside `table`).
- Layout words: none in a signature (no `Vec`, map, `Rc`, `Arc`, offset or `ids`).
- Crossing: `intern` and `text` are called per cell in kernels today (`finish.rs:2361`). The owner's rule is "once
  per file or per set, never per row or cell, with the type fixed at compile time". The draft keeps the type fixed:
  `SymbolLayer` and `FrozenSymbols` are concrete, so no call is virtual and each inlines across the crate under the
  release profile (`lto = "fat"`, `codegen-units = 1`, per the table sheet 8b). The per-cell count itself is the
  operators' and execute's to cut; this crate cannot.
- Sealed traits: only this crate implements `SymbolsUseCase`/`InternUseCase`; callers are generic or name the two
  concrete types.

## 10. Decisions for the owner              (each: the example from the code; why; what we get; my recommendation)
1. **The name.** Example: `ryo_relations::domain::terms::Symbol` (`relations/src/domain/terms/types.rs:76`) is one
   file's name number, re-exported to language authors (`pub use ryo::languages::Symbol`, tip `sdk/api.txt:411`) and
   used by 9 languages; the run number has no public name today (a bare `u32`). Options:
   (a) keep `symbols`/`Symbol` and rename the per-file side (`Symbols` -> e.g. `FileNames`): an SDK break for every
   language; (b) `words`: `Word`, `WordLayer`, `FrozenWords`, matching the tip's `BaseWords`/`FileWords`/`CellWord`,
   but `ryo_relations::...::Words(pub Vec<u64>)` (a buffer of cell words) is already used by the languages' relation
   items; (c) `dictionary`: `TextId`, `Dictionary` (the layer), `FrozenDictionary`, `DictionaryUseCase`,
   `InternUseCase`; the executor already says "dictionary" for this 74 times, and its one `Dictionary` type
   (`X/finish/cold_text.rs:68`) goes with the rewrite. Why: two concepts under one word is the confusion the plan
   already hit. **Recommend (c).**
2. **The frozen shareable form.** Examples: (A) the parked snapshot, copied into every worker
   (`finisher/worker.rs:102-103` at `b4210de05c`); (B) a frozen base read by all plus a layer per worker, as the
   tip's `BaseWords`/`FileWords` already do (`facts/types.rs:193,223`); (C) no run dictionary at all, main's
   per-file number plus `Name { hash, place }` across files. Why: row 13 is the largest parked gain and failed on
   exactly this. What we get with B: no copy per worker, the numbers of the shared plan valid everywhere, one
   pattern already live and tested (10 case rows). Width stays `u32` (the tip's). **Recommend B.**
3. **One dictionary for the languages and the run.** Example: `packet.rs:108-116` translates each file's `FileWords`
   codes into run numbers through `Run::sym`, and `base_word_sym` caches the base. Why: one definition per concept;
   the two have the same shape (frozen base + additions) and differ only in lifetime, which `clear` covers. What we
   get: the language's 11 items and the executor's 9 become one crate's 15; `FileWords` in the SDK becomes this
   crate's layer. Whether a language then interns straight into the worker's layer (no translation, no per-file
   dictionary) changes the cost of gain 39 and must be measured first. **Recommend: merge the type now; decide
   direct interning on a measurement.**
4. **Is `mint` still needed?** Example: on main every implementation answers `mint` as `intern`
   (`file_pass/service.rs:306`, `project_pass/.../inputs/service.rs:77`; the fixture records which was called but
   makes the same text). The tip has one call (`Run::sym`). **Recommend: drop `mint`.**
5. **What the answers hold.** Example: `answer::Words` (`answer.rs:18`) is a lock-guarded map from run number to
   `Arc<str>`, written while workers run so a kept finding can be decoded later. Why: a lock in the hot path of
   emitting, and a third dictionary. What we get: a worker's layer `freeze`s at its end and the answers keep that
   `FrozenSymbols`; decoding reads `text`. The plan's review already asked for "packed rows and a frozen dictionary"
   (`discovery-inputs/review-performance.md:68`). **Recommend: answers hold a `FrozenSymbols`.**

## 11. Not checked
- Quotes: all 82 `@rev:path:line` quotes in this sheet were re-checked by script against `git show "REV:PATH"`
  (exact line text after trimming): 82 match, 0 differ. The inline `path:line` references in prose either point at a
  line quoted in a code block or were checked by hand one at a time (`relations/.../types.rs:119`, `facts/types.rs:179`,
  `sdk/api.txt:132,411`, `tastemaker.toml:124,134`, `cold_text.rs:68`, `check_session.rs:845`, `reader.rs:34`).
  `api.txt` carries tags, not quotes; its lines rest on the quotes here.
- How the tip runs file workers: one `Run` per check session (`pub run: Run` in `World`,
  `X/check_session.rs:845`); whether per-file work is spread over several `Run`s, and so whether "every worker
  prepares every program again (1.637 s against 0.351 s)" (plan §8.2) describes the tip, was not traced. The figure
  is the plan's, not re-measured.
- Whether anything on the tip still compares main's `Name` by hash; whether `check_spellings` survives.
- The caller counts in section 3 are `git grep` line counts, not call graphs; `.texts` is not separated from other
  structs' fields named `texts`.
- Perf rows 13, 21, 22, 39 are quoted from the plan and `perf-history.md`; no measurement was taken. The cost of
  `find` against the base then the layer (two probes for a new word) against one probe today is not measured.
- Option B's claim "no copy" assumes the frozen base is never rebuilt mid-run; the tip's `base_changed` exists because
  a language's base can change between files (per language). With several languages in one run the frozen dictionary
  must hold every language's fixed names before freezing; whether all are known at load is not checked.
- Lines without tests for the concept were not counted (its code is scattered inside `finish.rs`, 4,388 lines).
- No `tm check` was run on a skeleton of the draft; the nominal-type rules (no bare `Option` in interface modules)
  may object to `find -> Option<Symbol>`.
- B133's engine `TextRowDriver::intern` (`engine/src/domain/stages/types.rs:310` at `f0219b3b3`) interns into
  `FileWords` too; it was read, not traced further.
