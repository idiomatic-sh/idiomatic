//! The services of the `sharing` part (SPIKE): after a table's pipelines are written, the pipelines that begin with
//! the same run of stages run it once. The run is written as a made table of its own: one pipeline of the shared
//! stages, whose row is the registers the run fills and a pipeline reads after it; each pipeline that began with the
//! run now scans that table, reads those columns into its own registers, and goes on with its own stages.
//!
//! Two runs are the same when their stages are equal by their own values with the registers compared up to renaming:
//! the stages are compared as they print themselves, each register replaced by the order it is first met in. No shape
//! of a rule is recognised.
//!
//! Why a made table and not one pipeline that fans out: a pipeline is a list of stages with one row at its end, and a
//! table's fill is a list of pipelines; nothing in the stage list says "from here, each of these". A table says it with
//! the two stages every pipeline already has (a scan, a read).
//!
//! What is shared: the common run cut back to its last `Keep` (after a keep its rows are fewest; a stage that gives
//! many rows for one is left to each pipeline), when it has at least `LEAST_STAGES` stages and the registers the
//! pipelines read after it are no more than `READS_PER_STAGE` times its stages; a run with no keep is shared whole
//! when a scan and a read a register cost fewer stages than it has. The costing's estimates are of drafts, before the
//! pipelines are written, and are not at hand here: this rule is of the written stages alone.

use super::types::SharedPrefix;
use crate::domain::lowered::types::{Emptiness, LoweredPack, MadeFill, MadeTable};
use crate::domain::steps::stages::types::{FillStage, Pipeline, PlacedStage};
use crate::domain::steps::types::{
    MadeColumn, MadeTableId, MatchColumns, ReadColumn, Register, RegisterCount, StepOperand,
    StepOperands, StepRows,
};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::table_words::types::ColumnCount;

/// The fewest stages a shared run has: a scan alone is not worth a table.
const LEAST_STAGES: usize = 2;

/// The most registers read after a run, for each of its stages.
const READS_PER_STAGE: usize = 2;

/// The registers a text names, in the order named (a stage or an operand list as it prints itself).
fn registers_of(text: &str) -> Vec<u32> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("Register(") {
        let after = rest.get(at.saturating_add(9)..).unwrap_or_default();
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty()
            && after
                .get(digits.len()..)
                .is_some_and(|tail| tail.starts_with(')'))
            && let Ok(number) = digits.parse::<u32>()
        {
            found.push(number);
        }
        rest = after;
    }
    found
}

/// A stage as it prints itself, each register replaced by its place in `met` (added when it is new).
fn renamed(stage: &FillStage, met: &mut Vec<u32>) -> String {
    let mut text = format!("{stage:?}");
    for register in registers_of(&text) {
        if !met.contains(&register) {
            met.push(register);
        }
    }
    // Longest numbers first, so that a register's number is never read inside another's.
    let mut order: Vec<(usize, u32)> = met.iter().copied().enumerate().collect();
    order.sort_by_key(|(_, register)| std::cmp::Reverse(*register));
    for (place, register) in order {
        text = text.replace(
            &format!("Register({register})"),
            &format!("Register(#{place})"),
        );
    }
    text
}

/// One pipeline's stages as they compare, and the registers it meets in each.
struct Compared {
    /// Each stage, registers renamed by order met.
    stages: Vec<String>,
    /// The registers met after each stage, in the order met.
    met: Vec<Vec<u32>>,
}

/// A pipeline as it compares.
fn compared(pipeline: &Pipeline) -> Compared {
    let mut met = Vec::new();
    let mut said = Compared {
        stages: Vec::new(),
        met: Vec::new(),
    };
    for placed in &pipeline.stages {
        said.stages.push(renamed(&placed.stage, &mut met));
        said.met.push(met.clone());
    }
    said
}

/// How many leading stages every one of `members` has alike.
fn alike(all: &[Compared], members: &[usize]) -> usize {
    let Some(first) = members.first().and_then(|place| all.get(*place)) else {
        return 0;
    };
    let mut length = first.stages.len();
    for place in members.iter().skip(1) {
        let Some(other) = all.get(*place) else {
            return 0;
        };
        let same = first
            .stages
            .iter()
            .zip(&other.stages)
            .take_while(|(a, b)| a == b)
            .count();
        length = length.min(same);
    }
    length
}

/// A digest for a prefix's table: its table's own, marked with the prefix's number, so that no two units bear one.
fn prefix_node(of: NodeDigest, number: usize) -> NodeDigest {
    let mut bytes = of.0;
    let mark = u8::try_from(number.saturating_add(1) % 251).unwrap_or(1);
    if let Some(last) = bytes.last_mut() {
        *last = last.wrapping_add(mark).rotate_left(3) ^ 0xA5;
    }
    if let Some(first) = bytes.first_mut() {
        *first = first.wrapping_add(mark) ^ 0x5A;
    }
    NodeDigest(bytes)
}

/// Shares the runs the pipelines of one table's `Rows` fill begin with; answers the tables made for them.
fn table_shared(pack: &mut LoweredPack, of: usize, made: &mut Vec<SharedPrefix>) {
    let Some(table) = pack.tables.get_mut(of) else {
        return;
    };
    let node = table.node;
    let MadeFill::Rows(pipelines, _) = &mut table.fill else {
        return;
    };
    if pipelines.len() < 2 {
        return;
    }
    let all: Vec<Compared> = pipelines.iter().map(compared).collect();
    // The pipelines are grouped by their first `LEAST_STAGES` stages; a group shares the longest run all its members have.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (place, said) in all.iter().enumerate() {
        let key = said.stages.get(..LEAST_STAGES);
        let home = groups.iter_mut().find(|group| {
            group
                .first()
                .and_then(|first| all.get(*first))
                .is_some_and(|first| key.is_some() && first.stages.get(..LEAST_STAGES) == key)
        });
        match home {
            Some(group) => group.push(place),
            None => groups.push(vec![place]),
        }
    }
    let mut owned: Vec<Option<Pipeline>> = std::mem::take(pipelines)
        .into_vec()
        .into_iter()
        .map(Some)
        .collect();
    let mut tables: Vec<MadeTable> = Vec::new();
    let first_new = pack.tables.len();
    let mut registers = pack.registers.0;
    for members in groups.iter().filter(|group| group.len() >= 2) {
        let length = alike(&all, members);
        let Some(leader) = members.first().copied() else {
            continue;
        };
        let Some(Some(leader_pipeline)) = owned.get(leader) else {
            continue;
        };
        // The run ends at its last `Keep`: after a keep its rows are fewest, and a stage that gives many rows for one
        // (a product, a probe) is left to each pipeline, which keeps few of them; a run with no keep is shared whole.
        let last_keep = leader_pipeline
            .stages
            .iter()
            .take(length)
            .rposition(|placed| matches!(placed.stage, FillStage::Keep { .. }));
        let length = last_keep.map_or(length, |place| place.saturating_add(1));
        if length < LEAST_STAGES {
            continue;
        }
        // The registers the run fills that SOME member reads after it (in its later stages, or in its row), by the
        // order the run meets them: a member's own stages may read one the leader's do not.
        let mut live: Vec<usize> = Vec::new();
        for member in members {
            let (Some(said), Some(Some(pipeline))) = (all.get(*member), owned.get(*member)) else {
                continue;
            };
            let filled = said
                .met
                .get(length.saturating_sub(1))
                .cloned()
                .unwrap_or_default();
            let later: String = pipeline
                .stages
                .iter()
                .skip(length)
                .map(|placed| format!("{:?}", placed.stage))
                .chain([format!("{:?}", pipeline.row)])
                .collect();
            let read_later = registers_of(&later);
            for (place, register) in filled.iter().enumerate() {
                if read_later.contains(register) && !live.contains(&place) {
                    live.push(place);
                }
            }
        }
        live.sort_unstable();
        // With no keep in it the run saves its stages and costs a scan and a read a register: shared when that is less.
        let reads = live.len().saturating_add(1);
        let worth = if last_keep.is_some() {
            live.len() <= length.saturating_mul(READS_PER_STAGE)
        } else {
            reads < length
        };
        if live.is_empty() || !worth {
            continue;
        }
        let number = first_new.saturating_add(tables.len());
        let Ok(id) = u32::try_from(number) else {
            continue;
        };
        let source = StepRows::Made(MadeTableId(id));
        let mut shared_stages: Option<Vec<PlacedStage>> = None;
        let mut shared_row: Vec<StepOperand> = Vec::new();
        for member in members {
            let (Some(said), Some(slot)) = (all.get(*member), owned.get_mut(*member)) else {
                continue;
            };
            let Some(pipeline) = slot.take() else {
                continue;
            };
            let names = said
                .met
                .get(length.saturating_sub(1))
                .cloned()
                .unwrap_or_default();
            let mut stages = pipeline.stages.into_vec();
            let own = stages.split_off(length.min(stages.len()));
            if shared_stages.is_none() {
                shared_row = live
                    .iter()
                    .filter_map(|place| names.get(*place).copied())
                    .map(|register| StepOperand::Register(Register(register)))
                    .collect();
                shared_stages = Some(stages);
            }
            let row = Register(registers);
            registers = registers.saturating_add(1);
            let mut reads = vec![PlacedStage {
                point: None,
                stage: FillStage::Scan {
                    source,
                    columns: MatchColumns(Box::new([])),
                    values: StepOperands(Box::new([])),
                    out: row,
                },
            }];
            for (column, place) in live.iter().enumerate() {
                let Some(out) = names.get(*place).copied() else {
                    continue;
                };
                reads.push(PlacedStage {
                    point: None,
                    stage: FillStage::Read {
                        source,
                        column: ReadColumn::Made(MadeColumn(
                            u16::try_from(column).unwrap_or(u16::MAX),
                        )),
                        row,
                        out: Register(out),
                    },
                });
            }
            reads.extend(own);
            *slot = Some(Pipeline {
                stages: reads.into(),
                row: pipeline.row,
            });
        }
        if let Some(stages) = shared_stages {
            tables.push(MadeTable {
                node: prefix_node(node, tables.len()),
                fill: MadeFill::Rows(
                    Box::new([Pipeline {
                        stages: stages.into(),
                        row: StepOperands(shared_row.into()),
                    }]),
                    RepeatedRows::Kept,
                ),
                width: ColumnCount(live.len()),
                indexes: Vec::new(),
                emptiness: Emptiness::Unknown,
            });
        }
    }
    let rebuilt: Vec<Pipeline> = owned.into_iter().flatten().collect();
    if let Some(table) = pack.tables.get_mut(of)
        && let MadeFill::Rows(pipelines, _) = &mut table.fill
    {
        *pipelines = rebuilt.into();
    }
    for (place, table) in tables.into_iter().enumerate() {
        let number = first_new.saturating_add(place);
        made.push(SharedPrefix {
            table: MadeTableId(u32::try_from(number).unwrap_or(u32::MAX)),
            of: MadeTableId(u32::try_from(of).unwrap_or(u32::MAX)),
        });
        pack.tables.push(table);
    }
    pack.registers = RegisterCount(registers);
}

/// Shares, in every table of the pack whose rows several pipelines write, the runs of stages those pipelines begin
/// with; answers the tables made, each with the table it was taken from. Off under `RYO_X_NO_SHARING` (the spike's
/// switch, for a row before and a row after from one build).
pub(crate) fn prefixes_shared(pack: &mut LoweredPack) -> Vec<SharedPrefix> {
    let mut made = Vec::new();
    if std::env::var_os("RYO_X_NO_SHARING").is_some() {
        return made;
    }
    for of in 0..pack.tables.len() {
        table_shared(pack, of, &mut made);
    }
    made
}
