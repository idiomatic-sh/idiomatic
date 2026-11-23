//! The `args` object of each operator over spans and text, derived over the plan's own argument types (serde's remote
//! derive): a field is written under its kebab-case name, an optional one only when given.

use super::super::fields::schemas::{
    column_in, column_out, columns_in, columns_out, no_column, optional_in, optional_out, step_in, step_out, text_in,
    text_out, texts_in, texts_out, unwritten, whole_in, whole_out,
};
use ryo_values::domain::finish_plans::arguments::texts::types::{
    CaptureArguments, FitsRenderedArguments, OptionalColumn, PrefixesArguments, RenderSyntaxArguments,
    ReplaceArguments, SourceMatchesArguments, SpanBetweenArguments, SpanBoundsArguments,
};
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};
use super::super::ordered::schemas::OrderedMap;
use serde::{Deserialize, Serialize};

/// `SpanBoundsArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "SpanBoundsArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct SpanBoundsSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `span`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    span: ColumnName,
    /// `start-output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    start_output: ColumnName,
    /// `end-output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    end_output: ColumnName,
}

/// `SpanBetweenArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "SpanBetweenArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct SpanBetweenSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `start`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    start: ColumnName,
    /// `end`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    end: ColumnName,
    /// `output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    output: ColumnName,
}

/// `ReplaceArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "ReplaceArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct ReplaceSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `text`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    text: ColumnName,
    /// `pattern`.
    #[serde(serialize_with = "text_out", deserialize_with = "text_in")]
    pattern: LiteralText,
    /// `replacement`.
    #[serde(serialize_with = "text_out", deserialize_with = "text_in")]
    replacement: LiteralText,
    /// `output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    output: ColumnName,
}

/// `SourceMatchesArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "SourceMatchesArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct SourceMatchesSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `span`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    span: ColumnName,
    /// `base-line`.
    #[serde(serialize_with = "optional_out", deserialize_with = "optional_in", skip_serializing_if = "unwritten", default = "no_column")]
    base_line: OptionalColumn,
    /// `pattern`.
    #[serde(serialize_with = "text_out", deserialize_with = "text_in")]
    pattern: LiteralText,
    /// `output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    output: ColumnName,
    /// `line-output`.
    #[serde(serialize_with = "optional_out", deserialize_with = "optional_in", skip_serializing_if = "unwritten", default = "no_column")]
    line_output: OptionalColumn,
    /// `capture`.
    #[serde(serialize_with = "optional_out", deserialize_with = "optional_in", skip_serializing_if = "unwritten", default = "no_column")]
    capture: OptionalColumn,
    /// `capture-output`.
    #[serde(serialize_with = "optional_out", deserialize_with = "optional_in", skip_serializing_if = "unwritten", default = "no_column")]
    capture_output: OptionalColumn,
    /// `capture-text-output`.
    #[serde(serialize_with = "optional_out", deserialize_with = "optional_in", skip_serializing_if = "unwritten", default = "no_column")]
    capture_text_output: OptionalColumn,
    /// `capture-line-output`.
    #[serde(serialize_with = "optional_out", deserialize_with = "optional_in", skip_serializing_if = "unwritten", default = "no_column")]
    capture_line_output: OptionalColumn,
    /// `capture-texts`: group -> column, in order.
    #[serde(serialize_with = "pairs_out", deserialize_with = "pairs_in", skip_serializing_if = "Vec::is_empty", default)]
    capture_texts: Vec<(ColumnName, ColumnName)>,
}

/// `CaptureArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "CaptureArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct CaptureSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `path`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    path: ColumnName,
    /// `pattern`.
    #[serde(serialize_with = "text_out", deserialize_with = "text_in")]
    pattern: LiteralText,
    /// `captures`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    captures: Vec<ColumnName>,
    /// `separator`.
    #[serde(serialize_with = "optional_text_out", deserialize_with = "optional_text_in", skip_serializing_if = "Option::is_none", default)]
    separator: Option<LiteralText>,
}

/// Writes a given optional text.
fn optional_text_out<S: serde::Serializer>(text: &Option<LiteralText>, serializer: S) -> Result<S::Ok, S::Error> {
    match text {
        Some(text) => serializer.serialize_str(&text.0),
        None => serializer.serialize_none(),
    }
}

/// Reads a given optional text.
fn optional_text_in<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<LiteralText>, D::Error> {
    Ok(Some(text_in(deserializer)?))
}

/// Writes name pairs as an object, in order.
fn pairs_out<S: serde::Serializer>(pairs: &[(ColumnName, ColumnName)], serializer: S) -> Result<S::Ok, S::Error> {
    OrderedMap(pairs.iter().map(|(k, v)| (k.0.to_string(), v.0.to_string())).collect()).serialize(serializer)
}

/// Reads an object of names as pairs, in order.
fn pairs_in<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<(ColumnName, ColumnName)>, D::Error> {
    let map = OrderedMap::<String>::deserialize(deserializer)?;
    Ok(map.0.into_iter().map(|(k, v)| (ColumnName(k.into()), ColumnName(v.into()))).collect())
}

/// `FitsRenderedArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "FitsRenderedArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct FitsRenderedSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `texts`.
    #[serde(serialize_with = "columns_out", deserialize_with = "columns_in")]
    texts: Vec<ColumnName>,
    /// `literals`.
    #[serde(serialize_with = "texts_out", deserialize_with = "texts_in")]
    literals: Vec<LiteralText>,
    /// `span`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    span: ColumnName,
    /// `max-line`.
    #[serde(serialize_with = "whole_out", deserialize_with = "whole_in")]
    max_line: WholeNumber,
    /// `tab-width`.
    #[serde(serialize_with = "whole_out", deserialize_with = "whole_in")]
    tab_width: WholeNumber,
}

/// `RenderSyntaxArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "RenderSyntaxArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct RenderSyntaxSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `node`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    node: ColumnName,
    /// `context`.
    #[serde(serialize_with = "text_out", deserialize_with = "text_in")]
    context: LiteralText,
    /// `output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    output: ColumnName,
}

/// `PrefixesArguments` as JSON.
#[derive(Serialize, Deserialize)]
#[serde(remote = "PrefixesArguments", rename_all = "kebab-case")]
pub(in crate::outbound::plan_codec::adapters::json) struct PrefixesSchema {
    /// `from`.
    #[serde(serialize_with = "step_out", deserialize_with = "step_in")]
    from: StepNumber,
    /// `text`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    text: ColumnName,
    /// `separator`.
    #[serde(serialize_with = "text_out", deserialize_with = "text_in")]
    separator: LiteralText,
    /// `output`.
    #[serde(serialize_with = "column_out", deserialize_with = "column_in")]
    output: ColumnName,
}
