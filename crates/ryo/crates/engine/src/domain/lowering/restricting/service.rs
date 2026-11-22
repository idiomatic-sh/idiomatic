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
use crate::domain::lowering::definitions::types::{AskSite, Drafts, Flattened, RowEstimate};
use crate::domain::lowering::flattening::service::variables;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, RowsPerFile,
};
use crate::domain::lowering::nodes::types::StepShape;
use crate::domain::lowering::placement::patterns::service::definition_table;
use crate::domain::lowering::placement::types::{LoadSample, RefusedPlace};
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
    /// The sites its keys were taken from, by number.
    sites: HashSet<u64>,
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

/// The numbers that follow `word(` in a text: the definitions or made tables a value names.
fn named(text: &str, word: &str) -> Vec<u32> {
    let opened = format!("{word}(");
    text.match_indices(&opened)
        .filter_map(|(at, _)| {
            let rest = text.get(at + opened.len()..)?;
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// Whether what a value reads, followed through the definitions and made tables it names, never leads to `table`'s
/// definition: a prefix holding it is no cycle. SPIKE: read off the debug form of the value, of each definition's branches
/// and of each made table's fill (every `DefinitionId(n)` and `MadeTableId(n)` named), so no kind of atom or stage is left
/// out.
fn before(drafts: &Drafts, text: &str, below: DefinitionId) -> bool {
    let mut definitions: Vec<u32> = named(text, "DefinitionId");
    let mut tables: Vec<u32> = named(text, "MadeTableId");
    let (mut seen_definitions, mut seen_tables) = (HashSet::new(), HashSet::new());
    loop {
        if let Some(definition) = definitions.pop() {
            if definition == below.0 {
                return false;
            }
            if !seen_definitions.insert(definition) {
                continue;
            }
            let Some(draft) = drafts
                .definitions
                .get(usize::try_from(definition).unwrap_or(usize::MAX))
            else {
                return false;
            };
            let branches = format!("{:?}", draft.branches);
            definitions.extend(named(&branches, "DefinitionId"));
            tables.extend(named(&branches, "MadeTableId"));
            continue;
        }
        let Some(table) = tables.pop() else {
            return true;
        };
        if !seen_tables.insert(table) {
            continue;
        }
        let Some(made) = drafts
            .tables
            .get(usize::try_from(table).unwrap_or(usize::MAX))
        else {
            return false;
        };
        let fill = format!("{:?}", made.fill);
        definitions.extend(named(&fill, "DefinitionId"));
        tables.extend(named(&fill, "MadeTableId"));
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

/// Whether every ask of a definition's table-form patterns is one of `sites`, none with no site.
fn covered(drafts: &Drafts, definition: DefinitionId, sites: &HashSet<u64>) -> bool {
    let Some(draft) = usize::try_from(definition.0)
        .ok()
        .and_then(|place| drafts.definitions.get(place))
    else {
        return false;
    };
    let mut tables = draft
        .patterns
        .iter()
        .filter(|pattern| matches!(pattern.form, Some(PatternForm::Table(_))));
    tables.all(|pattern| {
        !pattern.opaque
            && pattern
                .sites
                .iter()
                .all(|site| sites.contains(&site.serial))
    })
}

/// Drops the sites a definition's table's pipelines wrote before site `below`: they are written again.
fn purged(drafts: &mut Drafts, origin: DefinitionId, below: u64) {
    for draft in &mut drafts.definitions {
        for pattern in &mut draft.patterns {
            pattern
                .sites
                .retain(|site| site.origin != Some(origin) || site.serial >= below);
        }
    }
}

/// The table-form patterns of a definition as the trace says them: each key, its sites, whether some ask had none.
fn readers_said(drafts: &Drafts, definition: DefinitionId) -> String {
    let Some(draft) = usize::try_from(definition.0)
        .ok()
        .and_then(|place| drafts.definitions.get(place))
    else {
        return String::new();
    };
    draft
        .patterns
        .iter()
        .map(|pattern| {
            format!(
                "[{} bound {:?} sites {} opaque {}]",
                match pattern.form {
                    Some(PatternForm::Table(_)) => "table",
                    Some(PatternForm::Call(_)) => "call",
                    None => "none",
                },
                pattern
                    .asked
                    .bound
                    .0
                    .iter()
                    .map(|column| column.0)
                    .collect::<Vec<_>>(),
                pattern.sites.len(),
                pattern.opaque
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Says why a table was not restricted, when traced.
fn skipped(definition: DefinitionId, why: &str) {
    if std::env::var_os("RYO_L1B_RESTRICT_TRACE").is_some() {
        eprintln!("ryo l1b: not restricted definition={} {why}", definition.0);
    }
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
    place_asked: &mut dyn FnMut(&mut Drafts) -> Result<(), RefusedPlace>,
) -> Vec<Restricted> {
    let mut done = Vec::new();
    let count = drafts.definitions.len();
    // A definition reads only definitions before it: from the last, every reader of a table is rewritten before it.
    for place in (0..count).rev() {
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
            skipped(
                definition,
                &format!(
                    "table={} readers: {}",
                    table.0,
                    readers_said(drafts, definition)
                ),
            );
            continue;
        };
        if !sites.iter().all(|site| {
            site.prefix
                .iter()
                .all(|atom| before(drafts, &format!("{atom:?}"), definition))
        }) {
            skipped(definition, "a reader's prefix reads what leads back to it");
            continue;
        }
        let keys: Option<Vec<Vec<FlatTerm>>> = sites
            .iter()
            .map(|site| key_cells(site, definition, &mask))
            .collect();
        let Some(keys) = keys else {
            skipped(definition, "a reader does not read it by its key cells");
            continue;
        };
        // A key a test holds to a constant may be given by no atom before the ask: the keys' branch could not run.
        let given = sites.iter().zip(&keys).all(|(site, row)| {
            row.iter().all(|cell| match cell {
                FlatTerm::Variable(variable) => site
                    .prefix
                    .iter()
                    .any(|atom| variables(atom).contains(variable)),
                FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => true,
            })
        });
        if !given {
            skipped(definition, "a key no atom before the ask gives");
            continue;
        }
        let asked = sites
            .iter()
            .fold(PlanEstimate(0), |sum, site| plus(sum, site.rows));
        let whole = planned_rows(definition_rows(drafts, definition));
        let margin = PlanEstimate(margin.saturating_mul(ONE.0));
        if times(asked, margin) >= whole {
            skipped(definition, &format!("keys {} of {} rows", asked.0, whole.0));
            continue;
        }
        let Some(draft) = drafts.definitions.get(place) else {
            continue;
        };
        let (node, home, shape) = (draft.node, draft.home, draft.shape.clone());
        let keyless = !draft.keyless.is_empty();
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
        let serials: HashSet<u64> = sites.iter().map(|site| site.serial).collect();
        let first_new = drafts.asking.serial;
        // The keys are the rows their readers hold when they ask: so many, not their branches' own estimate.
        if let Some(draft) = drafts
            .definitions
            .get(usize::try_from(keyed.0).unwrap_or(usize::MAX))
        {
            let _set = draft.rows.set(RowEstimate(asked.0.div_ceil(ONE.0).max(1)));
        }
        let waiting = drafts.waiting.len();
        let _new = drafts.keys.insert(keyed);
        // The keys' table is made here: a prefix that only a program can run has no pipeline, and the table is not
        // restricted (made later, its failure would refuse the reader's rule).
        if definition_table(drafts, sample, keyed).is_err() {
            let _gone = drafts.keys.remove(&keyed);
            skipped(definition, "its keys have no pipeline");
            continue;
        }
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
            // A plan the keys leave a product stage in is not this table's: its definition states no keyless join.
            let written = planned(drafts, &joined, &HashSet::new(), UnitRole::Pipeline)
                .filter(|plan| plan.products.0 == 0 || keyless)
                .and_then(|plan| pipeline(drafts, sample.kinds, &joined, &plan, Some(definition)));
            match written {
                Some(written) => pipelines.push(written),
                None => break,
            }
        }
        if pipelines.len() != own.len() {
            skipped(definition, "a branch joined with its keys has no pipeline");
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
        // What the new pipelines asked is placed now: a pattern that cannot be refuses no rule, the table gets its
        // fill back.
        if place_asked(drafts).is_err() {
            drafts.waiting.truncate(waiting);
            if let Some(made) = drafts
                .tables
                .get_mut(usize::try_from(table.0).unwrap_or(usize::MAX))
            {
                made.fill = original;
            }
            skipped(definition, "a pattern its pipelines ask cannot be placed");
            continue;
        }
        purged(drafts, definition, first_new);
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
            sites: serials,
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
        if covered(drafts, each.definition, &each.sites) {
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
