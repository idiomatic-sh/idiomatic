//! Python file visit and reusable provider state feeding the shared executor.
use super::{ColFacts, Vocab, files, finish, finisher, flag, thread_cpu};
use ryo::languages;

pub(super) struct Finisher<'p> {
    inner: finisher::Finisher<'p>,
    fwords: languages::FileWords,
    completed: super::completed_facts::Completed,
    type_paths: super::type_expression::Paths,
    input_layout: super::composition_inputs::Layout,
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
        request: Option<(&str, &super::json_input::Metadata)>,
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
            implicit: None,
            filled: Default::default(),
            direct_memo: Default::default(),
            inner: finisher::Finisher::new(plan, run, args, request.and_then(|(path, doc)| doc.accepted.as_ref().map(|a| (path, a)))),
            fwords: ryo_python::fixtures::walking::walk_words(walk)
                .expect("the walk's fixed names"),
            project: super::provider_packaging::Project::new(
                args[2].trim_end_matches('/'),
                match request {
                    Some((_, doc)) => doc.search_roots.clone(),
                    None => flag(args, "--request").and_then(|path| super::json_input::metadata(&path).search_roots),
                },
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

    /// Use the same project answer that is indexed by file for the executor.
    pub(super) fn identity(
        &mut self,
        file: usize,
        words: &mut languages::FileWords,
        out: &mut languages::FilledColumns,
    ) -> bool {
        self.project.identity_columns(&self.inner.run.paths[file], file as u32, words, out)
    }

    fn walk_cells(
        file: languages::FileId,
        run: &finish::Run,
        rows: &finish::Table,
        words: &mut languages::FileWords,
        out: &mut languages::FilledColumns,
    ) -> bool {
        out.0.clear();
        for (kind, cells) in rows.tys.iter().zip(&rows.cols) {
            let mut col = Vec::with_capacity(cells.len());
            for &cell in cells {
                let value = if matches!(kind, finish::Ty::Sym) && cell != finish::NULL {
                    let Some(text) = run
                        .texts
                        .get(cell as usize)
                        .and_then(|v| std::str::from_utf8(v).ok())
                    else {
                        return false;
                    };
                    languages::facts::intern(
                        words,
                        ryo_relations::domain::terms::types::NameText(text),
                    )
                } else if matches!(kind, finish::Ty::Node) && cell != finish::NULL {
                    let at = languages::Position(u32::try_from(cell).expect("local node position"));
                    languages::CellWord(ryo_relations::domain::relations::service::node_id(file, at).0)
                } else {
                    languages::CellWord(cell)
                };
                col.push(value);
            }
            out.0.push(languages::FilledColumn(col));
        }
        true
    }
    /// Lend the complete provider answer to the walk. Keep the answer and its inputs for execution on this file.
    pub(super) fn walk_table(
        &mut self,
        number: usize,
        file: &mut ryo_python::fixtures::columns::ColumnFile<'_, '_>,
        bytes: &[u8],
        name: &str,
        words: &mut languages::FileWords,
        out: &mut languages::FilledColumns,
        kept: &mut super::composition_inputs::Tables,
    ) -> bool {
        let id = languages::FileId(number as u32);
        if let Some(rows) = kept.get(name) {
            return Self::walk_cells(id, &self.inner.run, rows, words, out);
        }
        finish::own_words_cleared(&mut self.inner.run);
        let mut completed = Default::default();
        self.with_provider(number, file, bytes, words, &mut completed, kept, |inner, provider| {
            let rows = if name == super::expression_parts::NESTED_DELIMITERS {
                super::expression_parts::fill(&mut inner.run, provider.file, provider.words, bytes, id.0, name)
                    .map(std::rc::Rc::new)
            } else {
                provider.fact(&mut inner.run, name)
            };
            let Ok(rows) = rows else { return false };
            let answered = Self::walk_cells(id, &inner.run, &rows, provider.words, out);
            provider.kept.insert(name, rows);
            answered
        })
    }
    pub(super) fn end(self) {
        self.inner.end(finish::json_output::Projection::DistinctLocations);
    }
    /// Swap the dictionary together with its rows. Each of the two borrowed
    /// parse loans owns both until its executor visit has finished.
    pub(super) fn swap_loan(&mut self, words: &mut languages::FileWords, completed: &mut super::completed_facts::Completed) {
        std::mem::swap(&mut self.fwords, words);
        std::mem::swap(&mut self.completed, completed);
    }

    /// The second half on one walked file of the direct-columns walk (its own model by parts, built on demand).
    pub(super) fn column_file(&mut self, file_no: usize, path: &str, file: &mut ryo_python::fixtures::columns::ColumnFile<'_, '_>, bytes: &[u8], wanted: &[usize], kept: &mut super::composition_inputs::Tables) {
        let t0 = thread_cpu();
        file.set_file(languages::FileId(file_no as u32));
        if !wanted.is_empty() && file.plain_spellings() { self.inner.exec.source_spellings(bytes); }
        for &rel in &file.empty { self.inner.exec.walk_empty(&self.inner.run, rel); }
        super::fact_catalog::composed_empty(&mut self.inner.run, file, path, |rel, run| self.inner.exec.walk_empty(run, rel));
        let ft = file.table;
        let mut fwords = std::mem::take(&mut self.fwords);
        let mut completed = std::mem::take(&mut self.completed);
        if completed.file == Some(languages::FileId(file_no as u32)) {
            self.inner.exec.absent_symbols(&completed.absent_symbols);
            std::mem::swap(self.inner.exec.prefix_decisions(file_no as u32), &mut completed.decisions);
            for &relation in &completed.empty {
                let name = ryo_relations::domain::relations::service::name(relation).0;
                self.inner.exec.walk_empty(&self.inner.run, name);
            }
        } else {
            completed.clear();
            languages::facts::clear_words(&mut fwords);
        }
        finish::own_words_cleared(&mut self.inner.run);
        self.with_provider(file_no, file, bytes, &mut fwords, &mut completed, kept,
            |inner, provider| inner.file(file_no, ft, bytes, provider, wanted));
        self.fwords = fwords;
        completed.clear();
        self.completed = completed;
        self.inner.time += thread_cpu() - t0;
    }

    fn with_provider<T>(
        &mut self, file_no: usize, file: &mut ryo_python::fixtures::columns::ColumnFile<'_, '_>,
        bytes: &[u8], words: &mut languages::FileWords, completed: &mut super::completed_facts::Completed,
        kept: &mut super::composition_inputs::Tables,
        visit: impl FnOnce(&mut finisher::Finisher<'p>, &mut ColFacts<'_, '_, '_, '_>) -> T,
    ) -> T {
        self.direct_memo.new_file();
        let mut provider = ColFacts {
            file, completed, words, src: bytes, file_no: file_no as u32,
            project: &mut self.project, implicit: &mut self.implicit, type_paths: &mut self.type_paths,
            input_layout: self.input_layout, input_columns: Default::default(),
            direct_memo: &mut self.direct_memo,
            import_comments: Default::default(), kept: std::mem::take(kept),
            filled: std::mem::take(&mut self.filled), by_rel: std::mem::take(&mut self.inner.by_rel),
            time: 0., rows: 0,
        };
        let result = visit(&mut self.inner, &mut provider);
        self.inner.provider_time += provider.time;
        self.inner.provider_rows += provider.rows;
        self.inner.by_rel = provider.by_rel;
        self.filled = provider.filled;
        *kept = provider.kept;
        result
    }

}
