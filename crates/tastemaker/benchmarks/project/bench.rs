//! Replay complete SDK inputs, one JSON record per line, in one process.
//! Only the caller owns admission, supplied tables, parameters and the comparison oracle.
use super::{finish, json_input, json_output, lower};
use json_input::{Project as Request, Hosted, HostCell, Parameters, read_parameters};
use finish::{Plan, Provider, Run, Scope, Supplier, Table, Ty};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead as _, Write as _};

include!("../../../ryo/crates/engine/benches/vec/check_session/hosted.rs");

fn path_id(run: &Run, path: &str) -> Result<u32, String> {
    let at = run
        .paths
        .binary_search_by(|p| p.as_str().cmp(path))
        .map_err(|_| format!("undeclared path {path}"))?;
    u32::try_from(at).map_err(|e| e.to_string())
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
        None,
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
pub use batch::calls::{link_executor, measure_dispatch, configure_bench_check, configured_workers, HostPhase, host_snapshot, host_wall, capture, capture_then, capture_retained, capture_one, print_measurement};
