//! Replay complete SDK inputs, one JSON record per line, in one process.
//! Only the caller owns admission, supplied tables, parameters and the comparison oracle.
use super::{finish, json_input, json_output, lower};
use json_input::{Project as Request, Hosted, HostCell, Parameters, read_parameters};
use finish::{Plan, Provider, Run, Scope, Supplier, Table, Ty};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead as _, Write as _};

struct Snapshot<'a> {
    project: Option<&'a crate::check_session::project::Population>,
    has_inventory: bool,
    tables: BTreeMap<String, Table>,
}

impl Provider for Snapshot<'_> {
    fn fill(&mut self, run: &mut Run, name: &str, scope: Scope<'_>) -> Result<Table, String> {
        let Scope::Files(files) = scope else {
            return Err(format!("local ask in closed session for {name}"));
        };
        if let Some(t) = self.tables.remove(name) {
            return Ok(t);
        }
        if let Some(t) = self.project.and_then(|p| p.tables.get(name)) {
            return Ok(t.copy());
        }
        if name != "filesystem.file" {
            return Err(format!("missing complete project population for {name}"));
        }
        // The release can withhold a named producer when all of its source
        // partitions were skipped. That state cannot prove a negative lookup.
        if files.is_empty() && self.has_inventory {
            return Err("all file partitions excluded; no complete inventory contribution".into());
        }
        let (_, names, tys) = run.catalog(name).ok_or("no file inventory declaration")?;
        let mut t = Table::with_rows(names, tys, files.len());
        t.cols[0].extend(files.iter().copied().map(u64::from));
        t.n = files.len();
        Ok(t)
    }

    fn render(&mut self, _: u32, _: &str) -> Result<Option<Vec<u8>>, String> {
        Err("project rendering requires source origins".into())
    }
}

fn path_id(run: &Run, path: &str) -> Result<u32, String> {
    let at = run
        .paths
        .binary_search_by(|p| p.as_str().cmp(path))
        .map_err(|_| format!("undeclared path {path}"))?;
    u32::try_from(at).map_err(|e| e.to_string())
}

fn bind_tables(
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
        'rows: for row in &table.rows {
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
                        if *n == -1 {
                            return Err("number outside representable project range".into());
                        }
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

fn execute(req: Request, plan: &Plan) -> Result<Vec<finish::answer::Row>, String> {
    let mut run = Run::default();
    run.empty = run.sym(b"");
    let mut inventory = req.inventory.ok_or("incomplete project inventory")?;
    inventory.sort_unstable();
    inventory.dedup();
    execute_bound(
        &mut finish::Exec::new(plan),
        &mut run,
        &inventory,
        &req.files,
        &req.hosted,
    )
}

fn execute_bound(
    exec: &mut finish::Exec<'_>,
    run: &mut Run,
    inventory: &[String],
    files: &[String],
    hosted: &[Hosted],
) -> Result<Vec<finish::answer::Row>, String> {
    execute_populations(exec, run, inventory, files, hosted, None)
}

fn execute_populations(
    exec: &mut finish::Exec<'_>,
    run: &mut Run,
    inventory: &[String],
    files: &[String],
    hosted: &[Hosted],
    project: Option<&crate::check_session::project::Population>,
) -> Result<Vec<finish::answer::Row>, String> {
    // Host File cells are numbered by the full inventory. Admission restricts
    // source contributions and the final subjects, not the host's numbering.
    if run.paths != inventory {
        run.path_sym = inventory.iter().map(|p| run.sym(p.as_bytes())).collect();
        run.paths = inventory.to_vec();
    }
    let numbers: Vec<_> = (0..inventory.len() as u32).collect();
    execute_ids(
        exec,
        run,
        &numbers,
        &BTreeMap::new(),
        inventory,
        files,
        hosted,
        project,
        &BTreeMap::new(),
    )
}

fn execute_ids(
    exec: &mut finish::Exec<'_>,
    run: &mut Run,
    numbers: &[u32],
    carried: &BTreeMap<usize, std::rc::Rc<Table>>,
    inventory: &[String],
    files: &[String],
    hosted: &[Hosted],
    project: Option<&crate::check_session::project::Population>,
    incomplete: &BTreeMap<String, Vec<Option<u32>>>,
) -> Result<Vec<finish::answer::Row>, String> {
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
    let tables = bind_tables(hosted, run, number)?;
    let mut snapshot = Snapshot {
        tables,
        project,
        has_inventory: !inventory.is_empty(),
    };
    finish::session::evaluate_covered(
        exec,
        run,
        &mut snapshot,
        &admitted,
        |file| subject_files.contains(&file),
        carried,
        incomplete,
    )
}

pub(super) fn replay() {
    let mut plans = std::collections::HashMap::new();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let req: Request =
            json_input::project(&line.expect("read request"));
        let id = req.id.clone();
        let key = (req.query.clone(), req.parameters.clone());
        let plan = plans.entry(key).or_insert_with(|| {
            let mut ctx = ryo_engine::domain::plan_lowering::types::RuleContext::default();
            let _previous = ctx
                .parameters
                .insert("query".into(), req.parameters.clone());
            lower::query_plan("query", &req.query, ctx)
        });
        let result = match plan {
            Ok(plan) => execute(req, plan),
            Err(reason) => Err(reason.clone()),
        };
        let receipt = json_output::project_receipt(&id, result.map(|rows| {
            (rows.len(), batch::digest(&rows.iter().collect::<Vec<_>>()))
        }));
        writeln!(out, "{receipt}").expect("write receipt");
    }
}

#[path = "batch.rs"]
mod batch;
pub(super) use batch::{batch, one, packets};

#[cfg(feature = "product-session")]
pub use batch::calls::{Live, LiveReport, capture, capture_one, print_measurement};
