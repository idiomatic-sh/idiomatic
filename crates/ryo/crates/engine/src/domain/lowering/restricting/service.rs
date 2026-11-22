//! SPIKE (restricted tables): a table every reader of which reaches by one key, from keys the reader holds before it
//! asks, is made only for those keys. The keys are a definition of their own (each reader's atoms before its ask, its
//! row the key cells), and the table's pipelines are its definition's branches each joined with that definition on the
//! key cells: the planner may start from the keys (a closure seeded by them). Readers read the table as before. Decided
//! from the plan alone: who reads the table, by which columns, and where their keys come from.

use crate::domain::lowered::types::{Emptiness, MadeFill, PatternForm};
use crate::domain::lowering::costing::constants::ONE;
use crate::domain::lowering::costing::service::{definition_rows, planned_rows, plus, times};
use crate::domain::lowering::costing::types::PlanEstimate;
use crate::domain::lowering::definitions::service::defined;
use crate::domain::lowering::definitions::types::{AskSite, Drafts, Flattened};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, RowsPerFile,
};
use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::placement::types::LoadSample;
use crate::domain::lowering::planning::service::planned;
use crate::domain::lowering::planning::types::UnitRole;
use crate::domain::lowering::planning::writing::service::pipeline;
use crate::domain::steps::types::{ColumnMask, DefinitionId, MadeTableId};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use std::collections::HashSet;

/// A table restricted by this pass: what to put back if a reader asks it later from where its keys are not known.
#[derive(Debug)]
pub(crate) struct Restricted {
    /// The definition.
    definition: DefinitionId,
    /// Its table.
    table: MadeTableId,
    /// How many sites its keys were taken from.
    sites: usize,
    /// The fill it had.
    original: MadeFill,
}

/// The margin the readers' keys must stay below the table's rows by: `RYO_L1B_RESTRICT` (none or nought: off).
#[must_use]
pub(crate) fn restrict_margin() -> u64 {
    static MARGIN: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *MARGIN.get_or_init(|| {
        std::env::var("RYO_L1B_RESTRICT")
            .ok()
            .and_then(|margin| margin.parse().ok())
            .unwrap_or(0)
    })
}

/// Whether an atom reads only relations and definitions numbered below `below`: a prefix holding one is no cycle.
fn before(atom: &FlatAtom, below: DefinitionId) -> bool {
    let read = |rows: &AtomRows| match rows {
        AtomRows::Relation(_) => true,
        AtomRows::Definition(definition) => definition.0 < below.0,
        AtomRows::Made(_) => false,
    };
    match atom {
        FlatAtom::Rows { rows, .. } | FlatAtom::Decided { rows, .. } => read(rows),
        FlatAtom::Test { .. } | FlatAtom::Called { .. } | FlatAtom::Walk { .. } => true,
        FlatAtom::Latest { .. } | FlatAtom::Nearest { .. } | FlatAtom::Closure { .. } => false,
    }
}

/// The key cells an ask reads the definition by, in the mask's order: none when the asking atom does not read this
/// definition by those cells.
fn key_cells(
    site: &AskSite,
    definition: DefinitionId,
    bound: &ColumnMask,
) -> Option<Vec<FlatTerm>> {
    let cell = |column: usize| -> Option<FlatTerm> {
        match &site.atom {
            FlatAtom::Rows {
                rows: AtomRows::Definition(read),
                cells,
                ..
            } if *read == definition => cells.get(column).copied(),
            FlatAtom::Decided {
                rows: AtomRows::Definition(read),
                cells,
                ..
            } if *read == definition => cells.get(column).copied().flatten(),
            FlatAtom::Rows { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Latest { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. } => None,
        }
    };
    bound
        .0
        .iter()
        .map(|column| cell(usize::from(column.0)))
        .collect()
}

/// The table-form patterns of a definition: their one bound mask and every site they were asked from. None when the
/// table is read by no pattern, by two masks, whole, or from a place whose keys are not known before the table.
fn readers(drafts: &Drafts, definition: DefinitionId) -> Option<(ColumnMask, Vec<AskSite>)> {
    let draft = drafts
        .definitions
        .get(usize::try_from(definition.0).ok()?)?;
    let mut mask: Option<ColumnMask> = None;
    let mut sites = Vec::new();
    for pattern in &draft.patterns {
        if !matches!(pattern.form, Some(PatternForm::Table(_))) {
            continue;
        }
        if pattern.opaque || pattern.asked.bound.0.is_empty() {
            return None;
        }
        match &mask {
            Some(held) if *held != pattern.asked.bound => return None,
            Some(_) => {}
            None => mask = Some(pattern.asked.bound.clone()),
        }
        sites.extend(pattern.sites.iter().cloned());
    }
    Some((mask?, sites)).filter(|(_, sites)| !sites.is_empty())
}

/// How many sites the table-form patterns of a definition were asked from, and whether any ask had none.
fn sites_now(drafts: &Drafts, definition: DefinitionId) -> (usize, bool) {
    let Some(draft) = usize::try_from(definition.0)
        .ok()
        .and_then(|place| drafts.definitions.get(place))
    else {
        return (0, true);
    };
    let tables = draft
        .patterns
        .iter()
        .filter(|pattern| matches!(pattern.form, Some(PatternForm::Table(_))));
    tables.fold((0, false), |(count, opaque), pattern| {
        (count + pattern.sites.len(), opaque || pattern.opaque)
    })
}

/// A node no plan names, for the definition of a table's asked keys.
fn keys_node(node: NodeDigest) -> NodeDigest {
    let mut bytes = node.0;
    for (place, byte) in bytes.iter_mut().enumerate() {
        *byte ^= 0xA5_u8.rotate_left(u32::try_from(place % 8).unwrap_or(0));
    }
    NodeDigest(bytes)
}

/// Restricts each table this pass may: its keys' definition made, its pipelines written again from its definition's
/// branches joined with the keys. Answers what it restricted, to be checked after the patterns it asked are placed.
pub(crate) fn restricted(
    drafts: &mut Drafts,
    sample: LoadSample<'_>,
    margin: u64,
) -> Vec<Restricted> {
    let mut done = Vec::new();
    let count = drafts.definitions.len();
    for place in 0..count {
        let definition = DefinitionId(u32::try_from(place).unwrap_or(u32::MAX));
        let Some(table) = drafts.definitions.get(place).and_then(|draft| draft.table) else {
            continue;
        };
        let is_rows = drafts
            .tables
            .get(usize::try_from(table.0).unwrap_or(usize::MAX))
            .is_some_and(|made| matches!(made.fill, MadeFill::Rows(..)));
        if !is_rows {
            continue;
        }
        let Some((mask, sites)) = readers(drafts, definition) else {
            continue;
        };
        if !sites
            .iter()
            .all(|site| site.prefix.iter().all(|atom| before(atom, definition)))
        {
            continue;
        }
        let keys: Option<Vec<Vec<FlatTerm>>> = sites
            .iter()
            .map(|site| key_cells(site, definition, &mask))
            .collect();
        let Some(keys) = keys else {
            continue;
        };
        let asked = sites
            .iter()
            .fold(PlanEstimate(0), |sum, site| plus(sum, site.rows));
        let whole = planned_rows(definition_rows(drafts, definition));
        let margin = PlanEstimate(margin.saturating_mul(ONE.0));
        if times(asked, margin) >= whole {
            continue;
        }
        let Some(draft) = drafts.definitions.get(place) else {
            continue;
        };
        let (node, home, shape) = (draft.node, draft.home, draft.shape.clone());
        let own: Vec<FlatBranch> = draft.branches.clone();
        let key_shape = StepShape(
            mask.0
                .iter()
                .filter_map(|column| shape.0.get(usize::from(column.0)).copied())
                .collect(),
        );
        let branches: Vec<FlatBranch> = sites
            .iter()
            .zip(keys)
            .map(|(site, row)| FlatBranch {
                atoms: site.prefix.clone(),
                row,
            })
            .collect();
        let keyed = defined(
            drafts,
            keys_node(node),
            home,
            key_shape,
            Flattened {
                branches,
                emptiness: Emptiness::Unknown,
                keyless: Vec::new(),
            },
        );
        let mut pipelines = Vec::with_capacity(own.len());
        for branch in &own {
            let cells: Option<Vec<FlatTerm>> = mask
                .0
                .iter()
                .map(|column| branch.row.get(usize::from(column.0)).copied())
                .collect();
            let Some(cells) = cells else {
                break;
            };
            let mut joined = branch.clone();
            joined.atoms.push(FlatAtom::Rows {
                rows: AtomRows::Definition(keyed),
                cells,
                per_file: RowsPerFile::Many,
            });
            let written = planned(drafts, &joined, &HashSet::new(), UnitRole::Pipeline)
                .and_then(|plan| pipeline(drafts, sample.kinds, &joined, &plan, Some(definition)));
            match written {
                Some(written) => pipelines.push(written),
                None => break,
            }
        }
        if pipelines.len() != own.len() {
            continue;
        }
        let Some(made) = drafts
            .tables
            .get_mut(usize::try_from(table.0).unwrap_or(usize::MAX))
        else {
            continue;
        };
        let original = std::mem::replace(
            &mut made.fill,
            MadeFill::Rows(pipelines.into(), RepeatedRows::Dropped),
        );
        if std::env::var_os("RYO_L1B_RESTRICT_TRACE").is_some() {
            eprintln!(
                "ryo l1b: restricted definition={} table={} sites={} asked={} whole={} keys=definition {}",
                definition.0,
                table.0,
                sites.len(),
                asked.0,
                whole.0,
                keyed.0
            );
        }
        done.push(Restricted {
            definition,
            table,
            sites: sites.len(),
            original,
        });
    }
    done
}

/// After the patterns the restricted pipelines asked are placed: a table some reader now asks from a place its keys did
/// not come from gets its own fill back. Answers how many stay restricted.
pub(crate) fn checked(drafts: &mut Drafts, done: Vec<Restricted>) -> usize {
    let mut kept = 0;
    for each in done {
        let (sites, opaque) = sites_now(drafts, each.definition);
        if sites == each.sites && !opaque {
            kept += 1;
            continue;
        }
        if let Some(made) = drafts
            .tables
            .get_mut(usize::try_from(each.table.0).unwrap_or(usize::MAX))
        {
            made.fill = each.original;
        }
        if std::env::var_os("RYO_L1B_RESTRICT_TRACE").is_some() {
            eprintln!("ryo l1b: unrestricted definition={}", each.definition.0);
        }
    }
    if std::env::var_os("RYO_L1B_RESTRICT_TRACE").is_some() {
        eprintln!("ryo l1b: restricted tables kept {kept}");
    }
    kept
}
