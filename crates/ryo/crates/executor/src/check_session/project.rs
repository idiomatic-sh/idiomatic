//! A closed inventory names both the primary manifests and the files whose notes
//! each provider may consume. Its disk authority is supplied separately from
//! the directory in which the caller staged effective source versions.
use super::{Result, Table, finish, reader};
use crate::languages as l;
use ryo_execution_domain::domain::coverage::types::{Lacking, LackingKind, Partial, PartialPlace};
use ryo_language::domain::{
    files::types::ProjectEntries,
    project_context::{service::observed, types::EntryObservation},
};
use ryo_relations::domain::relations::service::name;
use ryo_values::domain::stored_facts::types::PartialCause;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    rc::Rc,
};

/// Compact project input indexed by file. Evaluator tables are made only
/// inside the file visit so they use its file IDs and symbol dictionary.
pub struct Layout {
    pub relations: Vec<l::Relation>,
    pub parts: Vec<Part>,
    pub words: Vec<Box<[u8]>>,
    /// What the relations read in the visit need: the request the project is read under, once, and where its entries are observed.
    pub visit: Option<Visit>,
}

pub struct Part {
    pub columns: Vec<(String, finish::Ty)>,
    pub rows: std::collections::HashMap<String, Vec<u64>>,
    /// Whether the rows are read in the visit of each file, from its own notes, instead of being kept here.
    pub in_visit: bool,
}

/// The project as the visit of one file reads the relations about that file alone: every file's call shares this one request (the whole
/// inventory in `files`, no notes and no manifests) and the authority and environment its entries are observed under.
pub struct Visit {
    pub request: l::ProjectRequest,
    pub authority: Authority,
    pub environment: Option<std::sync::Arc<Environment>>,
}

/// What one file's own read of its project relations came to.
pub struct FileRead {
    /// Each relation read in the visit, by the alias its table is declared under, with the file's rows.
    pub tables: Vec<(String, Rc<Table>)>,
    /// What the language could not decide of the file, by relation: the file's partial coverage.
    pub partials: Vec<(l::Relation, String, ryo_relations::domain::relations::types::Missing)>,
}

pub fn layout<L: super::FileSource>(
    readers: &[L],
    authority: &Authority,
    inventory: &[String],
    environment: Option<std::sync::Arc<Environment>>,
    relations: &[l::Relation],
) -> Option<Layout> {
    use ryo_relations::domain::catalog::{service::needs_notes, types::RelationNotes};
    // A relation that needs the file's notes is read in the visit of the file, when its language answers it from one file's notes alone.
    let (in_visit, kept): (Vec<l::Relation>, Vec<l::Relation>) =
        relations.iter().partition(|relation| needs_notes(**relation) == RelationNotes::Needed);
    let mut world = super::World::default();
    world.declare_words(readers);
    let visit = if in_visit.is_empty() {
        None
    } else {
        Some(visit_of(readers, authority, inventory, &environment, &in_visit)?)
    };
    let environment_ref = environment.as_deref();
    let input = Rc::new(Contribution::default());
    let population = collect(&mut world, readers, authority, inventory, environment_ref,
        &vec![input; inventory.len()], &kept).ok()?;
    // The established closure path already reports incomplete and refused
    // answers with their exact project or file scope.
    if !population.partials.is_empty() || !population.refused.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for &relation in relations {
        if in_visit.contains(&relation) {
            let (_, names, types) = world.run.catalog(name(relation).0)?;
            let mut files = types.iter().filter(|ty| **ty == finish::Ty::File);
            files.next()?;
            if files.next().is_some() { return None; }
            parts.push(Part {
                columns: names.iter().cloned().zip(types.iter().copied()).collect(),
                rows: Default::default(),
                in_visit: true,
            });
            continue;
        }
        let table = population.tables.get(name(relation).0)?;
        let mut file_columns = table.tys.iter().enumerate()
            .filter_map(|(c, ty)| (*ty == finish::Ty::File).then_some(c));
        let key = file_columns.next()?;
        if file_columns.next().is_some()
            || table.tys.iter().any(|ty| matches!(ty, finish::Ty::Node | finish::Ty::Span))
        {
            return None;
        }
        let mut rows: std::collections::HashMap<String, Vec<u64>> = Default::default();
        for row in 0..table.n {
            let path = world.run.paths.get(usize::try_from(table.cols[key][row]).ok()?)?;
            rows.entry(path.clone()).or_default()
                .extend(table.cols.iter().map(|column| column[row]));
        }
        parts.push(Part {
            columns: table.names.iter().cloned().zip(table.tys.iter().copied()).collect(),
            rows,
            in_visit: false,
        });
    }
    Some(Layout {
        relations: relations.to_vec(),
        parts,
        words: world.run.texts.iter().map(|word| Box::from(word.as_ref())).collect(),
        visit,
    })
}

impl Layout {
    /// Declare each relation under its own alias and say which alias holds which.
    pub fn declare(&self, run: &mut finish::Run, prefix: &str) -> Vec<(l::Relation, String)> {
        self.relations.iter().zip(&self.parts).enumerate().map(|(at, (&relation, part))| {
            let alias = format!("{prefix}/{at}");
            let columns: Vec<_> = part.columns.iter().map(|(name, ty)| (name.as_str(), *ty)).collect();
            run.declare(&alias, &columns, finish::Supplier::Provider);
            (relation, alias)
        }).collect()
    }

    /// This relation's rows for the file as screening columns, with symbols interned in the file's own word table.
    pub fn witness_columns(&self, part: usize, path: &str, file: l::FileId, words: &mut l::FileWords, out: &mut l::FilledColumns) -> bool {
        let part = &self.parts[part];
        // The rows read in the visit are not known before it: a screen cannot answer from them.
        if part.in_visit {
            return false;
        }
        let cells = part.rows.get(path).map_or(&[][..], Vec::as_slice);
        out.0 = (0..part.columns.len()).map(|_| l::FilledColumn(Vec::new())).collect();
        for row in cells.chunks_exact(part.columns.len()) {
            for ((column, (_, ty)), &cell) in out.0.iter_mut().zip(&part.columns).zip(row) {
                column.0.push(match ty {
                    finish::Ty::File => l::CellWord(u64::from(file.0)),
                    finish::Ty::Sym if cell == finish::NULL => l::CellWord(u64::from(l::NO_SYMBOL.0)),
                    finish::Ty::Sym => match std::str::from_utf8(&self.words[cell as usize]) {
                        Ok(text) => l::facts::intern(words, l::NameText(text)),
                        Err(_) => return false,
                    },
                    _ => l::CellWord(cell),
                });
            }
        }
        true
    }

    /// The rows each relation of the layout gives the file: the ones kept here, and the ones `read` brings, which the file's own visit read.
    pub fn slice(&self, run: &mut finish::Run, aliases: &[(l::Relation, String)], path: &str, file: l::FileId, read: Option<&FileRead>) -> Vec<(String, Rc<Table>)> {
        aliases.iter().zip(&self.parts).map(|((_, alias), part)| {
            let (_, names, types) = run.catalog(alias).expect("bound project layout");
            if part.in_visit {
                let read = read.and_then(|read| read.tables.iter().find(|(held, _)| held == alias));
                return (alias.clone(), read.map_or_else(|| Rc::new(Table::with_rows(names, types, 0)), |(_, table)| Rc::clone(table)));
            }
            let cells = part.rows.get(path).map_or(&[][..], Vec::as_slice);
            let mut table = Table::with_rows(names, types, cells.len() / part.columns.len());
            for row in cells.chunks_exact(part.columns.len()) {
                for ((column, (_, ty)), &cell) in table.cols.iter_mut().zip(&part.columns).zip(row) {
                    column.push(match ty {
                        finish::Ty::File => u64::from(file.0),
                        finish::Ty::Sym if cell != finish::NULL => u64::from(run.sym(&self.words[cell as usize])),
                        _ => cell,
                    });
                }
                table.n += 1;
            }
            (alias.clone(), Rc::new(table))
        }).collect()
    }
}

impl Layout {
    /// Whether some relation of the layout is read in the visit of each file.
    pub fn reads_in_visit(&self) -> bool {
        self.visit.is_some()
    }

    /// The rows of the relations read in the visit, about the file `path` (file `file` of the run), from its own notes: asked of the
    /// file's own reader, with the entries it names observed as the project phase observes them. A file no reader reads, or a reader that
    /// does not answer a file alone, has no rows.
    ///
    /// # Errors
    ///
    /// What the reader refuses, or that the file is not in the inventory.
    pub fn read_file<L: super::FileSource>(
        &self,
        reader: Option<&L>,
        public: &std::collections::HashMap<u32, u32>,
        run: &mut finish::Run,
        aliases: &[(l::Relation, String)],
        path: &str,
        file: l::FileId,
        notes: &l::Bytes,
    ) -> Result<FileRead> {
        let mut read = FileRead { tables: Vec::new(), partials: Vec::new() };
        let Some(visit) = &self.visit else { return Ok(read); };
        let empty = |run: &mut finish::Run, alias: &str| {
            let (_, names, types) = run.catalog(alias).expect("bound project layout");
            Rc::new(Table::with_rows(names, types, 0))
        };
        let Some(reader) = reader else {
            read.tables = aliases.iter().zip(&self.parts).filter(|(_, part)| part.in_visit)
                .map(|((_, alias), _)| (alias.clone(), empty(run, alias))).collect();
            return Ok(read);
        };
        let at = visit.request.files.binary_search_by(|listed| listed.0.as_ref().cmp(path))
            .map_err(|_| format!("{path} is not in the project inventory"))?;
        let mut entries = ProjectEntries::default();
        reader.file_entries(&visit.request, at, notes, &mut entries)?;
        entries.0.sort_unstable();
        entries.0.dedup();
        let observed = observed(
            entries.0.into_iter().map(|entry| match &visit.environment {
                Some(env) if visit.authority.disk().is_some() && env.authority == visit.authority => {
                    EntryObservation { state: env.physical(Path::new(entry.0.as_ref())), path: entry }
                }
                _ => visit.authority.observe(&visit.request.files, entry),
            }).collect(),
        );
        let mut symbols = l::Symbols::default();
        let mut rows = l::contract::Collect::default();
        let answered = reader.file_rows(&visit.request, at, notes, &observed, &mut symbols, &mut rows)?;
        let mut tables: Vec<(l::Relation, String, Table)> = aliases.iter().zip(&self.parts)
            .filter(|(_, part)| part.in_visit)
            .map(|((relation, alias), _)| {
                let (_, names, types) = run.catalog(alias).expect("bound project layout");
                (*relation, alias.clone(), Table::with_rows(names, types, 0))
            })
            .collect();
        if answered.is_some() {
            'rows: for (relation, cells) in rows.rows() {
                let Some((_, _, table)) = tables.iter_mut().find(|(wanted, _, _)| *wanted == relation) else {
                    return Err(format!("unrequested project relation {}", name(relation).0));
                };
                let mut kept = Vec::with_capacity(cells.len());
                for (ty, word) in table.tys.iter().zip(cells) {
                    kept.push(match ty {
                        finish::Ty::Sym => u64::from(reader::symbol(run, public, &symbols, &[], l::Symbol(word as u32))),
                        finish::Ty::File if word == u64::from(l::NO_FILE.0) => finish::NULL,
                        // A row about another file is not this file's.
                        finish::Ty::File if word != at as u64 => continue 'rows,
                        finish::Ty::File => u64::from(file.0),
                        _ => word,
                    });
                }
                for (column, word) in table.cols.iter_mut().zip(kept) {
                    column.push(word);
                }
                table.n += 1;
            }
            for (relation, reason, missing) in rows.partials.drain(..) {
                read.partials.push((relation, reason, missing));
            }
            for (relation, id, reason) in rows.partials_at.drain(..) {
                if id.0 as usize == at {
                    read.partials.push((relation, reason, ryo_relations::domain::relations::types::Missing::Any));
                }
            }
        }
        read.tables = tables.into_iter().map(|(_, alias, table)| (alias, Rc::new(table))).collect();
        Ok(read)
    }
}

#[derive(Clone, Default)]
pub struct Contribution {
    pub reader: Option<usize>,
    pub notes: l::Bytes,
    pub manifest: Option<l::ManifestText>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
pub enum Authority {
    Given,
    Disk(String),
    Scoped { disk: Option<String>, roots: Vec<String> },
}

impl super::World {
    pub fn declare_words<L: super::FileSource>(&mut self, readers: &[L]) {
        self.run.empty = self.run.sym(b"");
        reader::public_words(&mut self.run, &[]);
        self.project_words = readers.iter()
            .map(|reader| reader::public_words(&mut self.run, &reader.words()))
            .collect();
    }
}

impl Authority {
    pub fn disk(&self) -> Option<&str> {
        match self {
            Self::Given => None,
            Self::Disk(root) => Some(root),
            Self::Scoped { disk, .. } => disk.as_deref(),
        }
    }

    pub fn with_roots(self, roots: Vec<String>) -> Self {
        Self::Scoped { disk: self.disk().map(str::to_owned), roots }
    }

    pub fn observe(&self, files: &[l::FilePath], path: l::FilePath) -> EntryObservation {
        use l::ObservedEntryState as State;
        let state = match self.disk() {
            None => {
                let file = files.binary_search(&path).is_ok();
                let prefix = format!("{path}/");
                let i = files.partition_point(|p| p.0.as_ref() < prefix.as_str());
                let directory = files.get(i).is_some_and(|p| p.starts_with(&prefix));
                match (file, directory) {
                    (true, false) => State::File,
                    (false, true) => State::Directory,
                    _ => State::Unknown,
                }
            }
            Some(root) => match std::fs::metadata(Path::new(root).join(&*path)) {
                Ok(m) if m.is_file() => State::File,
                Ok(m) if m.is_dir() => State::Directory,
                Ok(_) => State::Other,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => State::Missing,
                Err(_) => State::Unknown,
            },
        };
        EntryObservation { path, state }
    }
}

/// One materialization per immutable project and demanded population set. Column
/// words are in the project run's interner; file cells number this inventory.
/// A negative lookup is valid only in a completed population.
pub struct Population {
    pub tables: BTreeMap<String, Rc<Table>>,
    pub partials: Vec<Partial>,
    pub refused: Vec<l::Refused>,
    pub consulted: Vec<l::FilePath>,
    pub observations: Vec<EntryObservation>,
}

/// The request the project is read under: the listed files and manifests, no notes yet and no observation, over `authority`.
fn request_of(
    authority: &Authority,
    files: Vec<l::FilePath>,
    manifests: Vec<l::ManifestFile>,
    wanted: &[l::Relation],
) -> l::ProjectRequest {
    l::ProjectRequest {
        files,
        manifests,
        notes: Vec::new(),
        wanted: wanted.to_vec(),
        repository: if authority.disk().is_some() { l::Repository::Disk } else { l::Repository::Given },
        root: l::RootPath(authority.disk().unwrap_or_default().into()),
        search_scope: match authority {
            Authority::Scoped { roots, .. } => ryo_language::domain::project_context::types::ProjectSearchScope::Declared(
                roots.iter().map(|root| l::FilePath(root.as_str().into())).collect()
            ),
            _ => Default::default(),
        },
        given_inventory: Default::default(),
        observed_entries: Default::default(),
    }
}

/// The visit's project, when some reader answers the relations `wanted` from one file's notes alone: the probe is one file with no notes,
/// so a language that does not implement the per-file read (and answers `Unsupported`) keeps its relations on the closed path.
fn visit_of<L: super::FileSource>(
    readers: &[L],
    authority: &Authority,
    inventory: &[String],
    environment: &Option<std::sync::Arc<Environment>>,
    wanted: &[l::Relation],
) -> Option<Visit> {
    if inventory.is_empty() {
        return None;
    }
    let request = request_of(
        authority,
        inventory.iter().map(|path| l::FilePath(path.as_str().into())).collect(),
        Vec::new(),
        wanted,
    );
    let mut symbols = l::Symbols::default();
    let mut rows = l::contract::Collect::default();
    let none = l::Bytes::default();
    let observed = l::ObservedEntries::default();
    let answered = readers.iter().any(|reader| {
        matches!(reader.file_rows(&request, 0, &none, &observed, &mut symbols, &mut rows), Ok(Some(_)))
    });
    answered.then(|| Visit { request, authority: authority.clone(), environment: environment.clone() })
}

pub fn collect<L: super::FileSource>(
    world: &mut super::World,
    readers: &[L],
    authority: &Authority,
    inventory: &[String],
    environment: Option<&Environment>,
    contributions: &[Rc<Contribution>],
    wanted: &[l::Relation],
) -> Result<Population> {
    // A missing repository root cannot establish that its children are absent.
    if let Some(root) = authority.disk() {
        let entry = std::fs::metadata(root)
            .map_err(|e| format!("unavailable project root {root}: {e}"))?;
        if !entry.is_dir() {
            return Err(format!("project root is not a directory: {root}"));
        }
    }
    if inventory.len() != contributions.len() {
        return Err("incomplete project inputs".into());
    }
    let stage = finish::operators::enter(usize::MAX - 20, 0);
    let files: Vec<_> = inventory
        .iter()
        .map(|s| l::FilePath(s.as_str().into()))
        .collect();
    let manifests: Vec<_> = files
        .iter()
        .zip(contributions)
        .filter_map(|(path, c)| {
            Some(l::ManifestFile {
                path: path.clone(),
                text: c.manifest.clone()?,
            })
        })
        .collect();
    let mut request = request_of(authority, files, manifests, wanted);
    if let Some(start) = stage {
        finish::operators::leave(start, "collect-files", request.files.len(), true);
    }
    let stage = finish::operators::enter(usize::MAX - 21, 0);
    let mut entries = ProjectEntries::default();
    let notes = |index| {
        contributions
            .iter()
            .map(|c| {
                if c.reader == Some(index) {
                    c.notes.clone()
                } else {
                    l::Bytes::default()
                }
            })
            .collect()
    };
    for (i, lang) in readers.iter().enumerate() {
        request.notes = notes(i);
        lang.entries(&request, &mut entries)?;
    }
    entries.0.sort_unstable();
    entries.0.dedup();
    if let Some(start) = stage {
        finish::operators::leave(start, "collect-entries", entries.0.len(), true);
    }
    let stage = finish::operators::enter(usize::MAX - 22, 0);
    request.observed_entries = observed(
        entries
            .0
            .into_iter()
            .map(|path| match environment {
                Some(env) if authority.disk().is_some() && env.authority == *authority => {
                    EntryObservation { state: env.physical(Path::new(path.0.as_ref())), path }
                }
                _ => authority.observe(&request.files, path),
            })
            .collect(),
    );
    if let Some(start) = stage {
        finish::operators::leave(start, "collect-observe", 0, true);
    }
    let stage = finish::operators::enter(usize::MAX - 23, 0);
    let mut tables = BTreeMap::new();
    for &rel in wanted {
        let n = name(rel).0;
        let (_, cols, tys) = world
            .run
            .catalog(n)
            .ok_or_else(|| format!("undeclared project relation {n}"))?;
        tables.insert(n.to_owned(), Table::with_rows(cols, tys, 0));
    }
    let mut file_numbers: Vec<_> = inventory.iter().map(|p| world.file(p)).collect();
    let mut unlisted = Vec::new();
    let mut refused = Vec::new();
    let mut consulted = BTreeSet::new();
    let mut partials = Vec::new();
    if let Some(start) = stage {
        finish::operators::leave(start, "collect-numbers", file_numbers.len(), true);
    }
    let stage = finish::operators::enter(usize::MAX - 24, 0);
    let mut symbols = l::Symbols::default();
    for (i, lang) in readers.iter().enumerate() {
        request.notes = notes(i);
        let mut rows = l::contract::Collect::default();
        let reply = lang.project(&request, &mut symbols, &mut rows)?;
        if !reply.unlisted.is_empty() {
            if unlisted.is_empty() {
                file_numbers.extend(reply.unlisted.iter().map(|p| world.file(&p.0)));
                consulted.extend(reply.unlisted.iter().cloned());
                unlisted = reply.unlisted;
            } else if unlisted != reply.unlisted {
                return Err(
                    "providers assigned different paths to the same project file number".into(),
                );
            }
        }
        refused.extend(reply.refused);
        consulted.extend(reply.consulted);
        for (rel, cells) in rows.rows() {
            let n = name(rel).0;
            let t = tables
                .get_mut(n)
                .ok_or_else(|| format!("unrequested project relation {n}"))?;
            for ((col, ty), word) in t.cols.iter_mut().zip(t.tys.iter()).zip(cells) {
                col.push(match ty {
                    finish::Ty::Sym => u64::from(reader::symbol(
                        &mut world.run,
                        &world.project_words[i],
                        &symbols,
                        &[],
                        l::Symbol(word as u32),
                    )),
                    finish::Ty::File if word == u64::from(l::NO_FILE.0) => finish::NULL,
                    finish::Ty::File if word >= file_numbers.len() as u64 => {
                        return Err(format!("unbound project file in {n}"));
                    }
                    finish::Ty::File => u64::from(file_numbers[word as usize]),
                    _ => word,
                });
            }
            t.n += 1;
        }
        for (relation, reason, missing) in rows.partials {
            let lacking = match missing {
                ryo_relations::domain::relations::types::Missing::Any => Lacking::Any,
                ryo_relations::domain::relations::types::Missing::Kind(k) => {
                    Lacking::Kind(LackingKind(k.0.into()))
                }
            };
            partials.push(Partial {
                relation,
                at: PartialPlace::Repository,
                lacking,
                cause: PartialCause(reason.into()),
            });
        }
        for (relation, id, reason) in rows.partials_at {
            let file = *file_numbers
                .get(id.0 as usize)
                .ok_or("unbound partial project file")?;
            partials.push(Partial {
                relation,
                at: PartialPlace::File(l::FileId(file)),
                lacking: Lacking::Any,
                cause: PartialCause(reason.into()),
            });
        }
    }
    if let Some(start) = stage {
        finish::operators::leave(start, "collect-provider", 0, true);
    }
    partials.sort();
    partials.dedup();
    Ok(Population {
        tables: tables.into_iter().map(|(n, t)| (n, Rc::new(t))).collect(),
        partials,
        refused,
        consulted: consulted.into_iter().collect(),
        observations: request.observed_entries.0.to_vec(),
    })
}

/// Source-independent observations shared by every contextual file in an immutable overlay.
pub struct Environment {
    pub id: std::sync::Arc<str>,
    pub authority: Authority,
    /// Durable authority, selected reader and effective overlay. The store
    /// separately verifies the disk entries the provider actually observed.
    pub version_scope: Option<[u8; 32]>,
    pub supplied: BTreeSet<String>,
    pub epoch: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub observations: std::sync::Mutex<(usize, BTreeMap<std::path::PathBuf, l::ObservedEntryState>)>,
}

impl Environment {
    pub fn new(id: std::sync::Arc<str>, authority: Authority, supplied: BTreeSet<String>,
        epoch: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> Self {
        Self { id, authority, supplied, version_scope: None, epoch, observations: Default::default() }
    }

    /// Borrow the complete set of physical lookups made in this admission.
    /// Supplied overlay entries have their own version proofs. Unknown states
    /// are passed through so the store can decline to retain an answer.
    pub fn observed(&self, mut read: impl FnMut(&Path, l::ObservedEntryState)) {
        let epoch = self.epoch.load(std::sync::atomic::Ordering::Relaxed);
        let memo = self.observations.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if memo.0 == epoch {
            for (path, state) in &memo.1 {
                read(path, *state);
            }
        }
    }

    // File and project suppliers read the same physical entry in this epoch.
    // Overlay entries are answered before this lookup and never enter the disk memo.
    pub fn physical(&self, path: &Path) -> l::ObservedEntryState {
        let epoch = self.epoch.load(std::sync::atomic::Ordering::Relaxed);
        let mut memo = self.observations.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if memo.0 != epoch {
            memo.1.clear();
            memo.0 = epoch;
        }
        if let Some(&state) = memo.1.get(path) { return state; }
        let state = path.to_str().map_or(l::ObservedEntryState::Unknown, |path|
            self.authority.observe(&[], l::FilePath(path.into())).state);
        memo.1.insert(path.into(), state);
        state
    }
}
impl l::FileProject for Environment {
    fn roots(&self) -> Option<&[String]> {
        match &self.authority {
            Authority::Scoped { roots, .. } => Some(roots),
            _ => None,
        }
    }

    fn entry(&self, path: &Path) -> l::ObservedEntryState {
        use l::ObservedEntryState as State;
        let Some(path) = path.to_str() else { return self.physical(path); };
        if self.supplied.contains(path) { return State::File; }
        let prefix = format!("{path}/");
        if self.supplied.range(prefix.clone()..).next().is_some_and(|p| p.starts_with(&prefix)) {
            return State::Directory;
        }
        self.physical(Path::new(path))
    }
}

#[cfg(test)]
#[path = "project_test.rs"]
mod tests;
