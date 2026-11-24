//! Immutable inputs shared by SDK calls. File selection depends on paths and glob sets;
//! authority and file contents still accompany each registration.
use super::{HostCell, Hosted, sdk};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

type Selection = (usize, usize, usize, usize);

pub(super) enum PathFilter {
    Literals(Vec<Box<str>>),
    Patterns(GlobSet),
}

impl PathFilter {
    pub(super) fn compile(names: &[sdk::Glob]) -> Result<Self, String> {
        // Globset compares literal paths byte-for-byte on Unix. On other platforms
        // its candidate normalizes native separators before matching.
        if cfg!(unix)
            && names.iter().all(|g| {
                let s: &str = g.as_ref();
                !s.is_empty() && !s.contains(['*', '?', '[', ']', '{', '}', '\\'])
            })
        {
            let mut paths: Vec<Box<str>> =
                names.iter().map(|g| Box::<str>::from(g.as_ref())).collect();
            paths.sort_unstable();
            paths.dedup();
            return Ok(Self::Literals(paths));
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
        Ok(Self::Patterns(builder.build().map_err(|e| e.to_string())?))
    }

    fn candidates(&self) -> Option<&[Box<str>]> {
        match self {
            Self::Literals(paths) if !paths.is_empty() => Some(paths),
            _ => None,
        }
    }

    fn matches(&self, path: &str, empty_matches: bool) -> bool {
        match self {
            Self::Literals(paths) if paths.is_empty() => empty_matches,
            Self::Literals(paths) => paths.binary_search_by(|p| p.as_ref().cmp(path)).is_ok(),
            Self::Patterns(set) => (empty_matches && set.is_empty()) || set.is_match(path),
        }
    }
}

#[derive(Default)]
pub(super) struct CallCache {
    paths: Vec<Rc<Vec<String>>>,
    path_hashes: HashMap<u64, Vec<usize>>,
    selections: HashMap<Selection, usize>,
    queries: HashSet<Rc<str>>,
    schemas: HashMap<(String, Vec<(String, String)>), usize>,
    rows: Vec<Rc<[HostCell]>>,
    by_hash: HashMap<(usize, u64), Vec<usize>>,
    tables: HashMap<(usize, Vec<usize>), Hosted>,
}

impl CallCache {
    // Both callers supply sorted, unique paths. Compare full content on hash collisions.
    fn intern_paths(&mut self, paths: &[&str]) -> usize {
        let mut hash = DefaultHasher::new();
        paths.hash(&mut hash);
        let bucket = self.path_hashes.entry(hash.finish()).or_default();
        if let Some(id) = bucket.iter().find(|&&id| {
            self.paths[id]
                .iter()
                .map(String::as_str)
                .eq(paths.iter().copied())
        }) {
            return *id;
        }
        let id = self.paths.len();
        self.paths
            .push(Rc::new(paths.iter().map(|&path| path.to_owned()).collect()));
        bucket.push(id);
        id
    }

    pub(super) fn inventory(&mut self, mut paths: Vec<&str>) -> (usize, Rc<Vec<String>>) {
        paths.sort_unstable();
        paths.dedup();
        let id = self.intern_paths(&paths);
        (id, self.paths[id].clone())
    }

    pub(super) fn files(
        &mut self,
        key: Selection,
        request: &PathFilter,
        include: &PathFilter,
        exclude: &PathFilter,
    ) -> Rc<Vec<String>> {
        if let Some(&id) = self.selections.get(&key) {
            return self.paths[id].clone();
        }
        let inventory = self.paths[key.0].clone();
        let literals = [request.candidates(), include.candidates()]
            .into_iter()
            .flatten()
            .min_by_key(|paths| paths.len());
        let admits = |path: &&str| {
            request.matches(path, true)
                && include.matches(path, true)
                && !exclude.matches(path, false)
        };
        let selected: Vec<_> = match literals {
            Some(paths) if paths.len() < inventory.len() => paths
                .iter()
                .map(AsRef::as_ref)
                .filter(|p| inventory.binary_search_by(|v| v.as_str().cmp(p)).is_ok())
                .filter(admits)
                .collect(),
            _ => inventory
                .iter()
                .map(String::as_str)
                .filter(admits)
                .collect(),
        };
        let id = self.intern_paths(&selected);
        self.selections.insert(key, id);
        self.paths[id].clone()
    }

    pub(super) fn query(&mut self, query: &str) -> Rc<str> {
        if let Some(found) = self.queries.get(query) {
            return found.clone();
        }
        let owned: Rc<str> = Rc::from(query);
        self.queries.insert(owned.clone());
        owned
    }

    pub(super) fn table(&mut self, t: sdk::HostRelation) -> Result<Hosted, String> {
        let columns = t
            .columns
            .into_iter()
            .map(|column| {
                let kind = match column.kind {
                    sdk::HostKind::File => "File",
                    sdk::HostKind::Text => "Text",
                    sdk::HostKind::Number => "Number",
                    _ => return Err("unsupported SDK host column type".to_owned()),
                };
                Ok((column.name.0.into(), kind.to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let schema = (String::from(t.name.0), columns);
        let next = self.schemas.len();
        let id = *self.schemas.entry(schema.clone()).or_insert(next);
        let mut row_ids = Vec::with_capacity(t.rows.len());
        for row in t.rows {
            let mut hash = DefaultHasher::new();
            row.0.hash(&mut hash);
            let bucket = self.by_hash.entry((id, hash.finish())).or_default();
            let existing = bucket.iter().copied().find(|&i| {
                let cells = &self.rows[i];
                cells.len() == row.0.len()
                    && cells.iter().zip(&row.0).all(|(a, b)| match (a, b) {
                        (HostCell::File { file }, sdk::Cell::File(p)) => {
                            file.as_str() == p.0.as_ref()
                        }
                        (HostCell::Text(text), sdk::Cell::Text(s)) => text.as_str() == s.0.as_ref(),
                        (HostCell::Number(n), sdk::Cell::Number(v)) => *n == v.0,
                        _ => false,
                    })
            });
            if let Some(i) = existing {
                row_ids.push(i);
                continue;
            }
            if row.0.len() != schema.1.len() {
                return Err("host row width disagrees with declaration".into());
            }
            let cells = row
                .0
                .into_iter()
                .zip(&schema.1)
                .map(|(v, (_, kind))| {
                    Ok(match (kind.as_str(), v) {
                        ("File", sdk::Cell::File(p)) => HostCell::File { file: p.0.into() },
                        ("Text", sdk::Cell::Text(s)) => HostCell::Text(s.0.into()),
                        ("Number", sdk::Cell::Number(n)) => HostCell::Number(n.0),
                        _ => return Err("host cell disagrees with declaration".to_owned()),
                    })
                })
                .collect::<Result<Rc<[_]>, _>>()?;
            let i = self.rows.len();
            self.rows.push(cells);
            bucket.push(i);
            row_ids.push(i);
        }
        let key = (id, row_ids);
        if let Some(table) = self.tables.get(&key) {
            return Ok(table.clone());
        }
        let rows = key.1.iter().map(|&i| self.rows[i].clone()).collect();
        let table = Hosted {
            name: schema.0,
            columns: schema.1,
            rows,
            partial: None,
        };
        self.tables.insert(key, table.clone());
        Ok(table)
    }
}
