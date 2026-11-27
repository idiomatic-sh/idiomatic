//! The complete-project tables a hosted request supplies, and the provider that answers a query's reads of them.
use crate::{finish, lower};
pub use crate::request_input::{Project as Request, Hosted, HostCell, Parameters, read_parameters};
use finish::{Plan, Provider, Run, Scope, Supplier, Table, Ty};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead as _, Write as _};

pub struct Snapshot<'a> {
    pub project: Option<&'a crate::check_session::project::Population>,
    pub has_inventory: bool,
    /// The listed files the host found not there: they have no `filesystem.file` row.
    pub missing: BTreeSet<u32>,
    pub tables: BTreeMap<String, Table>,
}

impl Provider for Snapshot<'_> {
    fn shared_fill(&mut self, run:&mut Run, name:&str, scope:Scope<'_>) -> Result<std::rc::Rc<Table>,crate::check_session::refusal::Refusal> {
        if matches!(scope,Scope::Files(_)) && !self.tables.contains_key(name) && let Some(t)=self.project.and_then(|p|p.tables.get(name)) {
            return Ok(t.clone());
        }
        self.fill(run,name,scope).map(std::rc::Rc::new)
    }
    fn fill(&mut self, run: &mut Run, name: &str, scope: Scope<'_>) -> Result<Table, crate::check_session::refusal::Refusal> {
        let Scope::Files(files) = scope else {
            return Err(format!("local ask in closed session for {name}").into());
        };
        if let Some(t) = self.tables.remove(name) {
            return Ok(t);
        }
        if let Some(t) = self.project.and_then(|p| p.tables.get(name)) {
            return Ok(t.copy());
        }
        if name != "filesystem.file" {
            return Err(format!("missing complete project population for {name}").into());
        }
        // The release can withhold a named producer when all of its source
        // partitions were skipped. That state cannot prove a negative lookup.
        if files.is_empty() && self.has_inventory {
            return Err("all file partitions excluded; no complete inventory contribution".into());
        }
        let (_, names, tys) = run.catalog(name).ok_or("no file inventory declaration")?;
        // The host wrote the relation only for a file it observed: a listed file that is not there has no row (its relation is noted
        // read only in part), and a file that is there and cannot be read keeps its row.
        eprintln!("DBG4 fill files={files:?} missing={:?}", self.missing);
        let present: Vec<u32> = files.iter().copied().filter(|file| !self.missing.contains(file)).collect();
        let mut t = Table::with_rows(names, tys, present.len());
        t.cols[0].extend(present.iter().copied().map(u64::from));
        t.n = present.len();
        Ok(t)
    }

    fn render(&mut self, _: u32, _: &str) -> Result<Option<Vec<u8>>, String> {
        Err("project rendering requires source origins".into())
    }
}

pub fn bind_tables(
    hosted: &[Hosted],
    run: &mut Run,
    file_id: impl Fn(&str) -> Option<u32>,
) -> Result<BTreeMap<String, Table>, String> {
    let mut tables = BTreeMap::new();
    for table in hosted {
        if tables.contains_key(&table.name) {
            return Err(format!("duplicate host relation {}", table.name));
        }
        if let Some(reason) = &table.partial {
            return Err(format!("partial {}: {reason}", table.name));
        }
        if ryo_relations::domain::relations::service::relation_named(
            ryo_relations::domain::labels::types::NameRef(&table.name),
        )
        .is_some()
        {
            return Err(format!("relation already declared: {}", table.name));
        }
        let mut columns = Vec::new();
        for (name, kind) in &table.columns {
            let ty = match kind.as_str() {
                "File" => Ty::File,
                "Text" => Ty::Sym,
                "Number" => Ty::Num,
                _ => return Err(format!("unsupported host type {kind}")),
            };
            if columns.iter().any(|(n, _)| n == &name.as_str()) {
                return Err(format!("duplicate column {name}"));
            }
            columns.push((name.as_str(), ty));
        }
        run.declare(&table.name, &columns, Supplier::Host);
        let (_, names, tys) = run.catalog(&table.name).ok_or("missing host declaration")?;
        let mut t = Table::with_rows(names, tys, table.rows.len());
        let mut cells = Vec::with_capacity(columns.len());
        'rows: for row in table.rows.iter() {
            if row.len() != columns.len() {
                return Err(format!("host row width in {}", table.name));
            }
            cells.clear();
            for (cell, (_, ty)) in row.iter().zip(&columns) {
                let word = match ty {
                    Ty::File => {
                        let HostCell::File { file: path } = cell else {
                            return Err("expected file cell".into());
                        };
                        // table_of_query omits a row with no such file in the numbered request.
                        let Some(id) = file_id(path) else {
                            continue 'rows;
                        };
                        u64::from(id)
                    }
                    Ty::Sym => {
                        let HostCell::Text(text) = cell else {
                            return Err("expected text cell".into());
                        };
                        u64::from(run.sym(text.as_bytes()))
                    }
                    Ty::Num => {
                        let HostCell::Number(n) = cell else {
                            return Err("expected number cell".into());
                        };
                        *n as u64
                    }
                    _ => unreachable!(),
                };
                cells.push(word);
            }
            for (column, word) in t.cols.iter_mut().zip(&cells) {
                column.push(*word);
            }
            t.n += 1;
        }
        let _previous = tables.insert(table.name.clone(), t);
    }
    Ok(tables)
}

pub fn execute_ids(
    exec: &mut finish::Exec<'_>,
    run: &mut Run,
    numbers: &[u32],
    carried: &finish::population::Inputs,
    inventory: &[String],
    files: &[String],
    missing: &BTreeSet<String>,
    hosted: &[Hosted],
    project: Option<&crate::check_session::project::Population>,
    incomplete: &BTreeMap<String, Vec<Option<u32>>>,
    memo: Option<(&mut finish::session::reuse::Reuse, [usize; 3])>,
) -> Result<Vec<finish::answer::Row>, crate::check_session::refusal::Refusal> {
    let number = |p: &str| {
        inventory
            .binary_search_by(|s| s.as_str().cmp(p))
            .ok()
            .map(|i| numbers[i])
    };
    let admitted: Vec<_> = files
        .iter()
        .map(|p| number(p).ok_or_else(|| format!("undeclared path {p}")))
        .collect::<Result<_, _>>()?;
    let subject_files: BTreeSet<_> = admitted.iter().copied().collect();
    if subject_files.len() != admitted.len() {
        return Err("duplicate admitted path".into());
    }
    let binding = finish::operators::enter(usize::MAX - 13, 0);
    let tables = bind_tables(hosted, run, number);
    if let Some(start) = binding {
        finish::operators::leave(start, "ids-bind-tables", 0, true);
    }
    let tables = tables?;
    let mut snapshot = Snapshot {
        tables,
        project,
        has_inventory: !inventory.is_empty(),
        missing: files.iter().filter(|path| missing.contains(*path)).filter_map(|path| number(path)).collect(),
    };
    finish::session::evaluate_covered(
        exec,
        run,
        &mut snapshot,
        &admitted,
        |file| subject_files.contains(&file),
        carried,
        incomplete,
        memo,
    )
}
