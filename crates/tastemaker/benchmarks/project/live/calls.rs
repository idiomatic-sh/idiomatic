//! Typed calls from the pinned check. A refusal returns to the host as an error.
use super::super::HostCell;
use super::{Active, Authority, Batch, Hosted, Input};
use crate::thread_cpu;
#[path = "pinned.rs"]
mod pinned;
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
};

#[derive(Default)]
pub struct LiveReport {
    pub findings: Vec<(Box<str>, sdk::Slot, Vec<(Box<str>, sdk::Slot)>)>,
    pub unknowns: Vec<(Box<str>, String, String)>,
    pub unread: Vec<Box<str>>,
}

pub struct Live {
    batch: Batch,
    state: Active<release_sdk::languages::Elements, Dictionary>,
    languages: Vec<Pinned>,
    filters: HashMap<Vec<sdk::Glob>, Rc<GlobSet>>,
    versions: HashMap<(Authority, BTreeMap<String, Rc<[u8]>>), Rc<str>>,
}

impl Default for Live {
    fn default() -> Self {
        MEASURED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .sessions += 1;
        Self {
            batch: Batch::default(),
            state: Active {
                keep: true,
                ..Default::default()
            },
            languages: Vec::new(),
            filters: HashMap::new(),
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

fn table(t: sdk::HostRelation) -> Result<Hosted, String> {
    let columns = t
        .columns
        .into_iter()
        .map(|c| {
            let kind = match c.kind {
                sdk::HostKind::File => "File",
                sdk::HostKind::Text => "Text",
                sdk::HostKind::Number => "Number",
                _ => return Err("unsupported SDK host column type".to_owned()),
            };
            Ok((c.name.0.into(), kind.to_owned()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let rows = t
        .rows
        .into_iter()
        .map(|r| {
            if r.0.len() != columns.len() {
                return Err("host row width disagrees with declaration".into());
            }
            r.0.into_iter()
                .zip(&columns)
                .map(|(v, (_, kind))| {
                    Ok(match (kind.as_str(), v) {
                        ("File", sdk::Cell::File(p)) => HostCell::File { file: p.0.into() },
                        ("Text", sdk::Cell::Text(s)) => HostCell::Text(s.0.into()),
                        ("Number", sdk::Cell::Number(n)) => HostCell::Number(n.0),
                        _ => return Err("host cell disagrees with declaration".to_owned()),
                    })
                })
                .collect()
        })
        .collect::<Result<_, String>>()?;
    Ok(Hosted {
        name: t.name.0.into(),
        columns,
        rows,
        partial: None,
    })
}

fn slot(v: &Option<Cell<Rc<str>>>) -> sdk::Slot {
    let Some(c) = v else {
        return sdk::Slot::Nothing;
    };
    use release_sdk::languages::{Offset, Span};
    sdk::Slot::Is(match c {
        Cell::File(p) => sdk::Cell::File(sdk::RelativePath(p.as_ref().into())),
        Cell::Text(s) => sdk::Cell::Text(sdk::HostText(s.as_ref().into())),
        Cell::Number(n) => sdk::Cell::Number(sdk::HostNumber(n.0)),
        Cell::Span(s) => sdk::Cell::Span(Span {
            start: Offset(s.start.0),
            end: Offset(s.end.0),
        }),
    })
}

impl Live {
    fn globs(&mut self, names: &[sdk::Glob]) -> Result<Rc<GlobSet>, String> {
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
        let set = Rc::new(builder.build().map_err(|e| e.to_string())?);
        self.filters.insert(names.to_vec(), set.clone());
        Ok(set)
    }

    pub fn lint(
        &mut self,
        req: sdk::LintRequest,
        linked: &sdk::Languages,
    ) -> Result<LiveReport, String> {
        let t = thread_cpu();
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
        let physical: Rc<str> = Rc::from(req.root.to_str().ok_or("non-UTF8 root")?);
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
                Rc::<[u8]>::from(source.text.0.as_bytes()),
            );
        }
        let mut paths: Vec<String> = listed
            .iter()
            .map(|p| p.0.to_string())
            .chain(overlay.keys().cloned())
            .collect();
        paths.sort_unstable();
        paths.dedup();
        let key = (authority.clone(), overlay);
        let root = if let Some(id) = self.versions.get(&key) {
            id.clone()
        } else {
            let id: Rc<str> = Rc::from(format!("check-input:{}", self.versions.len()));
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
            .map(table)
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
            let files = paths
                .iter()
                .filter(|p| {
                    (req.files.is_empty() || filter.is_match(p))
                        && (rule.include.is_empty() || inc.is_match(p))
                        && !exc.is_match(p)
                })
                .cloned()
                .collect();
            let parameters = rule
                .parameters
                .into_iter()
                .map(|(k, v)| Ok((k.0, parameter(v)?)))
                .collect::<Result<_, String>>()?;
            let work = self
                .batch
                .register_query(
                    Rc::from(rule.query.as_ref()),
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
        self.batch.epoch(&self.languages, &mut self.state);
        let mut out = LiveReport::default();
        let start = thread_cpu();
        for (id, work) in ids.into_iter().zip(&self.batch.work) {
            let receipt =
                Batch::answer(work, &self.state.answers).map_err(|e| format!("{id}: {e}"))?;
            for row in receipt.rows {
                let subject = row.layout.identity.first()
                    .map(|&i| row.value(i))
                    .ok_or("missing row subject")?;
                out.findings.push((
                    id.as_ref().into(),
                    slot(&subject),
                    row.layout.names.iter().zip(row.cells())
                        .map(|(n, v)| (n.as_str().into(), slot(&v)))
                        .collect(),
                ));
            }
            for (p, msg) in receipt.unknowns {
                out.unknowns
                    .push((id.as_ref().into(), p.clone(), msg.clone()));
            }
        }
        for path in paths {
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
pub fn capture(linked: sdk::Languages, manifest: &str, output: &str) {
    let readers: Vec<_> = linked.0.into_iter().map(Pinned::from).collect();
    super::batch_with_sources(&readers, manifest, output);
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
});

/// Print the one executor's cumulative counters to the benchmark log after the host renders.
pub fn print_measurement() {
    let m = MEASURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (reason, count) in &m.reasons {
        eprintln!("UNANSWERED\t{count}\t{reason}");
    }
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
    let req = crate::json_input::decode(&bytes).map_err(|e| e.to_string())?;
    super::one_source(&readers, pack, req, out)
}
