//! Replay complete SDK inputs, one JSON record per line, in one process.
//! Only the caller owns admission, supplied tables, parameters and the comparison oracle.
use super::{bindings, finish, lower};
use finish::{Plan, Provider, Run, Scope, Supplier, Table, Ty};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead as _, Write as _};

#[derive(Deserialize)]
struct Request {
    id: String,
    query: String,
    files: Vec<String>,
    inventory: Option<Vec<String>>,
    hosted: Vec<Hosted>,
    parameters: BTreeMap<String, Value>,
}
#[derive(Deserialize)]
struct Hosted {
    name: String,
    columns: Vec<(String, String)>,
    rows: Vec<Vec<Value>>,
    #[serde(default)]
    partial: Option<String>,
}

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

fn execute(req: Request, plan: &Plan) -> Result<Vec<String>, String> {
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
                let word = match ty {
                    Ty::File => {
                        let path = cell["file"].as_str().ok_or("expected file cell")?;
                        // table_of_query omits a row with no such file in the numbered request.
                        let Ok(id) = run.paths.binary_search_by(|p| p.as_str().cmp(path)) else {
                            continue 'rows;
                        };
                        u64::try_from(id).map_err(|e| e.to_string())?
                    }
                    Ty::Sym => {
                        u64::from(run.sym(cell.as_str().ok_or("expected text cell")?.as_bytes()))
                    }
                    Ty::Num => cell
                        .as_i64()
                        .filter(|n| *n != -1)
                        .ok_or("number outside representable project range")?
                        as u64,
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
    let mut plans = BTreeMap::new();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let req: Request =
            serde_json::from_str(&line.expect("read request")).expect("project request");
        let id = req.id.clone();
        let key = (
            req.query.clone(),
            serde_json::to_string(&req.parameters).expect("parameters"),
        );
        let plan = plans.entry(key).or_insert_with(|| {
            let mut ctx = ryo_engine::domain::plan_lowering::types::RuleContext::default();
            let _previous = ctx.parameters.insert(
                "query".into(),
                req.parameters
                    .iter()
                    .map(|(k, v)| (k.as_str().into(), bindings::constant(v)))
                    .collect(),
            );
            lower::query_plan("query", &req.query, ctx)
        });
        let result = match plan {
            Ok(plan) => execute(req, plan),
            Err(reason) => Err(reason.clone()),
        };
        let receipt = match result {
            Ok(rows) => {
                let digest = Sha256::digest(format!("{}\n", rows.join("\n")));
                json!({"id":id, "rows":rows.len(), "sha256":format!("{digest:x}")})
            }
            Err(reason) => json!({"id":id, "refused":reason}),
        };
        writeln!(out, "{receipt}").expect("write receipt");
    }
}
