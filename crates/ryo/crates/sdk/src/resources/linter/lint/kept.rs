//! What `Cache::At` keeps: the findings of each file, and nothing else. A run over a listed corpus finds, for every file, the findings
//! its last run recorded under the same SCOPE (the rules, the request's other fields, the languages' build and the shape of the
//! project tree) and the same bytes and mode; it replays those and hands the executor only the files it has no record of. A run in
//! which every file has its record executes nothing. Before `lint` returns, the records of the files it answered are on disk, so a
//! host may end its process without running destructors.
//!
//! The record of a file is small: its findings, what its rules could not decide, why no language read it, and the rules skipped on
//! it. The session the executor builds (its plans, the columns of its cross-file operators) is not kept here: the whole-session
//! store is the `bench-hooks` cache `Cache::Session`.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::errors::LintError;
use super::types::{Corpus, Finding, FindingColumn, FindingKey, LintReport, LintRequest, LinterLintEngine, Message, RuleId, SkipReason, Skipped};
use crate::domain::cell::types::{Cell, HostNumber, HostText, Slot};
use crate::domain::file::types::{ReadFailure, RelativePath, Unreadable};
use crate::domain::relation::types::ColumnName;
use crate::domain::place::types::{Column, Line, Place};
use crate::domain::project::types::{DependencyBytes, DependencyFingerprint, ProjectDependency, ProjectDependencyState};
use ryo_relations::domain::relations::types::{Offset, Span};

/// The name of the file in the cache directory.
const FILE: &str = "findings.bin";
/// What a file of this form starts with: change it and every store written before is ignored.
const MAGIC: &[u8; 8] = b"ryo-kept";
/// How many scopes a store keeps: the least recent are dropped.
const SCOPES: usize = 8;

/// How many files the executor has been handed through the kept cache at each directory in this process: with every key present, none.
#[cfg(feature = "bench-hooks")]
static EVALUATED: std::sync::Mutex<BTreeMap<PathBuf, usize>> = std::sync::Mutex::new(BTreeMap::new());

/// The count of files the executor has been handed through the kept cache at `directory` in this process.
#[cfg(feature = "bench-hooks")]
#[must_use]
pub fn files_evaluated(directory: &Path) -> usize {
    EVALUATED.lock().map_or(0, |counts| counts.get(directory).copied().unwrap_or(0))
}

type Digest = [u8; 32];
type Key = [u8; 16];

/// What a file's file-local rules answered, as the report holds it.
#[derive(Clone, Default)]
struct Answer {
    findings: Vec<Finding>,
    unknowns: Vec<(RuleId, Message)>,
    unread: Option<Unreadable>,
    skipped: Vec<(RuleId, SkipReason)>,
}

/// The records of one scope.
#[derive(Clone, Default)]
struct Scope {
    /// The rules whose programs are not file-local: never replayed, evaluated on every run.
    not_file_local: Vec<RuleId>,
    dependencies: Vec<ProjectDependency>,
    files: BTreeMap<Key, Answer>,
}

/// The store: the scopes it holds, the most recent last.
#[derive(Default)]
struct Store {
    scopes: Vec<(Key, Scope)>,
}

/// What a request needs of the cache before it runs.
struct Plan {
    scope: Key,
    /// The corpus, in the order the request lists it.
    names: Vec<RelativePath>,
    /// The key of each file, none for a file that cannot be read now.
    keys: Vec<Option<Key>>,
    /// The record of each file, when the store has one.
    hits: Vec<Option<Answer>>,
    /// The scope as the store held it, when it did.
    held: Option<Scope>,
}

/// How the executor is handed a request.
enum Job {
    /// Every rule over every file: the store knows nothing of the request, or cannot serve it.
    Whole(usize),
    /// The file-local rules over the files with no record.
    Files(usize),
    /// The rules that are not file-local over every file.
    Rest(usize),
}

/// Answers `requests` as `run` would, replaying from the store at `directory` the findings of the file-local rules for the files it
/// has records of and running the executor over the others; the rules that are not file-local are run over every file every time.
/// Then publishes the records. A request the store cannot serve (a corpus that is not listed, files given as text) goes whole.
pub(super) fn execute(
    directory: &Path,
    engine: &LinterLintEngine,
    requests: Vec<LintRequest>,
    run: &mut dyn FnMut(Vec<LintRequest>) -> Vec<Result<LintReport, LintError>>,
) -> Vec<Result<LintReport, LintError>> {
    let build = build_identity();
    let mut store = load(directory, &build).unwrap_or_default();
    let plans: Vec<Option<Plan>> = requests.iter().map(|request| plan(request, engine, directory, &build, &store)).collect();
    let mut handed = Vec::new();
    let mut jobs = Vec::new();
    for (index, (request, plan)) in requests.iter().zip(&plans).enumerate() {
        let Some(Plan { held: Some(held), names, hits, .. }) = plan else {
            handed.push(request.clone());
            jobs.push(Job::Whole(index));
            continue;
        };
        let (rest, local): (BTreeMap<_, _>, BTreeMap<_, _>) =
            request.rules.clone().into_iter().partition(|(id, _)| held.not_file_local.contains(id));
        let missing: Vec<RelativePath> = names.iter().zip(hits).filter(|(_, hit)| hit.is_none()).map(|(name, _)| name.clone()).collect();
        if !local.is_empty() && !missing.is_empty() {
            let mut narrowed = request.clone();
            narrowed.rules = local;
            narrowed.corpus = Corpus::Listed(missing);
            handed.push(narrowed);
            jobs.push(Job::Files(index));
        }
        if !rest.is_empty() {
            let mut whole = request.clone();
            whole.rules = rest;
            handed.push(whole);
            jobs.push(Job::Rest(index));
        }
    }
    #[cfg(feature = "bench-hooks")]
    for job in &handed {
        if let Corpus::Listed(files) = &job.corpus {
            if let Ok(mut counts) = EVALUATED.lock() {
                *counts.entry(directory.to_path_buf()).or_default() += files.len();
            }
        }
    }
    let ran = if handed.is_empty() { Vec::new() } else { run(handed) };
    let not_file_local = super::session::take_not_file_local();
    let mut whole: Vec<Option<Result<LintReport, LintError>>> = (0..requests.len()).map(|_| None).collect();
    let mut files: Vec<Option<Result<LintReport, LintError>>> = (0..requests.len()).map(|_| None).collect();
    let mut rest: Vec<Option<Result<LintReport, LintError>>> = (0..requests.len()).map(|_| None).collect();
    for (job, answer) in jobs.iter().zip(ran) {
        match job {
            Job::Whole(index) => whole[*index] = Some(answer),
            Job::Files(index) => files[*index] = Some(answer),
            Job::Rest(index) => rest[*index] = Some(answer),
        }
    }
    let mut published = false;
    let mut answers = Vec::with_capacity(requests.len());
    for (index, plan) in plans.into_iter().enumerate() {
        let Some(plan) = plan else {
            answers.push(whole[index].take().unwrap_or_else(|| Err(LintError::Execution { reason: "the executor answered fewer requests than it was given".into() })));
            continue;
        };
        let request = &requests[index];
        let (report, scope) = match (plan.held.clone(), whole[index].take(), files[index].take(), rest[index].take()) {
            (None, Some(Ok(report)), _, _) => {
                let rules: Vec<RuleId> = request.rules.keys().filter(|id| not_file_local.contains(*id)).cloned().collect();
                remember(&plan, report, rules)
            }
            (Some(held), _, files, rest) => {
                let mut failure = None;
                let files = files.map(|answer| answer.inspect_err(|error| failure = Some(error.clone())).ok());
                let rest = rest.map(|answer| answer.inspect_err(|error| failure = Some(error.clone())).ok());
                if let Some(error) = failure {
                    answers.push(Err(error));
                    continue;
                }
                replay(&plan, held, files.flatten(), rest.flatten())
            }
            (None, whole, _, _) => {
                answers.push(whole.unwrap_or_else(|| Err(LintError::Execution { reason: "the executor answered fewer requests than it was given".into() })));
                continue;
            }
        };
        published |= scope.is_some();
        if let Some(scope) = scope {
            store.scopes.retain(|(key, _)| *key != plan.scope);
            store.scopes.push((plan.scope, scope));
        }
        answers.push(Ok(report));
    }
    if published {
        let excess = store.scopes.len().saturating_sub(SCOPES);
        store.scopes.drain(..excess);
        if let Err(error) = publish(directory, &build, &store) {
            eprintln!("the kept findings were not written: {error}");
        }
    }
    answers
}

/// The answers of a request the executor ran whole: the report, and the scope to record, whose files hold the answers of the
/// file-local rules.
fn remember(plan: &Plan, report: LintReport, not_file_local: Vec<RuleId>) -> (LintReport, Option<Scope>) {
    let index: BTreeMap<&str, usize> = plan.names.iter().enumerate().map(|(at, name)| (&*name.0, at)).collect();
    let mut answers: Vec<Answer> = plan.names.iter().map(|_| Answer::default()).collect();
    let mut belongs = true;
    let local = |rule: &RuleId| !not_file_local.contains(rule);
    for finding in &report.findings {
        match file_of(&finding.subject).and_then(|path| index.get(path)) {
            Some(at) => {
                if local(&finding.rule) {
                    answers[*at].findings.push(finding.clone());
                }
            }
            None => belongs = false,
        }
    }
    for unknown in &report.unknowns {
        match index.get(&*unknown.path.0) {
            Some(at) => {
                if local(&unknown.rule) {
                    answers[*at].unknowns.push((unknown.rule.clone(), unknown.message.clone()));
                }
            }
            None => belongs = false,
        }
    }
    for unread in &report.unread {
        match index.get(&*unread.path.0) {
            Some(at) => answers[*at].unread = Some(unread.reason.clone()),
            None => belongs = false,
        }
    }
    for skipped in &report.skipped {
        match index.get(&*skipped.path.0) {
            Some(at) => {
                if local(&skipped.rule) {
                    answers[*at].skipped.push((skipped.rule.clone(), skipped.reason));
                }
            }
            None => belongs = false,
        }
    }
    let mut scope = Scope { not_file_local, dependencies: report.dependencies.clone(), files: BTreeMap::new() };
    for (at, answer) in answers.into_iter().enumerate() {
        if let Some(key) = plan.keys[at] {
            scope.files.insert(key, answer);
        }
    }
    (report, belongs.then_some(scope))
}

/// The report of a request the store knew: the records of the files it had, the answers of the executor over the others for the
/// file-local rules (`files`), and the answers over every file for the rest (`rest`); with the scope to record.
fn replay(plan: &Plan, held: Scope, files: Option<LintReport>, rest: Option<LintReport>) -> (LintReport, Option<Scope>) {
    let index: BTreeMap<&str, usize> = plan.names.iter().enumerate().map(|(at, name)| (&*name.0, at)).collect();
    let mut answers: Vec<Answer> = plan.hits.iter().map(|hit| hit.clone().unwrap_or_default()).collect();
    let mut belongs = true;
    let mut dependencies = held.dependencies.clone();
    let mut extra = super::executor::report(Vec::new(), Vec::new(), Vec::new());
    if let Some(fresh) = files {
        for dependency in fresh.dependencies {
            if !dependencies.iter().any(|known| known.path == dependency.path) {
                dependencies.push(dependency);
            }
        }
        for finding in fresh.findings {
            match file_of(&finding.subject).and_then(|path| index.get(path)).filter(|at| plan.hits[**at].is_none()) {
                Some(at) => answers[*at].findings.push(finding),
                None => {
                    belongs = false;
                    extra.findings.push(finding);
                }
            }
        }
        for unknown in fresh.unknowns {
            match index.get(&*unknown.path.0).filter(|at| plan.hits[**at].is_none()) {
                Some(at) => answers[*at].unknowns.push((unknown.rule, unknown.message)),
                None => belongs = false,
            }
        }
        for unread in fresh.unread {
            match index.get(&*unread.path.0).filter(|at| plan.hits[**at].is_none()) {
                Some(at) => answers[*at].unread = Some(unread.reason),
                None => belongs = false,
            }
        }
        for skipped in fresh.skipped {
            match index.get(&*skipped.path.0).filter(|at| plan.hits[**at].is_none()) {
                Some(at) => answers[*at].skipped.push((skipped.rule, skipped.reason)),
                None => belongs = false,
            }
        }
    }
    let mut report = super::executor::report(Vec::new(), Vec::new(), Vec::new());
    let mut unread: BTreeMap<RelativePath, Unreadable> = BTreeMap::new();
    let mut scope = Scope { not_file_local: held.not_file_local.clone(), dependencies: Vec::new(), files: BTreeMap::new() };
    for (at, answer) in answers.iter().enumerate() {
        let name = &plan.names[at];
        report.findings.extend(answer.findings.iter().cloned());
        for (rule, message) in &answer.unknowns {
            report.unknowns.push(super::executor::unknown(rule.clone(), name.clone(), message.clone()));
        }
        if let Some(reason) = &answer.unread {
            let _ = unread.insert(name.clone(), reason.clone());
        }
        for (rule, reason) in &answer.skipped {
            report.skipped.push(Skipped { rule: rule.clone(), path: name.clone(), reason: *reason });
        }
        if let Some(key) = plan.keys[at] {
            scope.files.insert(key, answer.clone());
        }
    }
    if let Some(rest) = rest {
        for dependency in rest.dependencies {
            if !dependencies.iter().any(|known| known.path == dependency.path) {
                dependencies.push(dependency);
            }
        }
        report.findings.extend(rest.findings);
        report.unknowns.extend(rest.unknowns);
        report.skipped.extend(rest.skipped);
        for item in rest.unread {
            let _ = unread.entry(item.path).or_insert(item.reason);
        }
    }
    report.findings.extend(extra.findings);
    report.unread = unread.into_iter().map(|(path, reason)| super::executor::unread(path, reason)).collect();
    report.dependencies = dependencies.clone();
    scope.dependencies = dependencies;
    (report, belongs.then_some(scope))
}

/// The plan of `request` against `store`, none when the store cannot serve it.
fn plan(request: &LintRequest, engine: &LinterLintEngine, directory: &Path, build: &Digest, store: &Store) -> Option<Plan> {
    let Corpus::Listed(names) = &request.corpus else { return None };
    if !request.beside.is_empty() {
        return None;
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(build);
    for language in &engine.languages.0 {
        field(&mut hasher, language.name().0.as_bytes());
        field(&mut hasher, language.build().0.as_bytes());
    }
    // The rules and everything else the request says beside its files: a change in any of them is another scope.
    let said = format!(
        "{:?}",
        (&request.source_language, &request.root, &request.project_search_scope, &request.given_project_inventory, &request.rules, &request.files, &request.hosted)
    );
    field(&mut hasher, said.as_bytes());
    let mut sorted: Vec<&str> = names.iter().map(|name| &*name.0).collect();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return None;
    }
    for name in sorted {
        field(&mut hasher, name.as_bytes());
    }
    // The project's cross-file reads inspect which paths exist and of what kind: the tree below the root, but for what the languages skip
    // and the store's own directory, is part of the scope.
    let skipped: Vec<&str> = engine.skipped.iter().map(|name| &**name).collect();
    tree(&mut hasher, &request.root, &request.root, directory, &skipped).ok()?;
    let scope: Key = hasher.finalize().as_bytes()[..16].try_into().ok()?;
    // The files the project read observed, above the root too: a record is current only while each of them is as it was.
    let held = store
        .scopes
        .iter()
        .find(|(key, _)| *key == scope)
        .map(|(_, held)| held.clone())
        .filter(|held| dependencies_current(&request.root, &held.dependencies));
    let mut keys = Vec::with_capacity(names.len());
    let mut hits = Vec::with_capacity(names.len());
    for name in names {
        let path = request.root.join(&*name.0);
        let key = std::fs::metadata(&path).ok().and_then(|metadata| {
            use std::os::unix::fs::PermissionsExt as _;
            let bytes = std::fs::read(&path).ok()?;
            let mut hasher = blake3::Hasher::new();
            hasher.update(&scope);
            field(&mut hasher, name.0.as_bytes());
            hasher.update(&metadata.permissions().mode().to_le_bytes());
            hasher.update(blake3::hash(&bytes).as_bytes());
            hasher.finalize().as_bytes()[..16].try_into().ok()
        });
        hits.push(key.and_then(|key: Key| held.as_ref().and_then(|held| held.files.get(&key).cloned())));
        keys.push(key);
    }
    Some(Plan { scope, names: names.clone(), keys, hits, held })
}

/// Whether every project dependency a record holds is as it was when the record was made: a present file with the same bytes, a missing
/// file still missing. A path may leave the root (`../`): the project read looks above it for configuration.
fn dependencies_current(root: &Path, dependencies: &[ProjectDependency]) -> bool {
    dependencies.iter().all(|dependency| {
        let read = std::fs::read(root.join(&*dependency.path.0));
        match (&dependency.state, read) {
            (ProjectDependencyState::Present(fingerprint), Ok(bytes)) => DependencyFingerprint::from(DependencyBytes(&bytes)) == *fingerprint,
            (ProjectDependencyState::Missing, Err(error)) => error.kind() == std::io::ErrorKind::NotFound,
            _ => false,
        }
    })
}

fn file_of(subject: &Slot) -> Option<&str> {
    match subject {
        Slot::Is(Cell::File(path)) => Some(&path.0),
        _ => None,
    }
}

fn field(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// Hashes the names and kinds below `directory`, in order.
fn tree(hasher: &mut blake3::Hasher, root: &Path, directory: &Path, store: &Path, skipped: &[&str]) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_unstable_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if path == store || normalized(&path) == normalized(store) {
            continue;
        }
        let kind = entry.file_type()?;
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let name = relative.to_string_lossy();
        if kind.is_dir() && skipped.iter().any(|skip| *skip == &*entry.file_name().to_string_lossy() || *skip == &*name) {
            continue;
        }
        field(hasher, relative.as_os_str().as_encoded_bytes());
        if kind.is_file() {
            hasher.update(&[1]);
        } else if kind.is_dir() {
            hasher.update(&[2]);
            tree(hasher, root, &path, store, skipped)?;
        } else if kind.is_symlink() {
            hasher.update(&[3]);
            field(hasher, std::fs::read_link(&path)?.as_os_str().as_encoded_bytes());
        } else {
            hasher.update(&[4]);
        }
    }
    Ok(())
}

fn normalized(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_owned())
}

/// What identifies the program that wrote a store: a store of another build is ignored.
fn build_identity() -> Digest {
    let mut hasher = blake3::Hasher::new();
    field(&mut hasher, env!("CARGO_PKG_VERSION").as_bytes());
    if let Ok(metadata) = std::fs::metadata("/proc/self/exe").or_else(|_| std::env::current_exe().and_then(std::fs::metadata)) {
        hasher.update(&metadata.len().to_le_bytes());
        if let Ok(modified) = metadata.modified()
            && let Ok(since) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            hasher.update(&since.as_nanos().to_le_bytes());
        }
    }
    *hasher.finalize().as_bytes()
}

// ---- the form of the file: a checksum, then the body, in little-endian numbers and length-prefixed text ----

struct Writer(Vec<u8>);

impl Writer {
    fn number(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn bytes(&mut self, bytes: &[u8]) {
        self.number(bytes.len() as u64);
        self.0.extend_from_slice(bytes);
    }
    fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
    }
    fn slot(&mut self, slot: &Slot) {
        match slot {
            Slot::Is(Cell::File(path)) => {
                self.0.push(1);
                self.text(&path.0);
            }
            Slot::Is(Cell::Text(text)) => {
                self.0.push(2);
                self.text(&text.0);
            }
            Slot::Is(Cell::Number(number)) => {
                self.0.push(3);
                self.number(number.0 as u64);
            }
            Slot::Is(Cell::Span(span)) => {
                self.0.push(4);
                self.number(u64::from(span.start.0));
                self.number(u64::from(span.end.0));
            }
            _ => self.0.push(0),
        }
    }
    fn unreadable(&mut self, reason: &Unreadable) {
        match reason {
            Unreadable::Failed(failure) => {
                self.0.push(2);
                self.text(&failure.0);
            }
            _ => self.0.push(1),
        }
    }
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn number(&mut self) -> Option<u64> {
        let (head, rest) = self.0.split_at_checked(8)?;
        self.0 = rest;
        Some(u64::from_le_bytes(head.try_into().ok()?))
    }
    fn byte(&mut self) -> Option<u8> {
        let (head, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(*head)
    }
    fn bytes(&mut self) -> Option<&'a [u8]> {
        let length = usize::try_from(self.number()?).ok()?;
        let (head, rest) = self.0.split_at_checked(length)?;
        self.0 = rest;
        Some(head)
    }
    fn text(&mut self) -> Option<Box<str>> {
        Some(std::str::from_utf8(self.bytes()?).ok()?.into())
    }
    fn slot(&mut self) -> Option<Slot> {
        Some(match self.byte()? {
            0 => Slot::Nothing,
            1 => Slot::Is(Cell::File(RelativePath(self.text()?))),
            2 => Slot::Is(Cell::Text(HostText(self.text()?))),
            3 => Slot::Is(Cell::Number(HostNumber(self.number()? as i64))),
            4 => Slot::Is(Cell::Span(Span { start: Offset(u32::try_from(self.number()?).ok()?), end: Offset(u32::try_from(self.number()?).ok()?) })),
            _ => return None,
        })
    }
    fn unreadable(&mut self) -> Option<Unreadable> {
        Some(match self.byte()? {
            2 => Unreadable::Failed(ReadFailure(self.text()?)),
            _ => Unreadable::NoLanguage,
        })
    }
    fn rule(&mut self) -> Option<RuleId> {
        Some(RuleId(self.text()?))
    }
}

fn write_answer(out: &mut Writer, answer: &Answer) {
    out.number(answer.findings.len() as u64);
    for finding in &answer.findings {
        out.text(&finding.rule.0);
        out.slot(&finding.subject);
        out.number(finding.key.0.len() as u64);
        for slot in &finding.key.0 {
            out.slot(slot);
        }
        match finding.place {
            Place::At { line, column } => {
                out.0.push(1);
                out.number(u64::from(line.0));
                out.number(u64::from(column.0));
            }
            _ => out.0.push(0),
        }
        out.number(finding.columns.len() as u64);
        for column in &finding.columns {
            out.text(&column.name.0);
            out.slot(&column.value);
        }
    }
    out.number(answer.unknowns.len() as u64);
    for (rule, message) in &answer.unknowns {
        out.text(&rule.0);
        out.text(&message.0);
    }
    match &answer.unread {
        Some(reason) => {
            out.0.push(1);
            out.unreadable(reason);
        }
        None => out.0.push(0),
    }
    out.number(answer.skipped.len() as u64);
    for (rule, _) in &answer.skipped {
        out.text(&rule.0);
    }
}

fn read_answer(input: &mut Reader<'_>) -> Option<Answer> {
    let mut answer = Answer::default();
    for _ in 0..input.number()? {
        let rule = input.rule()?;
        let subject = input.slot()?;
        let mut key = Vec::new();
        for _ in 0..input.number()? {
            key.push(input.slot()?);
        }
        let place = match input.byte()? {
            1 => Place::At { line: Line(u32::try_from(input.number()?).ok()?), column: Column(u32::try_from(input.number()?).ok()?) },
            _ => Place::Missing,
        };
        let mut columns = Vec::new();
        for _ in 0..input.number()? {
            columns.push(FindingColumn { name: ColumnName(input.text()?), value: input.slot()? });
        }
        answer.findings.push(super::executor::finding(rule, subject, FindingKey(key), place, columns));
    }
    for _ in 0..input.number()? {
        answer.unknowns.push((input.rule()?, Message(input.text()?)));
    }
    if input.byte()? == 1 {
        answer.unread = Some(input.unreadable()?);
    }
    for _ in 0..input.number()? {
        answer.skipped.push((input.rule()?, SkipReason::SyntaxRejected));
    }
    Some(answer)
}

fn load(directory: &Path, build: &Digest) -> Option<Store> {
    let bytes = std::fs::read(directory.join(FILE)).ok()?;
    let (checksum, body) = bytes.split_at_checked(32)?;
    if blake3::hash(body).as_bytes() != checksum {
        return None;
    }
    let mut input = Reader(body);
    let (magic, rest) = input.0.split_at_checked(MAGIC.len())?;
    input.0 = rest;
    let (stored, rest) = input.0.split_at_checked(32)?;
    input.0 = rest;
    if magic != MAGIC || stored != build {
        return None;
    }
    let mut store = Store::default();
    for _ in 0..input.number()? {
        let (key, rest) = input.0.split_at_checked(16)?;
        input.0 = rest;
        let mut scope = Scope::default();
        for _ in 0..input.number()? {
            scope.not_file_local.push(input.rule()?);
        }
        for _ in 0..input.number()? {
            let path = RelativePath(input.text()?);
            let state = match input.byte()? {
                1 => {
                    let (fingerprint, rest) = input.0.split_at_checked(16)?;
                    input.0 = rest;
                    ProjectDependencyState::Present(DependencyFingerprint(fingerprint.try_into().ok()?))
                }
                _ => ProjectDependencyState::Missing,
            };
            scope.dependencies.push(ProjectDependency { path, state });
        }
        for _ in 0..input.number()? {
            let (file, rest) = input.0.split_at_checked(16)?;
            input.0 = rest;
            scope.files.insert(file.try_into().ok()?, read_answer(&mut input)?);
        }
        store.scopes.push((key.try_into().ok()?, scope));
    }
    Some(store)
}

fn publish(directory: &Path, build: &Digest, store: &Store) -> std::io::Result<()> {
    let mut body = Writer(Vec::new());
    body.0.extend_from_slice(MAGIC);
    body.0.extend_from_slice(build);
    body.number(store.scopes.len() as u64);
    for (key, scope) in &store.scopes {
        body.0.extend_from_slice(key);
        body.number(scope.not_file_local.len() as u64);
        for rule in &scope.not_file_local {
            body.text(&rule.0);
        }
        body.number(scope.dependencies.len() as u64);
        for dependency in &scope.dependencies {
            body.text(&dependency.path.0);
            match dependency.state {
                ProjectDependencyState::Present(fingerprint) => {
                    body.0.push(1);
                    body.0.extend_from_slice(&fingerprint.0);
                }
                _ => body.0.push(0),
            }
        }
        body.number(scope.files.len() as u64);
        for (file, answer) in &scope.files {
            body.0.extend_from_slice(file);
            write_answer(&mut body, answer);
        }
    }
    let checksum = blake3::hash(&body.0);
    let path = directory.join(FILE);
    if std::fs::read(&path).ok().is_some_and(|previous| previous.split_at_checked(32).is_some_and(|(sum, _)| sum == checksum.as_bytes())) {
        return Ok(());
    }
    std::fs::create_dir_all(directory)?;
    let temporary = directory.join(format!("{FILE}.{}.tmp", std::process::id()));
    {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(checksum.as_bytes())?;
        file.write_all(&body.0)?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, &path)?;
    std::fs::File::open(directory)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record holds the files the project read observed: one above the root, one beside it, one that was missing. It is current while
    /// they are as they were, and not when the file above the root changes, when the missing one appears, or when one disappears.
    #[test]
    fn a_record_is_current_only_while_the_files_the_project_read_observed_are_as_they_were() {
        let parent = tempfile::tempdir().expect("a directory");
        let root = parent.path().join("repository");
        std::fs::create_dir_all(&root).expect("a root");
        std::fs::write(parent.path().join("above.toml"), "a = 1\n").expect("a file above");
        std::fs::write(root.join("beside.toml"), "b = 1\n").expect("a file beside");
        let present = |path: &str, text: &str| ProjectDependency {
            path: RelativePath(path.into()),
            state: ProjectDependencyState::Present(DependencyFingerprint::from(DependencyBytes(text.as_bytes()))),
        };
        let observed = vec![
            present("../above.toml", "a = 1\n"),
            present("beside.toml", "b = 1\n"),
            ProjectDependency { path: RelativePath("later.toml".into()), state: ProjectDependencyState::Missing },
        ];
        assert!(dependencies_current(&root, &observed), "nothing changed");
        std::fs::write(parent.path().join("above.toml"), "a = 2\n").expect("an edit above the root");
        assert!(!dependencies_current(&root, &observed), "a change above the root is a change");
        std::fs::write(parent.path().join("above.toml"), "a = 1\n").expect("restored");
        assert!(dependencies_current(&root, &observed));
        std::fs::write(root.join("later.toml"), "c = 1\n").expect("a file that was missing appears");
        assert!(!dependencies_current(&root, &observed), "a file that was missing and is there now is a change");
        std::fs::remove_file(root.join("later.toml")).expect("removed");
        std::fs::remove_file(root.join("beside.toml")).expect("a file disappears");
        assert!(!dependencies_current(&root, &observed), "a file that disappeared is a change");
    }
}
