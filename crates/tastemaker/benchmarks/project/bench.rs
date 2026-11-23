//! Replay complete SDK inputs, one JSON record per line, in one process.
//! Only the caller owns admission, supplied tables, parameters and the comparison oracle.
use super::{
    finish,
    json_input::{self, Project, ProjectCell},
    json_output, lower,
};
use finish::{Plan, Provider, Run, Scope, Supplier, Table, Ty};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead as _, Write as _};

struct Snapshot {
    tables: BTreeMap<String, Table>,
}

impl Provider for Snapshot {
    fn fill(&mut self, run: &mut Run, name: &str, scope: Scope<'_>) -> Result<Table, String> {
        let Scope::Files(files) = scope else {
            return Err(format!("local ask in closed session for {name}"));
        };
        if let Some(t) = self.tables.remove(name) {
            return Ok(t);
        }
        if name != "filesystem.file" {
            return Err(format!("missing complete project population for {name}"));
        }
        // The release can withhold a named producer when all of its source
        // partitions were skipped. That state cannot prove a negative lookup.
        if files.is_empty() && !run.paths.is_empty() {
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

fn execute(req: Project, plan: &Plan) -> Result<finish::json_output::Records, String> {
    let mut run = Run::default();
    run.empty = run.sym(b"");
    // The release binds host File cells in the whole request's numbered paths.
    // Each rule admits a subset and can emit a subject owned by another file.
    let mut inventory = req.inventory.ok_or("incomplete project inventory")?;
    inventory.sort_unstable();
    inventory.dedup();
    if req.files.iter().collect::<BTreeSet<_>>().len() != req.files.len() {
        return Err("duplicate admitted path".into());
    }
    run.path_sym = inventory
        .iter()
        .map(|path| run.sym(path.as_bytes()))
        .collect();
    run.paths = inventory;
    let admitted = req
        .files
        .iter()
        .map(|path| path_id(&run, path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut snapshot = Snapshot {
        tables: BTreeMap::new(),
    };
    for table in req.hosted {
        if let Some(reason) = table.partial {
            return Err(format!("partial {}: {reason}", table.name));
        }
        if run.catalog(&table.name).is_some() {
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
        'rows: for row in table.rows {
            if row.len() != columns.len() {
                return Err(format!("host row width in {}", table.name));
            }
            cells.clear();
            for (cell, (_, ty)) in row.into_iter().zip(&columns) {
                let word = match (ty, cell) {
                    (Ty::File, ProjectCell::File { file: path }) => {
                        // table_of_query omits a row with no such file in the numbered request.
                        let Ok(id) = run.paths.binary_search_by(|p| p.as_str().cmp(&path)) else {
                            continue 'rows;
                        };
                        u64::try_from(id).map_err(|e| e.to_string())?
                    }
                    (Ty::Sym, ProjectCell::Text(text)) => u64::from(run.sym(text.as_bytes())),
                    (Ty::Num, ProjectCell::Number(n)) if n != -1 => n as u64,
                    (Ty::File, _) => return Err("expected file cell".into()),
                    (Ty::Sym, _) => return Err("expected text cell".into()),
                    (Ty::Num, _) => return Err("number outside representable project range".into()),
                    _ => unreachable!(),
                };
                cells.push(word);
            }
            for (column, word) in t.cols.iter_mut().zip(&cells) {
                column.push(*word);
            }
            t.n += 1;
        }
        let _previous = snapshot.tables.insert(table.name, t);
    }
    let subject_files: BTreeSet<_> = admitted.iter().copied().collect();
    finish::session::evaluate(plan, &mut run, &mut snapshot, &admitted, |file| {
        subject_files.contains(&file)
    })
}

pub(super) fn replay() {
    let mut plans = std::collections::HashMap::new();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let req = json_input::project(&line.expect("read request"));
        let id = req.id.clone();
        let key = (req.query.clone(), req.parameters.clone());
        let plan = plans.entry(key).or_insert_with(|| {
            let mut ctx = ryo_engine::domain::plan_lowering::types::RuleContext::default();
            let _previous = ctx.parameters.insert(
                "query".into(),
                req.parameters
                    .iter()
                    .map(|(k, v)| (k.as_str().into(), v.clone()))
                    .collect(),
            );
            lower::query_plan("query", &req.query, ctx)
        });
        let result = match plan {
            Ok(plan) => execute(req, plan),
            Err(reason) => Err(reason.clone()),
        };
        let receipt = json_output::receipt(&id, result);
        writeln!(out, "{receipt}").expect("write receipt");
    }
}
