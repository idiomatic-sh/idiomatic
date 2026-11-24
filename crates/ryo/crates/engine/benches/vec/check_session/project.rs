//! A closed inventory names both the primary manifests and the files whose notes
//! each provider may consume. Its disk authority is supplied separately from
//! the directory in which the caller staged effective source versions.
use super::{Result, Table, finish, reader};
use ryo::languages as l;
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

#[derive(Clone, Default)]
pub(crate) struct Contribution {
    pub reader: Option<usize>,
    pub notes: l::Bytes,
    pub manifest: Option<l::ManifestText>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
pub(crate) enum Authority {
    Given,
    Disk(String),
}

impl super::World {
    pub(crate) fn declare_words<L: super::FileSource>(&mut self, readers: &[L]) {
        self.run.empty = self.run.sym(b"");
        reader::public_words(&mut self.run, &[]);
        self.project_words = readers.iter()
            .map(|reader| reader::public_words(&mut self.run, &reader.words()))
            .collect();
    }
}

impl Authority {
    fn observe(&self, files: &[l::FilePath], path: l::FilePath) -> EntryObservation {
        use l::ObservedEntryState as State;
        let state = match self {
            Self::Given => {
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
            Self::Disk(root) => match std::fs::metadata(Path::new(root).join(&*path)) {
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
pub(crate) struct Population {
    pub tables: BTreeMap<String, Rc<Table>>,
    pub partials: Vec<Partial>,
    pub refused: Vec<l::Refused>,
    pub consulted: Vec<l::FilePath>,
    pub observations: Vec<EntryObservation>,
}

pub(crate) struct ProjectInput<'a> {
    pub(crate) authority: &'a Authority,
    pub(crate) inventory: &'a [String],
    pub(crate) contributions: Vec<&'a Contribution>,
    pub(crate) wanted: Vec<l::Relation>,
}

pub(crate) struct ProviderPackets {
    replies: Vec<(l::Resolved, l::contract::Collect, l::Symbols)>,
    observations: Vec<EntryObservation>,
}

impl ProjectInput<'_> {
    pub(crate) fn read<L: super::FileSource>(&self, readers: &[L]) -> Result<ProviderPackets> {
        let Self { authority, inventory, contributions, wanted } = self;
        // A missing repository root cannot establish that its children are absent.
        if let Authority::Disk(root) = authority {
            let entry = std::fs::metadata(root)
                .map_err(|e| format!("unavailable project root {root}: {e}"))?;
            if !entry.is_dir() {
                return Err(format!("project root is not a directory: {root}"));
            }
        }
        if inventory.len() != contributions.len() {
            return Err("incomplete project inputs".into());
        }
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
        let mut request = l::ProjectRequest {
            files,
            manifests,
            notes: Vec::new(),
            wanted: wanted.clone(),
            repository: match authority {
                Authority::Given => l::Repository::Given,
                Authority::Disk(_) => l::Repository::Disk,
            },
            root: l::RootPath(match authority {
                Authority::Given => "".into(),
                Authority::Disk(root) => root.as_str().into(),
            }),
            search_scope: Default::default(),
            given_inventory: Default::default(),
            observed_entries: Default::default(),
        };
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
        request.observed_entries = observed(
            entries
                .0
                .into_iter()
                .map(|path| authority.observe(&request.files, path))
                .collect(),
        );
        let mut replies = Vec::with_capacity(readers.len());
        for (i, lang) in readers.iter().enumerate() {
            request.notes = notes(i);
            let mut rows = l::contract::Collect::default();
            let mut symbols = l::Symbols::default();
            let reply = lang.project(&request, &mut symbols, &mut rows)?;
            replies.push((reply, rows, symbols));
        }
        Ok(ProviderPackets { replies, observations: request.observed_entries.0.to_vec() })
    }

    pub(crate) fn publish(&self, world: &mut super::World, packets: ProviderPackets) -> Result<Population> {
        let Self { inventory, wanted, .. } = self;
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
        for (i, (reply, rows, symbols)) in packets.replies.into_iter().enumerate() {
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
        partials.sort();
        partials.dedup();
        Ok(Population {
            tables: tables.into_iter().map(|(n, t)| (n, Rc::new(t))).collect(),
            partials,
            refused,
            consulted: consulted.into_iter().collect(),
            observations: packets.observations,
        })
    }
}
