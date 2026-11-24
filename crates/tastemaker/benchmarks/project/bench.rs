//! Replay complete SDK inputs, one JSON record per line, in one process.
//! Only the caller owns admission, supplied tables, parameters and the comparison oracle.
use super::{bindings, finish, lower};
use finish::{Plan, Provider, Run, Scope, Supplier, Table, Ty};
use ryo_values::domain::finish_plans::expressions::types::PlanConstant;
use serde::Deserialize;
use serde_json::json;
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
    #[serde(deserialize_with = "read_parameters")]
    parameters: Parameters,
}
type Parameters = BTreeMap<Box<str>, PlanConstant>;

fn read_parameters<'de, D: serde::Deserializer<'de>>(input: D) -> Result<Parameters, D::Error> {
    let fields = BTreeMap::<Box<str>, serde_json::Value>::deserialize(input)?;
    Ok(fields
        .into_iter()
        .map(|(k, v)| (k, bindings::constant(&v)))
        .collect())
}

#[derive(Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
enum HostCell {
    File { file: String },
    Number(i64),
    Text(String),
}

#[derive(Clone, Deserialize, PartialEq, Eq, Hash)]
struct Hosted {
    name: String,
    columns: Vec<(String, String)>,
    #[serde(deserialize_with = "read_host_rows")]
    rows: std::sync::Arc<[Vec<HostCell>]>,
    #[serde(default)]
    partial: Option<String>,
}

fn read_host_rows<'de, D: serde::Deserializer<'de>>(input: D) -> Result<std::sync::Arc<[Vec<HostCell>]>, D::Error> {
    Vec::deserialize(input).map(std::sync::Arc::from)
}

struct Snapshot<'a> {
    project: Option<&'a crate::check_session::project::Population>,
    has_inventory: bool,
    tables: BTreeMap<String, Table>,
}

impl Provider for Snapshot<'_> {
    fn shared_fill(&mut self, run:&mut Run, name:&str, scope:Scope<'_>) -> Result<std::rc::Rc<Table>,String> {
        if matches!(scope,Scope::Files(_)) && !self.tables.contains_key(name) && let Some(t)=self.project.and_then(|p|p.tables.get(name)) {
            return Ok(t.clone());
        }
        self.fill(run,name,scope).map(std::rc::Rc::new)
    }
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
    if run.paths.as_slice() != inventory {
        run.paths = inventory.to_vec().into();
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
    carried: &finish::population::Inputs,
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
            serde_json::from_str(&line.expect("read request")).expect("project request");
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
        let receipt = match result {
            Ok(rows) => {
                let rendered = batch::findings(rows.iter());
                let digest = Sha256::digest(format!("{}\n", rendered.join("\n")));
                json!({"id":id, "rows":rows.len(), "sha256":format!("{digest:x}")})
            }
            Err(reason) => json!({"id":id, "refused":reason}),
        };
        writeln!(out, "{receipt}").expect("write receipt");
    }
}

#[path = "batch.rs"]
mod batch;
pub(super) use batch::{batch, one, packets};

#[cfg(feature = "product-session")]
pub use batch::calls::{Live, LiveReport, capture, capture_one, print_measurement};
