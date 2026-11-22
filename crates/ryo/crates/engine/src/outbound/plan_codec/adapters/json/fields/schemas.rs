//! How each kind of argument is written: a relation as `{"rel": n}`, a column or text as a string, a list of them as an
//! array, a number as a number, a direction, a bound or a side as its word, an output as an object of `[side, column]`.

use super::super::expressions::schemas::ConstantSchema;
use super::super::ordered::schemas::OrderedMap;
use crate::domain::finish_plans::arguments::groups::types::CountedRows;
use crate::domain::finish_plans::arguments::joins::types::{AsOfBound, OutputColumn};
use crate::domain::finish_plans::arguments::literals::types::{ConstantRow, ExtendedValue};
use crate::domain::finish_plans::arguments::texts::types::OptionalColumn;
use crate::domain::finish_plans::arguments::types::StepNumber;
use crate::domain::finish_plans::arguments::walks::types::EdgeKeys;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::{Direction, LiteralText, Side, WholeNumber};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A relation argument: the step it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::outbound::plan_codec::adapters::json) struct RelSchema {
    /// The step's number.
    pub rel: usize,
}

/// Writes a step as `{"rel": n}`.
pub(in crate::outbound::plan_codec::adapters::json) fn step_out<S: Serializer>(
    step: &StepNumber,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    RelSchema { rel: step.0 }.serialize(serializer)
}

/// Reads `{"rel": n}`.
pub(in crate::outbound::plan_codec::adapters::json) fn step_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<StepNumber, D::Error> {
    Ok(StepNumber(RelSchema::deserialize(deserializer)?.rel))
}

/// Writes a column.
pub(in crate::outbound::plan_codec::adapters::json) fn column_out<S: Serializer>(
    column: &ColumnName,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&column.0)
}

/// Reads a column.
pub(in crate::outbound::plan_codec::adapters::json) fn column_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<ColumnName, D::Error> {
    Ok(ColumnName(String::deserialize(deserializer)?.into()))
}

/// Writes columns.
pub(in crate::outbound::plan_codec::adapters::json) fn columns_out<S: Serializer>(
    columns: &[ColumnName],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(columns.iter().map(|column| &*column.0))
}

/// Reads columns.
pub(in crate::outbound::plan_codec::adapters::json) fn columns_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ColumnName>, D::Error> {
    Ok(Vec::<String>::deserialize(deserializer)?
        .into_iter()
        .map(|column| ColumnName(column.into()))
        .collect())
}

/// Writes text.
pub(in crate::outbound::plan_codec::adapters::json) fn text_out<S: Serializer>(
    text: &LiteralText,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&text.0)
}

/// Reads text.
pub(in crate::outbound::plan_codec::adapters::json) fn text_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<LiteralText, D::Error> {
    Ok(LiteralText(String::deserialize(deserializer)?.into()))
}

/// Writes texts.
pub(in crate::outbound::plan_codec::adapters::json) fn texts_out<S: Serializer>(
    texts: &[LiteralText],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(texts.iter().map(|text| &*text.0))
}

/// Reads texts.
pub(in crate::outbound::plan_codec::adapters::json) fn texts_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<LiteralText>, D::Error> {
    Ok(Vec::<String>::deserialize(deserializer)?
        .into_iter()
        .map(|text| LiteralText(text.into()))
        .collect())
}

/// Writes a whole number.
pub(in crate::outbound::plan_codec::adapters::json) fn whole_out<S: Serializer>(
    number: &WholeNumber,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_i64(number.0)
}

/// Reads a whole number.
pub(in crate::outbound::plan_codec::adapters::json) fn whole_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<WholeNumber, D::Error> {
    Ok(WholeNumber(i64::deserialize(deserializer)?))
}

/// Whether an optional column is not given: then it is not written.
pub(in crate::outbound::plan_codec::adapters::json) fn unwritten(column: &OptionalColumn) -> bool {
    *column == OptionalColumn::Unwritten
}

/// An optional column that is not given.
pub(in crate::outbound::plan_codec::adapters::json) fn no_column() -> OptionalColumn {
    OptionalColumn::Unwritten
}

/// Writes a given optional column.
pub(in crate::outbound::plan_codec::adapters::json) fn optional_out<S: Serializer>(
    column: &OptionalColumn,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match column {
        OptionalColumn::Written(name) => serializer.serialize_str(&name.0),
        OptionalColumn::Unwritten => serializer.serialize_none(),
    }
}

/// Reads a given optional column.
pub(in crate::outbound::plan_codec::adapters::json) fn optional_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<OptionalColumn, D::Error> {
    Ok(OptionalColumn::Written(column_in(deserializer)?))
}

/// Whether a count counts every row: then `distinct` is not written.
pub(in crate::outbound::plan_codec::adapters::json) fn every_row(counted: &CountedRows) -> bool {
    *counted == CountedRows::Every
}

/// A count of every row.
pub(in crate::outbound::plan_codec::adapters::json) fn all_rows() -> CountedRows {
    CountedRows::Every
}

/// Writes `distinct`.
pub(in crate::outbound::plan_codec::adapters::json) fn distinct_out<S: Serializer>(
    counted: &CountedRows,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match counted {
        CountedRows::Distinct(column) => serializer.serialize_str(&column.0),
        CountedRows::Every => serializer.serialize_none(),
    }
}

/// Reads `distinct`.
pub(in crate::outbound::plan_codec::adapters::json) fn distinct_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<CountedRows, D::Error> {
    Ok(CountedRows::Distinct(column_in(deserializer)?))
}

/// Whether an edge walk has no keys: then they are not written.
pub(in crate::outbound::plan_codec::adapters::json) fn no_keys(keys: &EdgeKeys) -> bool {
    *keys == EdgeKeys::Unwritten
}

/// An edge walk with no keys.
pub(in crate::outbound::plan_codec::adapters::json) fn unkeyed() -> EdgeKeys {
    EdgeKeys::Unwritten
}

/// Writes an edge walk's keys.
pub(in crate::outbound::plan_codec::adapters::json) fn keys_out<S: Serializer>(
    keys: &EdgeKeys,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match keys {
        EdgeKeys::Written(columns) => columns_out(columns, serializer),
        EdgeKeys::Unwritten => serializer.serialize_none(),
    }
}

/// Reads an edge walk's keys.
pub(in crate::outbound::plan_codec::adapters::json) fn keys_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<EdgeKeys, D::Error> {
    Ok(EdgeKeys::Written(columns_in(deserializer)?))
}

/// An axis direction as written.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DirectionSchema {
    /// `parent`.
    Parent,
    /// `child`.
    Child,
    /// `ancestor`.
    Ancestor,
    /// `descendant`.
    Descendant,
    /// `nearest-ancestor`.
    NearestAncestor,
    /// `deep`.
    Deep,
    /// `previous-sibling`.
    PreviousSibling,
    /// `next-sibling`.
    NextSibling,
}

/// Writes a direction.
pub(in crate::outbound::plan_codec::adapters::json) fn direction_out<S: Serializer>(
    direction: &Direction,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match direction {
        Direction::Parent => DirectionSchema::Parent,
        Direction::Child => DirectionSchema::Child,
        Direction::Ancestor => DirectionSchema::Ancestor,
        Direction::Descendant => DirectionSchema::Descendant,
        Direction::NearestAncestor => DirectionSchema::NearestAncestor,
        Direction::Deep => DirectionSchema::Deep,
        Direction::PreviousSibling => DirectionSchema::PreviousSibling,
        Direction::NextSibling => DirectionSchema::NextSibling,
    }
    .serialize(serializer)
}

/// Reads a direction.
pub(in crate::outbound::plan_codec::adapters::json) fn direction_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Direction, D::Error> {
    Ok(match DirectionSchema::deserialize(deserializer)? {
        DirectionSchema::Parent => Direction::Parent,
        DirectionSchema::Child => Direction::Child,
        DirectionSchema::Ancestor => Direction::Ancestor,
        DirectionSchema::Descendant => Direction::Descendant,
        DirectionSchema::NearestAncestor => Direction::NearestAncestor,
        DirectionSchema::Deep => Direction::Deep,
        DirectionSchema::PreviousSibling => Direction::PreviousSibling,
        DirectionSchema::NextSibling => Direction::NextSibling,
    })
}

/// An `as-of` bound as written.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum BoundSchema {
    /// `at-or-before`.
    AtOrBefore,
    /// `before`.
    Before,
    /// `at-or-after`.
    AtOrAfter,
    /// `after`.
    After,
}

/// Writes a bound.
pub(in crate::outbound::plan_codec::adapters::json) fn bound_out<S: Serializer>(
    bound: &AsOfBound,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match bound {
        AsOfBound::AtOrBefore => BoundSchema::AtOrBefore,
        AsOfBound::Before => BoundSchema::Before,
        AsOfBound::AtOrAfter => BoundSchema::AtOrAfter,
        AsOfBound::After => BoundSchema::After,
    }
    .serialize(serializer)
}

/// Reads a bound.
pub(in crate::outbound::plan_codec::adapters::json) fn bound_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<AsOfBound, D::Error> {
    Ok(match BoundSchema::deserialize(deserializer)? {
        BoundSchema::AtOrBefore => AsOfBound::AtOrBefore,
        BoundSchema::Before => AsOfBound::Before,
        BoundSchema::AtOrAfter => AsOfBound::AtOrAfter,
        BoundSchema::After => AsOfBound::After,
    })
}

/// A side as written.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SideSchema {
    /// `left`.
    Left,
    /// `right`.
    Right,
}

/// Writes an ordered step's output: `{name: [side, column], ..}`.
pub(in crate::outbound::plan_codec::adapters::json) fn output_out<S: Serializer>(
    output: &[OutputColumn],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    OrderedMap(
        output
            .iter()
            .map(|column| {
                let side = match column.side {
                    Side::Left => SideSchema::Left,
                    Side::Right => SideSchema::Right,
                };
                (column.name.0.to_string(), (side, column.column.0.to_string()))
            })
            .collect(),
    )
    .serialize(serializer)
}

/// Reads an ordered step's output.
pub(in crate::outbound::plan_codec::adapters::json) fn output_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<OutputColumn>, D::Error> {
    Ok(OrderedMap::<(SideSchema, String)>::deserialize(deserializer)?
        .0
        .into_iter()
        .map(|(name, (side, column))| OutputColumn {
            name: ColumnName(name.into()),
            side: match side {
                SideSchema::Left => Side::Left,
                SideSchema::Right => Side::Right,
            },
            column: ColumnName(column.into()),
        })
        .collect())
}

/// Writes an `extend`'s values: `{name: constant, ..}`.
pub(in crate::outbound::plan_codec::adapters::json) fn values_out<S: Serializer>(
    values: &[ExtendedValue],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    OrderedMap(
        values
            .iter()
            .map(|value| (value.name.0.to_string(), ConstantSchema::from(&value.value)))
            .collect(),
    )
    .serialize(serializer)
}

/// Reads an `extend`'s values.
pub(in crate::outbound::plan_codec::adapters::json) fn values_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ExtendedValue>, D::Error> {
    Ok(OrderedMap::<ConstantSchema>::deserialize(deserializer)?
        .0
        .into_iter()
        .map(|(name, value)| ExtendedValue {
            name: ColumnName(name.into()),
            value: value.into(),
        })
        .collect())
}

/// Writes a constant table: an array of rows, each an array of texts.
pub(in crate::outbound::plan_codec::adapters::json) fn rows_out<S: Serializer>(
    rows: &[ConstantRow],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let written: Vec<Vec<&str>> = rows
        .iter()
        .map(|row| row.cells.iter().map(|cell| &*cell.0).collect())
        .collect();
    written.serialize(serializer)
}

/// Reads a constant table.
pub(in crate::outbound::plan_codec::adapters::json) fn rows_in<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ConstantRow>, D::Error> {
    Ok(Vec::<Vec<String>>::deserialize(deserializer)?
        .into_iter()
        .map(|row| ConstantRow {
            cells: row.into_iter().map(|cell| LiteralText(cell.into())).collect(),
        })
        .collect())
}
