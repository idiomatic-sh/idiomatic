//! Python file visit and reusable provider state feeding the shared executor.
use super::{ColFacts, Vocab, files, finish, finisher, flag, thread_cpu};
use ryo::languages;

pub(super) struct Finisher<'p> {
    inner: finisher::Finisher<'p>,
    fwords: languages::FileWords,
    completed: super::completed_facts::Completed,
    type_paths: super::type_expression::Paths,
    input_layout: super::composition_inputs::Layout,
    prefix_symbols: Vec<u32>,
    implicit: Option<(u32, finish::Table)>,
    filled: languages::FilledColumns,
    direct_memo: ryo_python::fixtures::columns::ColumnMemo,
    project: super::provider_packaging::Project,
}

impl<'p> std::ops::Deref for Finisher<'p> {
    type Target = finisher::Finisher<'p>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl std::ops::DerefMut for Finisher<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl<'p> Finisher<'p> {
    pub(super) fn new(
        plan: &'p finish::Plan,
        walk: &impl languages::Walk,
        vocab: &Vocab,
        args: &[&str],
    ) -> Self {
        let t0 = thread_cpu();
        let list = files(args[1]);
        let root = format!("{}/", args[2].trim_end_matches('/'));
        let mut run = finish::Run::default();
        run.empty = run.sym(b"");
        // redo-fills pass 2: the suppliers declare their relations beyond the provider's relations catalog
        super::fact_catalog::declare(&mut run);
        run.links();
        run.root_dir = args[2].trim_end_matches('/').to_owned();
        run.kind_sym = vocab
            .kind_names
            .clone()
            .iter()
            .map(|k| run.sym(k.as_bytes()))
            .collect();
        run.field_sym = vocab
            .field_names
            .clone()
            .iter()
            .map(|k| run.sym(k.as_bytes()))
            .collect();
        run.paths = list
            .iter()
            .map(|p| p.strip_prefix(&root).unwrap_or(p).to_owned())
            .collect();
        run.path_sym = run
            .paths
            .clone()
            .iter()
            .map(|p| run.sym(p.as_bytes()))
            .collect();
        // redo-general: the provider's token order (its lexical model, `provider_tokens.rs`)
        run.set_tokens(
            std::rc::Rc::new(super::provider_tokens::Lexicon::new(&|n| {
                vocab.kinds.get(n).copied()
            })),
            vocab.kind_names.len(),
        );
        let input_layout = super::composition_inputs::Layout::from_catalog(&mut run);
        let mut me = Self {
            completed: Default::default(),
            type_paths: Default::default(),
            input_layout,
            prefix_symbols: Vec::new(),
            implicit: None,
            filled: Default::default(),
            direct_memo: Default::default(),
            inner: finisher::Finisher::new(plan, run, args),
            fwords: ryo_python::fixtures::walking::walk_words(walk)
                .expect("the walk's fixed names"),
            project: super::provider_packaging::Project::new(
                args[2].trim_end_matches('/'),
                super::provider_packaging::Project::search_roots_of(
                    flag(args, "--request").as_deref(),
                ),
                &|n| vocab.kinds.get(n).copied(),
            ),
        };
        me.time = thread_cpu() - t0;
        me
    }

    pub(super) fn prepare_requests(&mut self, vocab: &Vocab, drivers: &[(usize, usize)]) {
        self.inner.prepare_requests(drivers);
        let start = thread_cpu();
        let language = super::python_language(vocab);
        self.inner.exec.prepare_spellings(&self.inner.run, &language.spelled, language.is_identifier);
        self.inner.time += thread_cpu() - start;
    }
    pub(super) fn end(self, walk_time: f64) {
        self.inner.end(walk_time, |locations| locations.dedup());
    }
    pub(super) fn prefix_storage(&mut self, file: usize) -> (&finish::Run, &mut languages::FileWords, super::completed_facts::FileWitness<'_, 'p>) {
        self.completed.begin(languages::FileId(file as u32));
        let run = &self.inner.run;
        let witness = super::completed_facts::FileWitness { completed: &mut self.completed, exec: &mut self.inner.exec, run };
        (run, &mut self.fwords, witness)
    }

    pub(super) fn prefix_table(&mut self, name: &str, t: &ryo_python::fixtures::columns::FactTable, words: &super::facts::Interner) {
        if t.rows == 0 { self.inner.exec.walk_empty(&self.inner.run, name); return; }
        let keys = (t.layout == ryo_python::fixtures::columns::Layout::PerNode).then(|| t.keys());
        self.inner.exec.prefix_symbols(&self.inner.run, name, &t.columns, keys, words.matches.len(), |text| words.ids.get(text));
    }

    /// The second half on one walked file of the direct-columns walk (its own model by parts, built on demand).
    pub(super) fn column_file(&mut self, file_no: usize, path: &str, file: &mut ryo_python::fixtures::columns::ColumnFile<'_, '_>, bytes: &[u8], wanted: &[usize], prefix: Option<super::direct_fill::Prefix<'_>>) {
        let t0 = thread_cpu();
        file.set_file(languages::FileId(file_no as u32));
        if !wanted.is_empty() && file.plain_spellings() { self.inner.exec.source_spellings(bytes); }
        for &rel in &file.empty { self.inner.exec.walk_empty(&self.inner.run, rel); }
        super::fact_catalog::composed_empty(&mut self.inner.run, file, path, |rel, run| self.inner.exec.walk_empty(run, rel));
        let ft = file.table;
        let mut fwords = std::mem::take(&mut self.fwords);
        let mut completed = std::mem::take(&mut self.completed);
        if completed.file == Some(languages::FileId(file_no as u32)) {
            self.inner.exec.empty_drivers(&completed.empty_drivers);
            for &relation in &completed.empty {
                let name = ryo_relations::domain::relations::service::name(relation).0;
                self.inner.exec.walk_empty(&self.inner.run, name);
            }
        } else {
            completed.clear();
            languages::facts::clear_words(&mut fwords);
        }
        finish::own_words_cleared(&mut self.inner.run);
        let mut project = std::mem::take(&mut self.project);
        let mut type_paths = std::mem::take(&mut self.type_paths);
        let mut prefix_symbols = std::mem::take(&mut self.prefix_symbols);
        let mut direct_memo = std::mem::take(&mut self.direct_memo);
        let mut implicit = self.implicit.take();
        direct_memo.new_file();
        let mut pv = ColFacts { file, completed: &mut completed, words: &mut fwords, filled: std::mem::take(&mut self.filled), time: 0.0, rows: 0, by_rel: std::mem::take(&mut self.inner.by_rel), file_no: file_no as u32, kept: Default::default(), implicit: &mut implicit, type_paths: &mut type_paths, input_layout: self.input_layout, input_columns: Default::default(), prefix, prefix_symbols: &mut prefix_symbols, direct_memo: &mut direct_memo, project: &mut project, src: bytes };
        self.inner.file(file_no, path, ft, bytes, &mut pv, wanted);
        self.inner.provider_time += pv.time;
        self.inner.provider_rows += pv.rows;
        self.inner.by_rel = pv.by_rel;
        self.filled = pv.filled;
        self.fwords = fwords;
        completed.clear();
        self.completed = completed;
        self.type_paths = type_paths;
        self.prefix_symbols = prefix_symbols;
        self.direct_memo = direct_memo;
        self.implicit = implicit;
        self.project = project;
        self.inner.time += thread_cpu() - t0;
    }

    /// The second half on one file through the reader's facts (`vec finish` without `VEC_COLWALK`).
    pub(super) fn reader_file(&mut self, file_no: usize, path: &str, file: &mut languages::FileFacts<'_>, bytes: &[u8], wanted: &[usize]) {
        let t0 = thread_cpu();
        let ft = file.table;
        let mut fwords = std::mem::take(&mut self.fwords);
        languages::facts::clear_words(&mut fwords);
        finish::own_words_cleared(&mut self.inner.run);
        let mut project = std::mem::take(&mut self.project);
        let mut pv = super::Facts { facts: file, words: &mut fwords, filled: std::mem::take(&mut self.filled), time: 0.0, rows: 0, by_rel: std::mem::take(&mut self.inner.by_rel), file: file_no as u32, project: &mut project, src: bytes };
        self.inner.file(file_no, path, ft, bytes, &mut pv, wanted);
        self.inner.provider_time += pv.time;
        self.inner.provider_rows += pv.rows;
        self.inner.by_rel = pv.by_rel;
        self.filled = pv.filled;
        self.fwords = fwords;
        self.project = project;
        self.inner.time += thread_cpu() - t0;
    }

}
