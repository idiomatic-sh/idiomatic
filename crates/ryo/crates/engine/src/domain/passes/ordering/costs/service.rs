//! The service of the `costs` part: what of each step the order is decided by beside its registers, by a `match` that
//! names every step; and what a sample says one use of each definition and one read of each made table cost. A use's cost
//! is its unit's counted work over the uses that reached it: a table's fill is paid by its readers, so a step that reads
//! a table is not taken for free and moved before the test that would have spared the file its fill.

use crate::domain::code::types::UnitKey;
use crate::domain::lowered::service::fill_code;
use crate::domain::lowered::types::LoweredPack;
use crate::domain::passes::ordering::types::{RunWork, StepFacts, StepPlacement, UseCosts};
use crate::domain::statistics::learning::service::{rows_code, unit_work};
use crate::domain::statistics::service::unit_of;
use crate::domain::statistics::types::{MeasuredUnit, Statistics};
use crate::domain::steps::types::{MadeTableId, ProgramStep, StepRows};
use ryo_relations::domain::relations::types::Count;

/// The made table a source's rows are read from, when it is one made in the file.
fn source_tables(source: StepRows) -> Vec<MadeTableId> {
    match source {
        StepRows::Made(table) => vec![table],
        StepRows::Node(_)
        | StepRows::Filled(_)
        | StepRows::Written(_)
        | StepRows::Definition(_) => Vec::new(),
    }
}

/// Whether a step may be moved within its level, and the made tables it reads rows of. A `Read` takes a column of a row
/// it was handed, so its table is made already and it reads none.
#[must_use]
pub(crate) fn step_facts(step: &ProgramStep) -> StepFacts {
    let free = |tables: Vec<MadeTableId>| StepFacts {
        placement: StepPlacement::Free,
        tables,
    };
    match step {
        ProgramStep::EachMatch { source, .. }
        | ProgramStep::EachRange { source, .. }
        | ProgramStep::EachRow { source, .. } => free(source_tables(*source)),
        ProgramStep::Latest { table, .. } => free(vec![*table]),
        ProgramStep::EachKind { .. }
        | ProgramStep::Children { .. }
        | ProgramStep::Axis { .. }
        | ProgramStep::Within { .. }
        | ProgramStep::Nearest { .. }
        | ProgramStep::Closure { .. }
        | ProgramStep::Read { .. }
        | ProgramStep::Test { .. }
        | ProgramStep::Call { .. }
        | ProgramStep::Render { .. }
        | ProgramStep::Let { .. }
        | ProgramStep::Exists { .. }
        | ProgramStep::Or { .. } => free(Vec::new()),
        ProgramStep::Fork { .. } | ProgramStep::Yield { .. } | ProgramStep::Emit { .. } => {
            StepFacts {
                placement: StepPlacement::Held,
                tables: Vec::new(),
            }
        }
    }
}

/// A count as the number the ranks are worked in: its high and low halves, each whole in a float.
#[must_use]
pub(crate) fn counted(count: Count) -> RunWork {
    let bits = u64::try_from(count.0).unwrap_or(u64::MAX);
    let [high, low] = [bits >> u32::BITS, bits & u64::from(u32::MAX)]
        .map(|half| u32::try_from(half).map_or(f64::from(u32::MAX), f64::from));
    RunWork(high * (f64::from(u32::MAX) + 1.0) + low)
}

/// The counted work of one use of what `units` are the forms of: their work together over the uses that reached them; all
/// of it when the sample counted no use.
fn for_each_use<'s>(units: impl Iterator<Item = &'s MeasuredUnit>) -> RunWork {
    let (mut work, mut uses) = (Count(0), Count(0));
    for unit in units {
        work = Count(work.0.saturating_add(unit_work(unit).0));
        uses = Count(uses.0.saturating_add(unit.counts.calls.0));
    }
    RunWork(counted(work).0 / counted(uses).0.max(1.0))
}

/// What the sample says a use costs: one use of each definition of the pack (its patterns' call forms and its own table,
/// whichever the sample ran, together), and one read of each made table.
#[must_use]
pub(crate) fn use_costs(pack: &LoweredPack, statistics: &Statistics) -> UseCosts {
    let definitions = pack.definitions.iter().map(|definition| {
        let patterns = definition.patterns.iter().map(|pattern| {
            UnitKey::Pattern(
                definition.node,
                pattern.bound.clone(),
                pattern.collected.clone(),
            )
        });
        let keys = patterns.chain([UnitKey::Table(definition.node, rows_code())]);
        let units: Vec<&MeasuredUnit> = keys.filter_map(|key| unit_of(statistics, &key)).collect();
        for_each_use(units.into_iter())
    });
    let tables = pack.tables.iter().map(|table| {
        let key = UnitKey::Table(table.node, fill_code(&table.fill));
        for_each_use(unit_of(statistics, &key).into_iter())
    });
    UseCosts {
        definitions: definitions.collect(),
        tables: tables.collect(),
    }
}
