//! Immutable inputs shared by SDK calls. File selection depends on paths and glob sets;
//! authority and file contents still accompany each registration.
use super::{HostCell, Hosted, sdk};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

type Selection = (usize, usize, usize, usize);

#[derive(Default)]
pub(super) struct CallCache {
    paths: HashMap<Rc<Vec<String>>, usize>,
    selections: HashMap<Selection, Rc<Vec<String>>>,
    queries: HashSet<Rc<str>>,
    schemas: HashMap<(String, Vec<(String, String)>), usize>,
    rows: Vec<Rc<[HostCell]>>,
    by_hash: HashMap<(usize, u64), Vec<usize>>,
    tables: HashMap<(usize, Vec<usize>), Hosted>,
}

impl CallCache {
    pub(super) fn inventory(&mut self, paths: Vec<String>) -> (usize, Rc<Vec<String>>) {
        if let Some((found, id)) = self.paths.get_key_value(&paths) {
            return (*id, found.clone());
        }
        let paths = Rc::new(paths);
        let id = self.paths.len();
        self.paths.insert(paths.clone(), id);
        (id, paths)
    }

    pub(super) fn files(
        &mut self,
        key: Selection,
        select: impl FnOnce() -> Vec<String>,
    ) -> Vec<String> {
        let paths = self
            .selections
            .entry(key)
            .or_insert_with(|| Rc::new(select()));
        paths.as_ref().clone()
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
