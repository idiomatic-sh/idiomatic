//! The services of the `sites` part: what a measuring worker does when a file's scan ends. The rows of each size site of
//! the code are counted, through the seam, and then the file's counts are folded (`measuring::service::file_ended`). It
//! is apart from the hooks because it asks the fill side, which the hooks' own file must not: the loop's answers to the
//! fill side call the hooks.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::need_at;
use crate::domain::code::records::types::NeedRecord;
use crate::domain::code::types::{ConstantNumber, NeedSite, RelationNumber, WordSlot};
use crate::domain::executing::measuring::service::{file_ended, slot_mut};
use crate::domain::executing::types::Asker;
use crate::domain::indexes::reading::service::cell_at;
use crate::domain::made_tables::reading::service::view_of;
use crate::domain::made_tables::sources::service::scanned;
use crate::domain::made_tables::types::ReadSource;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::constant;
use ryo_language::domain::facts::service::text;
use ryo_relations::domain::relations::types::{Count, KindBits};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Counts the rows of each size site of the code in the file: the rows of a filled relation that hold one constant in
/// one column. The relation is filled whole for it, so every site is counted in every file a measuring worker scans.
fn literals_counted(frame: &mut StepFrame<'_, '_>) {
    let code = frame.fill.programs.code;
    for (place, site) in code.sizes.iter().enumerate() {
        let source = ReadSource::Filled(RelationNumber(site.relation.0));
        let wanted = constant(frame, ConstantNumber(site.value.0));
        let asked = scanned(
            frame.fill,
            &mut Asker {
                state: &mut *frame.state,
                found: &mut *frame.found,
                ports: frame.ports,
            },
            source,
        );
        let Ok(whole) = asked else {
            continue;
        };
        // The column is asked once for the site, and its cells are read where they lie.
        let column = view_of(frame.fill.store, source, site.column);
        let rows = (whole.rows.start.0..whole.rows.end.0)
            .filter(|row| cell_at(column, Count(*row)) == wanted)
            .count();
        if let Some(measure) = frame.state.measure.as_deref_mut()
            && let Some(held) = slot_mut(&mut measure.literals, Count(place))
        {
            *held = Count(held.0.saturating_add(rows));
        }
    }
}

/// The file's scan ends: in a measuring worker the size sites' rows are counted and the file's counts folded; a plain
/// worker does nothing.
pub(crate) fn file_measured(frame: &mut StepFrame<'_, '_>) {
    if frame.state.measure.is_some() {
        // SPIKE: under RYO_X_FILLS the size sites are not counted: counting them fills their relations whole in every
        // file, which a plain run does not, and the run is counting the fills a plain run makes.
        static PLAIN_FILLS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if !*PLAIN_FILLS.get_or_init(|| std::env::var_os("RYO_X_FILLS").is_some()) {
            literals_counted(frame);
        }
        file_ended(frame);
    }
}

/// SPIKE: a kind as a term says it: its name among the run's words.
pub(crate) fn kind_named(frame: &StepFrame<'_, '_>, kind: KindBits) -> String {
    let word = frame.fill.programs.kinds.get(usize::from(kind.0)).copied();
    match word {
        Some(word) => format!("kind:{}", text(frame.fill.words, word).0),
        None => format!("kind:#{}", kind.0),
    }
}

/// SPIKE: a row term as it is said: the relation, the column's place and the constant's text, when it has them.
pub(crate) fn row_named(
    frame: &StepFrame<'_, '_>,
    relation: RelationNumber,
    column: WordSlot,
    value: WordSlot,
) -> String {
    let code = frame.fill.programs.code;
    let read = code
        .relations
        .get(usize::try_from(relation.0).unwrap_or(usize::MAX));
    let name = read.map_or_else(|| format!("#{}", relation.0), |held| format!("{held:?}"));
    if column == NO_SLOT || value == NO_SLOT {
        return format!("row:{name}");
    }
    let cell = constant(frame, ConstantNumber(value.0));
    format!(
        "row:{name}.{}={:?}",
        column.0,
        text(frame.fill.words, cell).0
    )
}

/// SPIKE: how deep a traced formula is printed: 6, or what `RYO_X_NEED_DEPTH` says (a measuring worker alone reads it,
/// once), to see what a nested site holds when one rule is traced.
fn need_depth() -> usize {
    static DEPTH: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *DEPTH.get_or_init(|| {
        std::env::var("RYO_X_NEED_DEPTH")
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(6)
    })
}

/// SPIKE: the formula at a place among the needs, as text, to a depth.
fn formula_named(frame: &StepFrame<'_, '_>, at: WordSlot, depth: usize) -> String {
    let code = frame.fill.programs.code;
    let slot_at = |from: WordSlot, ahead: u32| {
        code.needs
            .get(usize::try_from(from.0.wrapping_add(ahead)).unwrap_or(usize::MAX))
            .copied()
    };
    let parts = |count: WordSlot, offsets: WordSlot| -> String {
        let each: Vec<String> = (0..count.0)
            .map(|number| match slot_at(offsets, number) {
                Some(place) if depth < need_depth() => {
                    formula_named(frame, place, depth.saturating_add(1))
                }
                Some(_) | None => "...".to_owned(),
            })
            .collect();
        each.join(", ")
    };
    match need_at(&code.needs, at) {
        None => "?".to_owned(),
        Some(NeedRecord::Unknown) => "unknown".to_owned(),
        Some(NeedRecord::Kinds(list)) => {
            let from = usize::try_from(list.0).unwrap_or(usize::MAX);
            let length = code
                .pool
                .get(from)
                .map_or(0, |held| usize::try_from(held.0).unwrap_or(0));
            let kinds: Vec<String> = code
                .pool
                .get(from.saturating_add(1)..from.saturating_add(1).saturating_add(length))
                .unwrap_or_default()
                .iter()
                .map(|kind| kind_named(frame, KindBits(u16::try_from(kind.0).unwrap_or(u16::MAX))))
                .collect();
            format!("any-kind({})", kinds.join(", "))
        }
        Some(NeedRecord::Row {
            relation,
            column,
            constant,
        }) => row_named(frame, relation, column, constant),
        Some(NeedRecord::Site(site)) => match slot_at(WordSlot(site.0), 0) {
            Some(place) if depth < need_depth() => {
                format!(
                    "site:{}{{{}}}",
                    site.0,
                    formula_named(frame, place, depth.saturating_add(1))
                )
            }
            Some(_) | None => format!("site:{}", site.0),
        },
        Some(NeedRecord::Table(table)) => format!("table:{}", table.0),
        Some(NeedRecord::Path {
            comparison,
            regex,
            constant,
        }) => format!("path:{}:{}:{}", comparison.0, regex.0, constant.0),
        Some(NeedRecord::Bytes { regex, constant }) => format!("bytes:{}:{}", regex.0, constant.0),
        Some(NeedRecord::Shape { parent, field, child }) => {
            format!("shape:{}:{}:{}", parent.0, field.0, child.0)
        }
        Some(NeedRecord::All {
            parts: count,
            offsets,
        }) => format!("all({})", parts(count, offsets)),
        Some(NeedRecord::Any {
            parts: count,
            offsets,
        }) => format!("any({})", parts(count, offsets)),
    }
}

/// SPIKE: the dispatch is about to decide a rule's branch's need: a measuring worker traces the decision, and says the
/// site's formula once.
pub(crate) fn need_decision_begun(
    frame: &mut StepFrame<'_, '_>,
    site: NeedSite,
    rule: SelectedRuleOrdinal,
) {
    if frame.state.measure.is_none() {
        return;
    }
    let known = frame
        .state
        .measure
        .as_deref()
        .is_some_and(|measure| measure.need_fills.formulas.contains_key(&site.0));
    let said = if known {
        None
    } else {
        let place = frame
            .fill
            .programs
            .code
            .needs
            .get(usize::try_from(site.0).unwrap_or(usize::MAX))
            .copied();
        let formula = place.map_or_else(|| "?".to_owned(), |at| formula_named(frame, at, 0));
        Some(format!("rule={} formula={formula}", rule.0))
    };
    if let Some(measure) = frame.state.measure.as_deref_mut() {
        if let Some(text) = said {
            drop(measure.need_fills.formulas.insert(site.0, text));
        }
        measure.need_fills.tracing = Some(site);
        measure.need_fills.nested = Count(0);
        measure.need_fills.trace.clear();
    }
}
