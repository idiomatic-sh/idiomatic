//! Python file visit and reusable provider state feeding the shared executor.
use super::{ColFacts, HashMap, Vocab, files, finish, finisher, flag, thread_cpu};
use ryo::languages;

pub(super) struct Finisher<'p> {
    inner: finisher::Finisher<'p>,
    fwords: languages::FileWords,
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
        paths:Option<&[String]>,
    ) -> Self {
        let t0 = thread_cpu();
        let list = paths.map_or_else(||files(args[1]),|p|p.to_vec());
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
            .collect::<Vec<_>>().into();
        // redo-general: the provider's token order (its lexical model, `provider_tokens.rs`)
        run.set_tokens(
            std::rc::Rc::new(super::provider_tokens::Lexicon::new(&|n| {
                vocab.kinds.get(n).copied()
            })),
            vocab.kind_names.len(),
        );
        let mut me = Self {
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
    /// Use the same project answer that is indexed by file for the executor.
    pub(super) fn identity(
        &mut self,
        file: usize,
        words: &mut languages::FileWords,
        out: &mut languages::FilledColumns,
    ) -> bool {
        use super::finish::project::{ProjectAsk, for_file};
        let result = for_file(
            file as u32,
            &mut self.inner.run,
            super::provider_packaging::MODULE_IDENTITY_REL,
            ProjectAsk::Run,
            |run, files| self.project.module_identity(run, files),
        );
        let Ok(rows) = result else { return false };
        out.0.clear();
        for (kind, cells) in rows.tys.iter().zip(&rows.cols) {
            let mut col = Vec::with_capacity(cells.len());
            for &cell in cells {
                let value = if matches!(kind, finish::Ty::Sym) && cell != finish::NULL {
                    let Some(text) = self
                        .inner
                        .run
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
                } else {
                    languages::CellWord(cell)
                };
                col.push(value);
            }
            out.0.push(languages::FilledColumn(col));
        }
        true
    }
    pub(super) fn end(self, _walk_time:f64)->bool {
        self.inner.end(finish::json_output::Projection::DistinctLocations)
    }
    /// The second half on one walked file of the direct-columns walk (its own model by parts, built on demand).
    pub(super) fn column_file(
        &mut self,
        file_no: usize,
        file: &mut ryo_python::fixtures::columns::ColumnFile<'_, '_>,
        bytes: &[u8],
        wanted: &[usize],
    ) {
        let t0 = thread_cpu();
        file.set_file(languages::FileId(file_no as u32));
        let ft = file.table;
        let mut fwords = std::mem::take(&mut self.fwords);
        languages::facts::clear_words(&mut fwords);
        finish::own_words_cleared(&mut self.run);
        let mut project = std::mem::take(&mut self.project);
        let mut pv = ColFacts {
            import_comments: Default::default(),
            file,
            words: &mut fwords,
            filled: languages::FilledColumns::default(),
            time: 0.0,
            rows: 0,
            by_rel: HashMap::new(),
            file_no: file_no as u32,
            kept: HashMap::new(),
            project: &mut project,
            src: bytes,
        };
        self.file(file_no, ft, bytes, &mut pv, wanted);
        self.provider_time += pv.time;
        self.provider_rows += pv.rows;
        for (k, v) in pv.by_rel {
            let e = self.by_rel.entry(k).or_default();
            e.0 += v.0;
            e.1 += v.1;
            e.2 += v.2;
        }
        self.fwords = fwords;
        self.project = project;
        self.time += thread_cpu() - t0;
    }
}
