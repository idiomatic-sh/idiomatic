//! The services of the `running` part: the walk of everything a rule can reach in the code (its branches' programs, the
//! bodies and patterns they ask, the tables they read and the tables' fills, the tables a need may make), asking of each
//! word and fill whether the executor runs it. The load refuses a rule that can reach one it does not, so no such rule
//! reaches a scan. Each `Op` is named: a code that is newly built must be marked as running here to compile.
//!
//! The fill side's words and fills are listed here as it runs them at its frozen head (`claude/engine-x2` at 3e01ab9d7);
//! a fill kind it does not run is its own to say (`fills::service::fill_at` refuses it).

use super::types::{WalkPlace, WordBuilt};
use crate::domain::code::anchoring::service::{
    each_match_filled_many_of, each_match_filled_of, each_match_made_of, each_match_node_of,
    each_row_made_of, latest_of,
};
use crate::domain::code::constants::BUCKET;
use crate::domain::code::controlling::service::{exists_of, fork_of, or_of};
use crate::domain::code::deciding::service::test_of;
use crate::domain::code::filling::service::{
    stage_keep_of, stage_latest_of, stage_probe_of, stage_product_of, stage_scan_of, stage_use_of,
};
use crate::domain::code::records::service::body_at;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::small_codes::service::comparison_of;
use crate::domain::code::types::{
    CodeEntry, CodeWord, IndexSite, Op, PoolAt, ProgramCode, RuleCode, WordSlot,
};
use crate::domain::code::walking::service::nearest_of;
use crate::domain::made_tables::fills::service::{fill_at, input_at};
use crate::domain::made_tables::fills::types::FillKind;
use crate::domain::made_tables::reading::service::{latest_site, site_at, source_of};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::needs::service::tables_asked;
use crate::domain::programs::operands::service::listed;
use crate::domain::steps::types::{MadeTableId, TestOperation};

/// Whether the executor runs a test: every comparison of two cells, a set either way, a regular expression and a glob.
const fn tested(test: Option<TestOperation>) -> WordBuilt {
    match test {
        Some(
            TestOperation::Equal
            | TestOperation::NotEqual
            | TestOperation::Less
            | TestOperation::LessOrEqual
            | TestOperation::Greater
            | TestOperation::GreaterOrEqual
            | TestOperation::In
            | TestOperation::NotIn
            | TestOperation::Matches(_)
            | TestOperation::Glob
            | TestOperation::NotGlob,
        ) => WordBuilt::Runs,
        Some(
            TestOperation::StartsWith
            | TestOperation::EndsWith
            | TestOperation::NumericAtLeast
            | TestOperation::NaturalLess
            | TestOperation::Successor,
        )
        | None => WordBuilt::NotYet,
    }
}

/// The table a site's index is over, when its source is a made table.
fn site_table(code: &ProgramCode, site: IndexSite, out: &mut Vec<WalkPlace>) {
    if let Ok(index) = site_at(code, site)
        && index.source.0 == 2
    {
        out.push(WalkPlace::Table(MadeTableId(index.number.0)));
    }
}

/// What a body record reaches: an inline body its program in the asking unit, a pattern in call form its program in its
/// own unit, a pattern in table form its table. Answers the place after the record; none when the pool does not hold it.
fn body(
    code: &ProgramCode,
    at: PoolAt,
    unit: WordSlot,
    out: &mut Vec<WalkPlace>,
) -> Option<PoolAt> {
    let read = body_at(&code.pool, at)?;
    match read.record {
        BodyRecord::Inline { entry } => out.push(WalkPlace::Program { entry, unit }),
        BodyRecord::Call {
            entry, unit: own, ..
        } => out.push(WalkPlace::Program { entry, unit: own }),
        BodyRecord::Table { index, .. } => site_table(code, index, out),
    }
    Some(read.after)
}

/// Whether the loop runs a word of a step program, and what the word reaches.
fn step_word(
    code: &ProgramCode,
    word: CodeWord,
    unit: WordSlot,
    out: &mut Vec<WalkPlace>,
) -> WordBuilt {
    match word.op {
        Op::EachMatchNode => {
            site_table(code, each_match_node_of(word).index, out);
            WordBuilt::Runs
        }
        Op::EachMatchFilled => {
            site_table(code, each_match_filled_of(word).index, out);
            WordBuilt::Runs
        }
        Op::EachMatchFilledMany => {
            site_table(code, each_match_filled_many_of(word).index, out);
            WordBuilt::Runs
        }
        Op::EachMatchMade => {
            site_table(code, each_match_made_of(word).index, out);
            WordBuilt::Runs
        }
        Op::End
        | Op::EachKind
        | Op::EachRowNode
        | Op::EachRowFilled
        | Op::ReadNode
        | Op::ReadFilled
        // A read is by a handle another word found: its table was reached there.
        | Op::ReadMade
        | Op::TestEqualConstant
        | Op::TestNotEqualConstant
        | Op::TestEqualRegister
        | Op::TestNotEqualRegister
        | Op::TestInConstants
        | Op::Let
        | Op::Yield
        | Op::Emit
        | Op::Axis => WordBuilt::Runs,
        Op::EachRowMade => {
            out.push(WalkPlace::Table(each_row_made_of(word).table));
            WordBuilt::Runs
        }
        Op::Latest => {
            if let Ok(site) = latest_site(code, latest_of(word).operands) {
                site_table(code, site, out);
            }
            WordBuilt::Runs
        }
        Op::Test => {
            let read = test_of(word);
            tested(comparison_of(read.comparison, read.regex))
        }
        Op::Exists => match body(code, exists_of(word).body, unit, out) {
            Some(_) => WordBuilt::Runs,
            None => WordBuilt::NotYet,
        },
        Op::Nearest => match body(code, nearest_of(word).operands, unit, out) {
            Some(_) => WordBuilt::Runs,
            None => WordBuilt::NotYet,
        },
        Op::Or => {
            let asking = or_of(word);
            let mut at = Some(asking.records);
            for _ in 0..asking.bodies.0 {
                at = at.and_then(|place| body(code, place, unit, out));
            }
            match at {
                Some(_) => WordBuilt::Runs,
                None => WordBuilt::NotYet,
            }
        }
        Op::Fork => {
            for entry in listed(&code.pool, fork_of(word).entries) {
                out.push(WalkPlace::Program {
                    entry: CodeEntry(entry.0),
                    unit,
                });
            }
            WordBuilt::Runs
        }
        // The loop does not run these yet.
        Op::EachRange
        | Op::Children
        | Op::ChildrenOfField
        | Op::Within
        | Op::NearestDerived
        | Op::Closure
        | Op::Call
        | Op::Render => WordBuilt::NotYet,
        // A stage is no word of a step program.
        Op::StageScan
        | Op::StageProbe
        | Op::StageLatest
        | Op::StageRead
        | Op::StageKeep
        | Op::StageCall
        | Op::StageProduct
        | Op::StageUse
        | Op::StageRender
        | Op::StageWalk
        | Op::StageWrite => WordBuilt::NotYet,
    }
}

/// The table a stage reads as a source, when it is one.
fn source_table(source: ReadSource, out: &mut Vec<WalkPlace>) {
    match source {
        ReadSource::Made(table) | ReadSource::Written(table) => out.push(WalkPlace::Table(table)),
        ReadSource::Nodes(_) | ReadSource::Filled(_) => {}
    }
}

/// Whether the fill side runs a stage word of a pipeline, and what the stage reaches.
fn stage_word(
    code: &ProgramCode,
    word: CodeWord,
    unit: WordSlot,
    out: &mut Vec<WalkPlace>,
) -> WordBuilt {
    match word.op {
        Op::End | Op::StageRead | Op::StageWalk | Op::StageWrite => WordBuilt::Runs,
        Op::StageScan => {
            let stage = stage_scan_of(word);
            if stage.flag.0 & BUCKET.0 == 0 {
                source_table(source_of(stage.flag, stage.read), out);
            } else {
                site_table(code, IndexSite(stage.read.0), out);
            }
            WordBuilt::Runs
        }
        Op::StageProduct => {
            let stage = stage_product_of(word);
            source_table(source_of(stage.flag, stage.read), out);
            WordBuilt::Runs
        }
        Op::StageProbe => {
            site_table(code, stage_probe_of(word).index, out);
            WordBuilt::Runs
        }
        Op::StageLatest => {
            if let Ok(site) = latest_site(code, stage_latest_of(word).operands) {
                site_table(code, site, out);
            }
            WordBuilt::Runs
        }
        Op::StageKeep => {
            let stage = stage_keep_of(word);
            tested(comparison_of(stage.comparison, stage.regex))
        }
        Op::StageUse => match body(code, stage_use_of(word).body, unit, out) {
            Some(_) => WordBuilt::Runs,
            None => WordBuilt::NotYet,
        },
        // The fill side answers these as not built.
        Op::StageCall | Op::StageRender => WordBuilt::NotYet,
        // A word of a step program is no stage.
        Op::EachKind
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
        | Op::Emit => WordBuilt::NotYet,
    }
}

/// The unit of a table when the fill side does not run its fill; none when it does. What the fill reaches is pushed: its
/// input's pipelines, and the tables its own need may ask.
fn fill(code: &ProgramCode, table: MadeTableId, out: &mut Vec<WalkPlace>) -> Option<WordSlot> {
    let Some(written) = code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
    else {
        return Some(WordSlot(0));
    };
    let unit = written.unit;
    let mut asked = Vec::new();
    tables_asked(code, written.need, &mut asked);
    out.extend(asked.into_iter().map(WalkPlace::Table));
    match fill_at(code, table).map(|held| (held.kind, held.inputs)) {
        Ok((FillKind::Constant, _)) => None,
        Ok((FillKind::Rows | FillKind::Count | FillKind::Sorted, inputs)) => {
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
        Err(_) => Some(unit),
    }
}

/// The first unit `rule` can reach that holds a word or a fill the executor does not run; none when it runs them all.
pub(crate) fn unit_not_run(code: &ProgramCode, rule: &RuleCode) -> Option<WordSlot> {
    let mut pending: Vec<WalkPlace> = Vec::new();
    let mut asked = Vec::new();
    for branch in &rule.branches {
        pending.push(WalkPlace::Program {
            entry: branch.entry,
            unit: rule.unit,
        });
        tables_asked(code, branch.need, &mut asked);
    }
    pending.extend(asked.into_iter().map(WalkPlace::Table));
    let (mut words, mut tables) = (
        vec![false; code.words.len()],
        vec![false; code.tables.len()],
    );
    while let Some(next) = pending.pop() {
        let (entry, unit) = match next {
            WalkPlace::Program { entry, unit } | WalkPlace::Pipeline { entry, unit } => {
                (entry, unit)
            }
            WalkPlace::Table(table) => {
                let seen = tables.get_mut(usize::try_from(table.0).unwrap_or(usize::MAX));
                if let Some(seen) = seen.filter(|seen| !**seen) {
                    *seen = true;
                    if let Some(unit) = fill(code, table, &mut pending) {
                        return Some(unit);
                    }
                }
                continue;
            }
        };
        let mut at = usize::try_from(entry.0).unwrap_or(usize::MAX);
        // A word walked before ends the walk: the words after it were walked then.
        while let Some(seen) = words.get_mut(at).filter(|seen| !**seen) {
            *seen = true;
            let Some(word) = code.words.get(at).copied() else {
                break;
            };
            let built = if matches!(next, WalkPlace::Pipeline { .. }) {
                stage_word(code, word, unit, &mut pending)
            } else {
                step_word(code, word, unit, &mut pending)
            };
            if built == WordBuilt::NotYet {
                return Some(unit);
            }
            if word.op == Op::End {
                break;
            }
            at = at.saturating_add(1);
        }
    }
    None
}
