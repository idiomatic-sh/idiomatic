//! The services of the `filling` part: what a stage word of a pipeline and a table's fill reach, which the walk follows.
//! Whether the fill side runs a stage or a fill is its own one answer (`made_tables::built::service::runs`), asked here:
//! this part holds no list of what runs.

use crate::domain::code::constants::BUCKET;
use crate::domain::code::filling::service::{
    stage_latest_of, stage_probe_of, stage_product_of, stage_scan_of, stage_use_of,
};
use crate::domain::code::types::{
    CodeEntry, CodeWord, FillCode, IndexSite, NeedSite, Op, ProgramCode, WordSlot,
};
use crate::domain::executing::running::places::service::{body, site_table};
use crate::domain::executing::running::types::WalkPlace;
use crate::domain::filled::types::FillRuns;
use crate::domain::made_tables::built::service::runs;
use crate::domain::made_tables::built::types::FillPart;
use crate::domain::made_tables::fills::service::{fill_at, input_at};
use crate::domain::made_tables::fills::types::FillKind;
use crate::domain::made_tables::reading::service::{latest_site, source_of};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::needs::service::tables_asked;
use crate::domain::steps::types::MadeTableId;

/// Whether the fill side reads a stage's source, and the table when the source is one. A table read while its fill
/// writes it, it does not read yet.
fn source_table(source: ReadSource, out: &mut Vec<WalkPlace>) -> FillRuns {
    match source {
        ReadSource::Made(table) => {
            out.push(WalkPlace::Table(table));
            FillRuns::Runs
        }
        ReadSource::Nodes(_) | ReadSource::Filled(_) => FillRuns::Runs,
        ReadSource::Written(_) => FillRuns::NotYet,
    }
}

/// Whether the fill side runs a stage word of a pipeline, which is its own to say (`made_tables::built::service::runs`),
/// and what the stage reaches, which the walk follows: the table a stage reads or probes, the pattern a `Use` asks.
pub(crate) fn stage_word(
    code: &ProgramCode,
    word: CodeWord,
    unit: WordSlot,
    out: &mut Vec<WalkPlace>,
) -> FillRuns {
    if word.op == Op::End {
        return FillRuns::Runs;
    }
    if runs(code, FillPart::Stage(word)) == FillRuns::NotYet {
        return FillRuns::NotYet;
    }
    let reached = match word.op {
        Op::StageScan => {
            let stage = stage_scan_of(word);
            if stage.flag.0 & BUCKET.0 == 0 {
                source_table(source_of(stage.flag, stage.read), out)
            } else {
                site_table(code, IndexSite(stage.read.0), out)
            }
        }
        Op::StageProduct => {
            let stage = stage_product_of(word);
            source_table(source_of(stage.flag, stage.read), out)
        }
        Op::StageProbe => site_table(code, stage_probe_of(word).index, out),
        Op::StageLatest => match latest_site(code, stage_latest_of(word).operands) {
            Ok(site) => site_table(code, site, out),
            Err(_) => FillRuns::NotYet,
        },
        Op::StageUse => match body(code, stage_use_of(word).body, unit, out) {
            Some(_) => FillRuns::Runs,
            None => FillRuns::NotYet,
        },
        Op::StageRead
        | Op::StageKeep
        | Op::StageCall
        | Op::StageRender
        | Op::StageWalk
        | Op::StageWrite => FillRuns::Runs,
        // A word of a step program is no stage: the fill side's answer above was `NotYet`.
        Op::End
        | Op::EachKind
        | Op::EachMatchNode
        | Op::EachMatchFilled
        | Op::EachMatchFilledMany
        | Op::EachMatchMade
        | Op::EachRange
        | Op::EachRowNode
        | Op::EachRowFilled
        | Op::EachRowMade
        | Op::Children
        | Op::ChildrenOfField
        | Op::Axis
        | Op::Within
        | Op::Nearest
        | Op::NearestDerived
        | Op::Closure
        | Op::ReadNode
        | Op::ReadFilled
        | Op::ReadMade
        | Op::Test
        | Op::TestEqualConstant
        | Op::TestNotEqualConstant
        | Op::TestEqualRegister
        | Op::TestNotEqualRegister
        | Op::TestInConstants
        | Op::Call
        | Op::Render
        | Op::Let
        | Op::Exists
        | Op::Or
        | Op::Fork
        | Op::Yield
        | Op::Latest
        | Op::Emit => FillRuns::NotYet,
    };
    reached
}

/// The unit of a table when the fill side does not run its fill; none when it does. What the fill reaches is pushed: its
/// input's pipelines, and the tables its own need may ask.
pub(crate) fn fill(
    code: &ProgramCode,
    table: MadeTableId,
    sites: &mut hashbrown::HashSet<NeedSite>,
    out: &mut Vec<WalkPlace>,
) -> Option<WordSlot> {
    let Some(written) = code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
    else {
        return Some(WordSlot(0));
    };
    let unit = written.unit;
    if runs(code, FillPart::Fill(FillCode(written.fill.0))) == FillRuns::NotYet {
        return Some(unit);
    }
    let mut asked = Vec::new();
    tables_asked(code, written.need, sites, &mut asked);
    out.extend(asked.into_iter().map(WalkPlace::Table));
    match fill_at(code, table).map(|held| (held.kind, held.inputs)) {
        Ok((FillKind::Constant, _)) => None,
        Ok((
            FillKind::Rows
            | FillKind::Count
            | FillKind::Sorted
            | FillKind::Ends
            | FillKind::Places
            | FillKind::Follow,
            inputs,
        )) => {
            match input_at(code, inputs) {
                Ok(input) => {
                    out.extend(input.entries.iter().map(|entry| WalkPlace::Pipeline {
                        entry: CodeEntry(entry.0),
                        unit,
                    }));
                    None
                }
                Err(_) => Some(unit),
            }
        }
        // An ordered match has two inputs, one after the other in its record.
        Ok((FillKind::Match, inputs)) => {
            let both = input_at(code, inputs)
                .and_then(|left| input_at(code, left.after).map(|right| [left, right]));
            match both {
                Ok(sides) => {
                    for side in sides {
                        out.extend(side.entries.iter().map(|entry| WalkPlace::Pipeline {
                            entry: CodeEntry(entry.0),
                            unit,
                        }));
                    }
                    None
                }
                Err(_) => Some(unit),
            }
        }
        Err(_) => Some(unit),
    }
}
