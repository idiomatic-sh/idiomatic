//! The service of the `bytes` part: statistics written as bytes and read back. After the header's three lines (the word
//! that says they are statistics, the version of the engine that wrote them, the stage of the sample that wrote them)
//! come the units: their count, then each unit as its name (what it is, its node's digest, a pattern's bound and
//! collected columns or a table's fill code), its form, its shape, its order, what it was asked and gave (uses, keys,
//! rows, a table's build, and its own time and idle time, which no rule here reads), and the runs of each of its plan
//! points. Every number is eight bytes, the low byte first. A
//! header alone is the statistics of a sample that measured no unit. Bytes that are not exactly this are no statistics.

use crate::domain::code::types::{FillCode, MeasuredForm, ShapeChoice, UnitKey};
use crate::domain::packs::constants::ENGINE_VERSION;
use crate::domain::statistics::constants::{
    FIRST, FIRST_CHOICE, HEADING, LINE_END, NUMBER_WIDTH, PATTERN_UNIT, RULE_UNIT, SECOND,
    SECOND_CHOICE, TABLE_UNIT,
};
use crate::domain::statistics::types::{
    EncodedStatistics, LearnStage, MeasuredUnit, NamedSize, NamedSizes, OrderVerdict,
    ReadStatistics, StatisticsBytes, UnitCounts,
};
use crate::domain::steps::types::{ColumnMask, MadeColumn};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::query_plans::types::NodeDigest;

/// The header of statistics of `stage` written by this engine: its three lines, each ended.
fn header(stage: LearnStage) -> EncodedStatistics {
    let stage = match stage {
        LearnStage::First => FIRST,
        LearnStage::Second => SECOND,
    };
    let mut bytes = Vec::new();
    for line in [HEADING.0, ENGINE_VERSION.as_bytes(), stage.0] {
        bytes.extend_from_slice(line);
        bytes.extend_from_slice(LINE_END.0);
    }
    EncodedStatistics(bytes)
}

/// Writes a number.
fn number_written(bytes: &mut EncodedStatistics, number: Count) {
    bytes
        .0
        .extend_from_slice(&u64::try_from(number.0).unwrap_or(u64::MAX).to_le_bytes());
}

/// Writes the columns of a mask: how many, then each.
fn mask_written(bytes: &mut EncodedStatistics, mask: &ColumnMask) {
    number_written(bytes, Count(mask.0.len()));
    for column in &mask.0 {
        number_written(bytes, Count(usize::from(column.0)));
    }
}

/// Writes a unit's name: what it is, its node's digest, then a pattern's columns or a table's fill code.
fn key_written(bytes: &mut EncodedStatistics, key: &UnitKey) {
    match key {
        UnitKey::Pattern(node, bound, collected) => {
            number_written(bytes, PATTERN_UNIT);
            bytes.0.extend_from_slice(&node.0);
            mask_written(bytes, bound);
            mask_written(bytes, collected);
        }
        UnitKey::Rule(node) => {
            number_written(bytes, RULE_UNIT);
            bytes.0.extend_from_slice(&node.0);
        }
        UnitKey::Table(node, fill) => {
            number_written(bytes, TABLE_UNIT);
            bytes.0.extend_from_slice(&node.0);
            number_written(bytes, Count(usize::from(fill.0)));
        }
    }
}

/// Writes one unit.
fn unit_written(bytes: &mut EncodedStatistics, unit: &MeasuredUnit) {
    key_written(bytes, &unit.key);
    let form = match unit.form {
        MeasuredForm::Call => FIRST_CHOICE,
        MeasuredForm::Table => SECOND_CHOICE,
    };
    let shape = match unit.shape {
        ShapeChoice::Decided => FIRST_CHOICE,
        ShapeChoice::Open => SECOND_CHOICE,
    };
    let order = match unit.order {
        OrderVerdict::Learned => FIRST_CHOICE,
        OrderVerdict::Denied => SECOND_CHOICE,
    };
    for chosen in [form, shape, order] {
        number_written(bytes, chosen);
    }
    let counts = unit.counts;
    for count in [
        counts.calls,
        counts.keys,
        counts.rows,
        counts.build,
        counts.nanos,
        counts.idle_nanos,
    ] {
        number_written(bytes, count);
    }
    number_written(bytes, Count(unit.points.len()));
    for runs in &unit.points {
        number_written(bytes, *runs);
    }
}

/// Writes a text: its length in bytes, then its bytes.
fn text_written(bytes: &mut EncodedStatistics, text: &str) {
    number_written(bytes, Count(text.len()));
    bytes.0.extend_from_slice(text.as_bytes());
}

/// Writes sizes under names: how many, then each name and its rows.
fn sizes_written(bytes: &mut EncodedStatistics, sizes: &[NamedSize]) {
    number_written(bytes, Count(sizes.len()));
    for size in sizes {
        text_written(bytes, &size.name);
        number_written(bytes, size.rows);
    }
}

/// Statistics as bytes: the header of their stage, then their units, then what the sample sized (its files, their
/// elements, the kinds, the fields and the constant buckets, each under its name). Statistics of no unit and no file are
/// their header alone.
#[must_use]
pub(crate) fn written(
    stage: LearnStage,
    units: &[MeasuredUnit],
    sizes: &NamedSizes,
) -> EncodedStatistics {
    let mut bytes = header(stage);
    if !units.is_empty() || sizes.files.0 > 0 {
        number_written(&mut bytes, Count(units.len()));
        for unit in units {
            unit_written(&mut bytes, unit);
        }
        number_written(&mut bytes, sizes.files);
        number_written(&mut bytes, sizes.elements);
        for named in [&sizes.kinds, &sizes.fields, &sizes.literals] {
            sizes_written(&mut bytes, named);
        }
    }
    EncodedStatistics(bytes.0)
}

/// Reads `width` bytes from `at` on, moving `at` past them; none when the bytes end first.
fn taken<'b>(
    bytes: StatisticsBytes<'b>,
    at: &mut Count,
    width: Count,
) -> Option<StatisticsBytes<'b>> {
    let end = at.0.checked_add(width.0)?;
    let held = bytes.0.get(at.0..end)?;
    *at = Count(end);
    Some(StatisticsBytes(held))
}

/// Reads a number.
fn number_read(bytes: StatisticsBytes<'_>, at: &mut Count) -> Option<Count> {
    let held = taken(bytes, at, NUMBER_WIDTH)?;
    // A number the bytes do not hold whole, or one past what a count holds, is not statistics.
    let Ok(number) = held.0.try_into() else {
        return None;
    };
    match usize::try_from(u64::from_le_bytes(number)) {
        Ok(number) => Some(Count(number)),
        Err(_) => None,
    }
}

/// A number that names a column or a fill: one past sixteen bits is not statistics.
fn short_of(number: Count) -> Option<MadeColumn> {
    match u16::try_from(number.0) {
        Ok(short) => Some(MadeColumn(short)),
        Err(_) => None,
    }
}

/// Reads one of two choices: any other number is not statistics.
fn choice_read<T>(bytes: StatisticsBytes<'_>, at: &mut Count, first: T, second: T) -> Option<T> {
    let chosen = number_read(bytes, at)?;
    if chosen == FIRST_CHOICE {
        Some(first)
    } else if chosen == SECOND_CHOICE {
        Some(second)
    } else {
        None
    }
}

/// Reads a list of numbers: how many, then each. A count the bytes left cannot hold is not statistics.
fn numbers_read(bytes: StatisticsBytes<'_>, at: &mut Count) -> Option<Vec<Count>> {
    let count = number_read(bytes, at)?;
    let left = bytes.0.len().checked_sub(at.0)?;
    if count.0 > left / NUMBER_WIDTH.0 {
        return None;
    }
    (0..count.0).map(|_| number_read(bytes, at)).collect()
}

/// Reads the columns of a mask.
fn mask_read(bytes: StatisticsBytes<'_>, at: &mut Count) -> Option<ColumnMask> {
    let columns: Option<Vec<MadeColumn>> =
        numbers_read(bytes, at)?.into_iter().map(short_of).collect();
    Some(ColumnMask(columns?.into()))
}

/// Reads a unit's name.
fn key_read(bytes: StatisticsBytes<'_>, at: &mut Count) -> Option<UnitKey> {
    let kind = number_read(bytes, at)?;
    let digest = taken(bytes, at, Count(size_of::<NodeDigest>()))?;
    let Ok(digest) = digest.0.try_into() else {
        return None;
    };
    let node = NodeDigest(digest);
    if kind == PATTERN_UNIT {
        let bound = mask_read(bytes, at)?;
        Some(UnitKey::Pattern(node, bound, mask_read(bytes, at)?))
    } else if kind == RULE_UNIT {
        Some(UnitKey::Rule(node))
    } else if kind == TABLE_UNIT {
        let fill = short_of(number_read(bytes, at)?)?;
        Some(UnitKey::Table(node, FillCode(fill.0)))
    } else {
        None
    }
}

/// Reads one unit.
fn unit_read(bytes: StatisticsBytes<'_>, at: &mut Count) -> Option<MeasuredUnit> {
    let key = key_read(bytes, at)?;
    let form = choice_read(bytes, at, MeasuredForm::Call, MeasuredForm::Table)?;
    let shape = choice_read(bytes, at, ShapeChoice::Decided, ShapeChoice::Open)?;
    let order = choice_read(bytes, at, OrderVerdict::Learned, OrderVerdict::Denied)?;
    let counts = UnitCounts {
        calls: number_read(bytes, at)?,
        keys: number_read(bytes, at)?,
        rows: number_read(bytes, at)?,
        build: number_read(bytes, at)?,
        nanos: number_read(bytes, at)?,
        idle_nanos: number_read(bytes, at)?,
    };
    Some(MeasuredUnit {
        key,
        form,
        shape,
        order,
        points: numbers_read(bytes, at)?,
        counts,
    })
}

/// Reads a text.
fn text_read<'b>(bytes: StatisticsBytes<'b>, at: &mut Count) -> Option<&'b str> {
    let length = number_read(bytes, at)?;
    match std::str::from_utf8(taken(bytes, at, length)?.0) {
        Ok(text) => Some(text),
        Err(_) => None,
    }
}

/// Reads sizes under names.
fn sizes_read(bytes: StatisticsBytes<'_>, at: &mut Count) -> Option<Vec<NamedSize>> {
    let count = number_read(bytes, at)?;
    let mut sizes = Vec::new();
    for _ in 0..count.0 {
        let name = text_read(bytes, at)?.into();
        sizes.push(NamedSize {
            name,
            rows: number_read(bytes, at)?,
        });
    }
    Some(sizes)
}

/// The statistics bytes hold as they are written, or none when they are of another engine version or are not exactly a
/// header, its units and its sizes. The units are answered sorted by their names, each name once: a name written twice
/// keeps its first unit.
#[must_use]
pub(crate) fn read(bytes: StatisticsBytes<'_>) -> Option<ReadStatistics> {
    let (stage, body) = [LearnStage::First, LearnStage::Second]
        .into_iter()
        .find_map(|stage| Some((stage, bytes.0.strip_prefix(header(stage).0.as_slice())?)))?;
    let (body, mut at) = (StatisticsBytes(body), Count(0));
    let (mut units, mut sizes) = (Vec::new(), NamedSizes::default());
    if !body.0.is_empty() {
        let count = number_read(body, &mut at)?;
        for _ in 0..count.0 {
            units.push(unit_read(body, &mut at)?);
        }
        sizes = NamedSizes {
            files: number_read(body, &mut at)?,
            elements: number_read(body, &mut at)?,
            kinds: sizes_read(body, &mut at)?,
            fields: sizes_read(body, &mut at)?,
            literals: sizes_read(body, &mut at)?,
        };
        // Units and sizes of nothing are a header alone, and bytes after the sizes are nobody's.
        if (count.0 == 0 && sizes.files.0 == 0) || at.0 != body.0.len() {
            return None;
        }
    }
    units.sort_by(|left: &MeasuredUnit, right| left.key.cmp(&right.key));
    units.dedup_by(|later, kept| later.key == kept.key);
    Some(ReadStatistics {
        stage,
        units,
        sizes,
    })
}
