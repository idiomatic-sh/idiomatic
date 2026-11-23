//! Each operator's `args` object, derived over the plan's own argument types (serde's remote derive): a field is written
//! under its kebab-case name, an optional one only when given.

use super::super::fields::schemas::{
    all_rows, bound_in, bound_out, column_in, column_out, columns_in, columns_out, direction_in,
    direction_out, distinct_in, distinct_out, every_row, keys_in, keys_out, no_keys, output_in,
    output_out, rows_in, rows_out, step_in, step_out, unkeyed, values_in, values_out,
};
use ryo_values::domain::finish_plans::arguments::groups::types::{
    CountedRows, GroupCountArguments, GroupOrderArguments, GroupOrdinalArguments,
    ProjectArguments,
};
use ryo_values::domain::finish_plans::arguments::joins::types::{
    AntijoinArguments, AsOfArguments, AsOfBound, OrderedMatchArguments, OutputColumn,
    UnionArguments,
};
use ryo_values::domain::finish_plans::arguments::literals::types::{
    ConstantRow, ExtendArguments, ExtendedValue, RowsArguments,
};
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::arguments::walks::types::{
    AxisArguments, EdgeArguments, EdgeKeys, OrderedDfsArguments, SubtreeClassArguments,
};
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::Direction;
use serde::{Deserialize, Serialize};

/// `UnionArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "UnionArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct UnionSchema {
    /// `left`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    left: StepNumber,
    /// `right`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    right: StepNumber,
}

/// `AntijoinArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "AntijoinArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct AntijoinSchema {
    /// `left`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    left: StepNumber,
    /// `right`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    right: StepNumber,
    /// `left-on`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    left_on: Vec<ColumnName>,
    /// `right-on`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    right_on: Vec<ColumnName>,
}

/// `AsOfArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "AsOfArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct AsOfSchema {
    /// `left`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    left: StepNumber,
    /// `right`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    right: StepNumber,
    /// `left-on`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    left_on: Vec<ColumnName>,
    /// `right-on`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    right_on: Vec<ColumnName>,
    /// `left-order`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    left_order: ColumnName,
    /// `right-order`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    right_order: ColumnName,
    /// `bound`.
    #[serde(serialize_with = "bound_out", deserialize_with = "bound_in")]
    bound: AsOfBound,
    /// `output`.
    #[serde(serialize_with = "output_out", deserialize_with = "output_in")]
    output: Vec<OutputColumn>,
}

/// `OrderedMatchArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "OrderedMatchArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct OrderedMatchSchema {
    /// `left`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    left: StepNumber,
    /// `right`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    right: StepNumber,
    /// `left-on`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    left_on: Vec<ColumnName>,
    /// `right-on`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    right_on: Vec<ColumnName>,
    /// `left-order`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    left_order: ColumnName,
    /// `right-order`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    right_order: ColumnName,
    /// `output`.
    #[serde(serialize_with = "output_out", deserialize_with = "output_in")]
    output: Vec<OutputColumn>,
}

/// `AxisArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "AxisArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct AxisSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `relation`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    relation: StepNumber,
    /// `direction`.
    #[serde(serialize_with = "direction_out", deserialize_with = "direction_in")]
    direction: Direction,
    /// `domain`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    domain: StepNumber,
}

/// `EdgeArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "EdgeArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct EdgeSchema {
    /// `edges`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    edges: StepNumber,
    /// `source`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    source: ColumnName,
    /// `destination`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    destination: ColumnName,
    /// `keys`.
    #[serde(serialize_with = "keys_out", deserialize_with = "keys_in", skip_serializing_if = "no_keys", default = "unkeyed")]
    keys: EdgeKeys,
}

/// `OrderedDfsArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "OrderedDfsArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct OrderedDfsSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `root`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    root: ColumnName,
    /// `edges`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    edges: StepNumber,
    /// `source`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    source: ColumnName,
    /// `destination`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    destination: ColumnName,
    /// `order`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    order: ColumnName,
    /// `edge`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    edge: ColumnName,
    /// `keys`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    keys: Vec<ColumnName>,
    /// `output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    output: ColumnName,
}

/// `SubtreeClassArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "SubtreeClassArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct SubtreeClassSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `relation`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    relation: StepNumber,
    /// `labels`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    labels: StepNumber,
    /// `leaves`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    leaves: StepNumber,
    /// `label`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    label: Vec<ColumnName>,
    /// `leaf-label`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    leaf_label: Vec<ColumnName>,
    /// `step`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    step: Vec<ColumnName>,
    /// `output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    output: ColumnName,
    /// `id`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    id: ColumnName,
    /// `parent`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    parent: ColumnName,
    /// `child`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    child: ColumnName,
}

/// `GroupCountArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "GroupCountArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct GroupCountSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `keys`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    keys: Vec<ColumnName>,
    /// `count`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    count: ColumnName,
    /// `distinct`.
    #[serde(serialize_with = "distinct_out", deserialize_with = "distinct_in", skip_serializing_if = "every_row", default = "all_rows")]
    distinct: CountedRows,
}

/// `GroupOrderArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "GroupOrderArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct GroupOrderSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `keys`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    keys: Vec<ColumnName>,
    /// `order`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    order: ColumnName,
}

/// `GroupOrdinalArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "GroupOrdinalArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct GroupOrdinalSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `keys`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    keys: Vec<ColumnName>,
    /// `order`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    order: ColumnName,
    /// `output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    output: ColumnName,
}

/// `ProjectArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "ProjectArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct ProjectSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `columns`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    columns: Vec<ColumnName>,
}

/// `ExtendArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "ExtendArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct ExtendSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `values`.
    #[serde(serialize_with = "values_out", deserialize_with = "values_in")]
    values: Vec<ExtendedValue>,
}

/// `RowsArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "RowsArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct RowsSchema {
    /// `rows`.
    #[serde(rename = "from", serialize_with = "rows_out", deserialize_with = "rows_in")]
    rows: Vec<ConstantRow>,
    /// `columns`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    columns: Vec<ColumnName>,
}
