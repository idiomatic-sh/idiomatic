//! Typed calls from the pinned check. A refusal returns to the host as an error.
use super::super::HostCell;
use super::{Active, Authority, Batch, Hosted, Input};
use crate::thread_cpu;
#[path = "pinned.rs"]
mod pinned;
#[path = "call_cache.rs"]
mod call_cache;
#[path = "allocated.rs"]
mod allocated;
use call_cache::CallCache;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use pinned::{Dictionary, Pinned};
use release_sdk::linter as sdk;
use ryo_values::domain::finish_plans::expressions::types::{
    ConstantEntry, ConstantEntryKey, PlanConstant, PlanNumber,
};
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use ryo_values::domain::row_texts::types::Cell;
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::Arc,
};

#[derive(Default)]
pub struct LiveReport {
    pub findings: Vec<(Box<str>, sdk::Slot, Vec<(Box<str>, sdk::Slot)>)>,
    pub unknowns: Vec<(Box<str>, String, String)>,
    pub unread: Vec<Box<str>>,
}

pub struct Live {
    batch: Batch,
    store:Option<super::stored::Store>,
    directory:Option<std::path::PathBuf>,
    state: Active<pinned::File, Dictionary>,
    languages: Vec<Pinned>,
    filters: HashMap<Vec<sdk::Glob>, (usize, Rc<GlobSet>)>,
    cache: CallCache,
    versions: HashMap<(Authority, BTreeMap<String, Arc<[u8]>>), Arc<str>>,
}

impl Drop for Live {
    fn drop(&mut self){
        if let Some(store)=&mut self.store && let Err(e)=store.publish(){eprintln!("publish completed session: {e}");}
    }
}
impl Default for Live {
    fn default() -> Self {
        MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sessions += 1;
        Self {
            batch: Batch::default(),
            store:None, directory:None,
            state: Active {
                keep: true,
                ..Default::default()
            },
            languages: Vec::new(),
            filters: HashMap::new(),
            cache: CallCache::default(),
            versions: HashMap::new(),
        }
    }
}

fn parameter(v: sdk::ParameterValue) -> Result<PlanConstant, String> {
    use sdk::ParameterValue::*;
    let number = |n| PlanConstant::Number(PlanNumber::Whole(WholeNumber(n)));
    let text = |s| PlanConstant::Text(LiteralText(s));
    let texts =
        |v: Vec<sdk::ParameterText>| PlanConstant::List(v.into_iter().map(|s| text(s.0)).collect());
    Ok(match v {
        Number(n) => number(n.0),
        Text(s) => text(s.0),
        Flag(f) => text((f == sdk::FlagState::On).to_string().into()),
        Numbers(ns) => PlanConstant::List(ns.0.into_iter().map(number).collect()),
        Texts(ss) => texts(ss),
        TextSets(sets) => PlanConstant::List(sets.into_iter().map(texts).collect()),
        Tables(rows) => PlanConstant::List(
            rows.into_iter()
                .map(|row| {
                    let mut fields: Vec<_> = row
                        .0
                        .into_iter()
                        .map(|c| ConstantEntry {
                            key: ConstantEntryKey(c.key.0),
                            value: text(c.value.0),
                        })
                        .collect();
                    fields.sort_by(|a, b| a.key.0.cmp(&b.key.0));
                    PlanConstant::Map(fields)
                })
                .collect(),
        ),
        _ => return Err("unsupported SDK parameter type".into()),
    })
}

fn slot(v: &Option<Cell>) -> sdk::Slot {
    let Some(c) = v else {
        return sdk::Slot::Nothing;
    };
    use release_sdk::languages::{Offset, Span};
    sdk::Slot::Is(match c {
        Cell::File(p) => sdk::Cell::File(sdk::RelativePath(p.clone())),
        Cell::Text(s) => sdk::Cell::Text(sdk::HostText(s.clone())),
        Cell::Number(n) => sdk::Cell::Number(sdk::HostNumber(n.0)),
        Cell::Span(s) => sdk::Cell::Span(Span {
            start: Offset(s.start.0),
            end: Offset(s.end.0),
        }),
    })
}

impl Live {
    pub fn use_cache(&mut self,directory:Option<&std::path::Path>)->Result<(),String>{
        if self.directory.as_deref()!=directory {
            if let Some(old)=&mut self.store {old.publish().map_err(|e|e.to_string())?;}
            self.store=directory.map(|p|super::stored::Store::directory(p.to_owned())).transpose().map_err(|e|e.to_string())?;
            self.directory=directory.map(std::path::Path::to_owned);
        }
        Ok(())
    }
    fn globs(&mut self, names: &[sdk::Glob]) -> Result<(usize, Rc<GlobSet>), String> {
        if let Some(found) = self.filters.get(names) {
            return Ok(found.clone());
        }
        let mut builder = GlobSetBuilder::new();
        for name in names {
            builder.add(
                GlobBuilder::new(name.as_ref())
                    .literal_separator(true)
                    .build()
                    .map_err(|e| e.to_string())?,
            );
        }
        let set = (self.filters.len(), Rc::new(builder.build().map_err(|e| e.to_string())?));
        self.filters.insert(names.to_vec(), set.clone());
        Ok(set)
    }

    pub fn lint(
        &mut self,
        req: sdk::LintRequest,
        linked: &sdk::Languages,
        memory: sdk::MemoryBudget,
    ) -> Result<LiveReport, String> {
        let t = thread_cpu();
        let limit = match memory {
            sdk::MemoryBudget::Machine => None,
            sdk::MemoryBudget::Bytes(n) => Some(n.get()),
            _ => return Err("unsupported memory budget".into()),
        };
        self.state.retention.configure(limit, allocated::bytes);
        let count = req.rules.len();
        let result = self.answer(req, linked);
        let mut last = MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let c = &self.batch.clocks;
        last.calls += 1;
        last.requests += count;
        last.refused += usize::from(result.is_err());
        if let Err(reason) = &result {
            *last.reasons.entry(reason.clone()).or_default() += 1;
        }
        last.elapsed += thread_cpu() - t;
        last.parts = [
            c.prepare,
            c.read_parse,
            c.screen,
            c.evaluate,
            c.project,
            c.assemble,
        ];
        last.retained = self.state.retention.counts();
        last.project_counts = [
            self.batch.projects.len(),
            self.batch.populations_built,
            self.batch.relations_built,
            self.batch.relations_rebuilt,
        ];
        last.counts = [
            self.batch.schedule.programs.len(),
            self.batch.bindings.len(),
            self.batch.schedule.sources.len(),
            c.reads,
            c.files,
            c.unique_pairs,
            c.skipped,
            c.evaluated,
        ];
        result
    }

    fn answer(
        &mut self,
        req: sdk::LintRequest,
        linked: &sdk::Languages,
    ) -> Result<LiveReport, String> {
        if self.languages.is_empty() {
            self.languages = linked.0.iter().cloned().map(Pinned::from).collect();
        }
        if req.source_language != sdk::SourceLanguage::Recognized
            || req.project_search_scope != sdk::ProjectSearchScope::Undeclared
            || req.given_project_inventory != Default::default()
        {
            return Err("unsupported source or project authority".into());
        }
        let physical: Arc<str> = Arc::from(req.root.to_str().ok_or("non-UTF8 root")?);
        let (authority, listed, given) = match req.corpus {
            sdk::Corpus::Given(files) => (Authority::Given, Vec::new(), files),
            sdk::Corpus::Listed(paths) => {
                (Authority::Disk(physical.to_string()), paths, Vec::new())
            }
            _ => return Err("a closed source inventory is required".into()),
        };
        let mut overlay = BTreeMap::new();
        for source in given.into_iter().chain(req.beside) {
            if source.executable != sdk::ExecutablePermission::Unknown {
                return Err("unhandled supplied executable permission".into());
            }
            overlay.insert(
                source.path.0.into_string(),
                Arc::<[u8]>::from(source.text.0.as_bytes()),
            );
        }
        let (inventory, paths) = self.cache.inventory(listed.iter().map(|p| p.0.as_ref())
            .chain(overlay.keys().map(String::as_str)).collect());
        let key = (authority.clone(), overlay);
        let root = if let Some(id) = self.versions.get(&key) {
            id.clone()
        } else {
            let id: Arc<str> = Arc::from(format!("check-input:{}", self.versions.len()));
            for (p, v) in &key.1 {
                self.batch.schedule.provide(&id, p.clone(), v.clone())?;
            }
            self.versions.insert(key.clone(), id.clone());
            id
        };
        for p in listed {
            if !key.1.contains_key(p.0.as_ref()) {
                self.batch.schedule.listed(&root, &physical, &p.0)?;
            }
        }
        let filter = self.globs(&req.files)?;
        let hosted = req
            .hosted
            .into_iter()
            .map(|t| self.cache.table(t))
            .collect::<Result<Vec<_>, _>>()?;
        let mut ids = Vec::new();
        self.batch.work.clear();
        for (id, rule) in req.rules {
            if rule.applicability != sdk::RuleApplicability::Always || !rule.observations.is_empty()
            {
                return Err(format!(
                    "undeclared observation or syntax requirement for {id}"
                ));
            }
            let inc = self.globs(&rule.include)?;
            let exc = self.globs(&rule.exclude)?;
            let files = self.cache.files((inventory, filter.0, inc.0, exc.0), |p| {
                    (req.files.is_empty() || filter.1.is_match(p))
                        && (rule.include.is_empty() || inc.1.is_match(p))
                        && !exc.1.is_match(p)
            });
            let parameters = rule
                .parameters
                .into_iter()
                .map(|(k, v)| Ok((k.0, parameter(v)?)))
                .collect::<Result<_, String>>()?;
            let work = self
                .batch
                .register_query(
                    self.cache.query(rule.query.as_ref()),
                    Input {
                        root: root.to_string(),
                        files,
                        inventory: Some(paths.clone()),
                        repository: Some(authority.clone()),
                        hosted: hosted.clone(),
                        parameters,
                    },
                    None,
                )
                .map_err(|e| format!("{id}: {e}"))?;
            self.batch.work.push(work);
            ids.push(id);
        }
        if let Some(store)=&mut self.store {
            self.batch.epoch_with_store(&self.languages,&mut self.state,Some(store));
        } else {
            self.batch.epoch(&self.languages,&mut self.state);
        }
        let mut out = LiveReport::default();
        let start = thread_cpu();
        for (id, work) in ids.into_iter().zip(&self.batch.work) {
            let receipt =
                Batch::answer(work, &self.state.answers).map_err(|e| format!("{id}: {e}"))?;
            for row in receipt.rows {
                let subject = row
                    .identity
                    .first()
                    .and_then(|i| row.columns.get(*i))
                    .ok_or("missing row subject")?;
                out.findings.push((
                    id.as_ref().into(),
                    slot(&subject.1),
                    row.columns
                        .iter()
                        .map(|(n, v)| (n.clone(), slot(v)))
                        .collect(),
                ));
            }
            for (p, msg) in receipt.unknowns {
                out.unknowns
                    .push((id.as_ref().into(), p.clone(), msg.clone()));
            }
        }
        for path in Rc::unwrap_or_clone(paths) {
            let p = sdk::RelativePath(path.into());
            if linked.reads(&p) != release_sdk::languages::Recognition::Reads
                && !linked.0.iter().any(|reader| reader.primary_role(release_sdk::languages::SourcePath(&p.0)) == release_language::domain::files::types::PrimaryFileRole::Configuration)
            {
                out.unread.push(p.0);
            }
        }
        out.unread
            .extend(self.state.manifest_failures.iter().map(|p| p.0.clone()));
        out.unread.sort_unstable();
        self.batch.clocks.assemble += thread_cpu() - start;
        Ok(out)
    }
}

/// Replay a closed capture with exactly the parsers linked into this release.
pub fn capture(linked: sdk::Languages, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>) {
    capture_then(linked, manifest, output, workers, || {});
}

pub fn capture_then(linked: sdk::Languages, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>, completed: impl FnOnce()) {
    let readers: Vec<_> = linked.0.into_iter().map(Pinned::from).collect();
    super::batch_mode_with_workers(&readers, manifest, output, None, workers, completed);
}

pub fn capture_retained(linked:sdk::Languages,manifest:&str,output:&str,root:&str){
    let readers:Vec<_>=linked.0.into_iter().map(Pinned::from).collect();
    super::batch_mode(&readers,manifest,output,Some(std::path::Path::new(root)));
}

#[derive(Default)]
struct Measurement {
    sessions: usize,
    project_counts: [usize; 4],
    calls: usize,
    requests: usize,
    refused: usize,
    reasons: BTreeMap<String, usize>,
    elapsed: f64,
    parts: [f64; 6],
    counts: [usize; 8],
    retained: [u64; 4],
}
static MEASURED: std::sync::Mutex<Measurement> = std::sync::Mutex::new(Measurement {
    sessions: 0,
    project_counts: [0; 4],
    calls: 0,
    requests: 0,
    refused: 0,
    reasons: BTreeMap::new(),
    elapsed: 0.0,
    parts: [0.0; 6],
    counts: [0; 8],
    retained: [0; 4],
});

/// Print the one executor's cumulative counters to the benchmark log after the host renders.
pub fn print_measurement() {
    let m = MEASURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (reason, count) in &m.reasons {
        eprintln!("UNANSWERED\t{count}\t{reason}");
    }
    let [versions,hits,evictions,peak_charge]=m.retained;
    eprintln!("FILE_CACHE versions={versions} hits={hits} evictions={evictions} peak_charge={peak_charge}");
    let [prepare, read, first, second, project, assembly] = m.parts;
    let [plans, bindings, sources, reads, parses, pairs, skips, eval] = m.counts;
    let [inventories, builds, relations, repeated] = m.project_counts;
    eprintln!(
        "PROJECT_POPULATIONS sessions={} inventories={inventories} builds={builds} relations={relations} repeated_relations={repeated}",
        m.sessions,
    );
    eprintln!(
        "CHECK_SESSION live calls={} requests={} failed_calls={} programs={plans} bindings={bindings} sources={sources} reads={reads} parses={parses} distinct_pairs={pairs} skipped={skips} evaluated={eval} prep={prepare:.6} read_parse={read:.6} first={first:.6} second={second:.6} project={project:.6} assembly={assembly:.6} admission={:.6}",
        m.calls,
        m.requests,
        m.refused,
        m.elapsed - m.parts.iter().sum::<f64>()
    );
}

/// Write one fixture's complete rows, through the same reader and executor as the check.
pub fn capture_one(
    linked: sdk::Languages,
    pack: &str,
    input: &str,
    out: &str,
) -> Result<(), String> {
    let readers: Vec<_> = linked.0.into_iter().map(Pinned::from).collect();
    let bytes = std::fs::read(input).map_err(|e| e.to_string())?;
    let req = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    super::one_source(&readers, pack, req, out)
}
