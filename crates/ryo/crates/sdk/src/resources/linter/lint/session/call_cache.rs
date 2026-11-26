//! Immutable inputs shared by SDK calls. File selection depends on paths and glob sets;
//! authority and file contents still accompany each registration.
use ryo_executor::check_session::hosted::{HostCell, Hosted, Parameters};
use ryo_executor::check_session::replay::stored;
use crate::resources::linter as sdk;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

type Selection = (usize, usize, usize, usize);

pub(super) enum PathFilter {
    Literals(Vec<Box<str>>),
    Patterns { set: GlobSet, prefixes: Vec<Box<str>> },
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
        let set = builder.build().map_err(|e| e.to_string())?;
        let prefixes = path_prefixes(names).unwrap_or_default();
        Ok(Self::Patterns { set, prefixes })
    }

    fn candidates(&self) -> Option<&[Box<str>]> {
        match self {
            Self::Literals(paths) if !paths.is_empty() => Some(paths),
            _ => None,
        }
    }

    fn prefix_ranges(&self, inventory: &[String]) -> Option<Vec<std::ops::Range<usize>>> {
        let Self::Patterns { prefixes, .. } = self else {
            return None;
        };
        // On a small inventory the binary searches can exceed the cost of a scan.
        if prefixes.is_empty()
            || Self::range_search_cost(prefixes.len(), inventory.len()) >= inventory.len()
        {
            return None;
        }
        Some(prefixes.iter().map(|prefix| {
            let start = inventory.partition_point(|path| path.as_str() < prefix.as_ref());
            let end = start + inventory[start..].partition_point(|path| path.starts_with(prefix.as_ref()));
            start..end
        }).collect())
    }

    fn range_search_cost(ranges: usize, paths: usize) -> usize {
        // Each prefix has two binary searches; budget both in path comparisons.
        ranges.saturating_mul(2 * (paths.max(1).ilog2() as usize + 1))
    }

    fn matches(&self, path: &str, empty_matches: bool) -> bool {
        match self {
            Self::Literals(paths) if paths.is_empty() => empty_matches,
            Self::Literals(paths) => paths.binary_search_by(|p| p.as_ref().cmp(path)).is_ok(),
            Self::Patterns { set, .. } => (empty_matches && set.is_empty()) || set.is_match(path),
        }
    }
}

/// A prefix bounds the candidate range. The original matcher still decides every
/// candidate, including directory recursion and negated character classes.
fn path_prefixes(names: &[sdk::Glob]) -> Option<Vec<Box<str>>> {
    if !cfg!(unix) {
        return None;
    }
    let mut prefixes = Vec::<Box<str>>::with_capacity(names.len());
    for name in names {
        let pattern = name.as_ref();
        if pattern.contains(['{', '}', '\\']) {
            return None;
        }
        let prefix = pattern.split(['*', '?', '[']).next().unwrap();
        // A recursive suffix can also recognize the directory without its slash.
        let prefix = prefix.trim_end_matches('/');
        if prefix.is_empty() {
            return None;
        }
        prefixes.push(prefix.into());
    }
    prefixes.sort_unstable();
    let mut distinct: Vec<Box<str>> = Vec::new();
    for prefix in prefixes {
        if !distinct.last().is_some_and(|prior| prefix.starts_with(prior.as_ref())) {
            distinct.push(prefix);
        }
    }
    Some(distinct)
}

#[derive(Default)]
pub(super) struct CallCache {
    paths: Vec<Rc<Vec<String>>>,
    path_hashes: HashMap<u64, Vec<usize>>,
    selections: HashMap<Selection, usize>,
    queries: HashMap<stored::Key, Rc<str>>,
    // A caller repeats one allocation per rule: its digest is known by address, and a hit is confirmed by content.
    query_digests: rustc_hash::FxHashMap<(usize, usize), (Rc<str>, stored::Key)>,
    catalogs: HashMap<usize, (super::modules::ModuleSources, ryo_executor::check_session::query::Modules)>,
    schemas: HashMap<(String, Vec<(String, String)>), usize>,
    rows: Vec<std::sync::Arc<[HostCell]>>,
    by_hash: HashMap<(usize, u64), Vec<usize>>,
    tables: HashMap<(usize, Vec<usize>), (usize, Hosted, std::cell::OnceCell<stored::Key>)>,
    path_digests: HashMap<*const Vec<String>, stored::Key>,
    pub(super) pattern_visits: [usize; 4],
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
    ) -> (usize, Rc<Vec<String>>) {
        if let Some(&id) = self.selections.get(&key) {
            return (id, self.paths[id].clone());
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
            _ => {
                let candidates = [request.prefix_ranges(&inventory), include.prefix_ranges(&inventory)];
                self.pattern_visits[3] += 2 * candidates.iter().flatten().map(Vec::len).sum::<usize>();
                let ranges = candidates.into_iter()
                    .flatten()
                    .min_by_key(|ranges| ranges.iter().map(|range| range.len()).sum::<usize>());
                self.pattern_visits[0] += inventory.len();
                if let Some(ranges) = ranges.filter(|ranges| {
                    ranges.iter().map(|range| range.len()).sum::<usize>()
                        + PathFilter::range_search_cost(ranges.len(), inventory.len())
                        < inventory.len()
                }) {
                    let count = ranges.iter().map(|range| range.len()).sum::<usize>();
                    self.pattern_visits[1] += count;
                    self.pattern_visits[2] += 1;
                    ranges.into_iter().flat_map(|range| &inventory[range])
                        .map(String::as_str).filter(admits).collect()
                } else {
                    self.pattern_visits[1] += inventory.len();
                    inventory.iter().map(String::as_str).filter(admits).collect()
                }
            }
        };
        let id = self.intern_paths(&selected);
        self.selections.insert(key, id);
        (id, self.paths[id].clone())
    }

    pub(super) fn query(&mut self, query: &str) -> (Rc<str>, stored::Key) {
        let at = (query.as_ptr() as usize, query.len());
        if let Some((text, digest)) = self.query_digests.get(&at)
            && &**text == query
        {
            return (text.clone(), *digest);
        }
        let digest = *blake3::hash(query.as_bytes()).as_bytes();
        let text = self.queries.entry(digest).or_insert_with(|| Rc::from(query)).clone();
        self.query_digests.insert(at, (text.clone(), digest));
        (text, digest)
    }

    pub(super) fn modules(&mut self, query: &crate::resources::query::RyoQlQuery) -> Option<ryo_executor::check_session::query::Modules> {
        let registry = super::modules::query_modules(query)?;
        let (_, modules) = self.catalogs.entry(registry.identity()).or_insert_with(|| {
            let sources = registry.sources().map(|(name, text)| (name.into(), text.clone())).collect();
            (registry, ryo_executor::check_session::query::Modules::new(sources))
        });
        Some(modules.clone())
    }

    pub(super) fn path_digest(&mut self, paths: &Rc<Vec<String>>) -> stored::Key {
        // Each list remains interned in self.paths for this cache's lifetime.
        *self
            .path_digests
            .entry(Rc::as_ptr(paths))
            .or_insert_with(|| stored::key(paths))
    }

    pub(super) fn table(
        &mut self,
        t: sdk::HostRelation,
        digests: Option<&mut Vec<stored::Key>>,
    ) -> Result<(usize, Hosted), sdk::LintError> {
        let columns = t
            .columns
            .iter()
            .map(|column| {
                let kind = match column.kind {
                    sdk::HostKind::File => "File",
                    sdk::HostKind::Text => "Text",
                    sdk::HostKind::Number => "Number",
                    _ => return Err(sdk::LintError::Execution { reason: "unsupported SDK host column type".into() }),
                };
                Ok((column.name.0.to_string(), kind.to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let schema = (t.name.0.to_string(), columns);
        let next = self.schemas.len();
        let id = *self.schemas.entry(schema.clone()).or_insert(next);
        let mut row_ids = Vec::with_capacity(t.rows.len());
        for (at, row) in t.rows.into_iter().enumerate() {
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
                return Err(sdk::LintError::HostRowWidth {
                    relation: t.name,
                    row: sdk::RowNumber(at + 1),
                    expected: schema.1.len(),
                    given: row.0.len(),
                });
            }
            let cells = row
                .0
                .into_iter()
                .zip(&t.columns)
                .map(|(v, column)| {
                    Ok(match (column.kind, v) {
                        (sdk::HostKind::File, sdk::Cell::File(p)) => HostCell::File { file: p.0.into() },
                        (sdk::HostKind::Text, sdk::Cell::Text(s)) => HostCell::Text(s.0.into()),
                        (sdk::HostKind::Number, sdk::Cell::Number(n)) => HostCell::Number(n.0),
                        _ => return Err(sdk::LintError::HostCellKind {
                            relation: t.name.clone(),
                            row: sdk::RowNumber(at + 1),
                            column: column.name.clone(),
                            expected: column.kind,
                        }),
                    })
                })
                .collect::<Result<std::sync::Arc<[_]>, _>>()?;
            let i = self.rows.len();
            self.rows.push(cells);
            bucket.push(i);
            row_ids.push(i);
        }
        let key = (id, row_ids);
        let next = self.tables.len();
        let (id, table, digest) = self.tables.entry(key).or_insert_with_key(|key| {
            let rows = key.1.iter().map(|&i| self.rows[i].clone()).collect();
            (
                next,
                Hosted {
                    name: schema.0,
                    columns: schema.1,
                    rows,
                    partial: None,
                },
                Default::default(),
            )
        });
        if let Some(digests) = digests {
            digests.push(*digest.get_or_init(|| stored::key(&*table)));
        }
        Ok((*id, table.clone()))
    }
}
