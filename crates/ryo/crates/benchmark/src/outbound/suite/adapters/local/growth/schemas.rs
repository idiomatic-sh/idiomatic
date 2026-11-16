//! The shape of a language's growth table: the program's file, the rules and their profiles, the whole sizes, the frames
//! and the operations grown in them.
use ryo_benchmark_contracts::domain::growth::types::{
    GrowthFrame, GrowthOperation, GrowthRule, GrowthSegment, GrowthSize, GrowthTable,
    Repetitions, SourceFileName, TemplateName, TemplateText,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Deserialize;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// The whole table.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrowthTableSchema {
    /// The program's source file name.
    file: String,
    /// The whole sizes.
    sizes: Vec<usize>,
    /// The rules grown.
    rules: Vec<GrowthRuleSchema>,
    /// The frames, by name.
    frames: BTreeMap<String, GrowthFrameSchema>,
    /// The operations.
    operations: Vec<GrowthOperationSchema>,
}

/// One rule and its profile directory.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrowthRuleSchema {
    /// The rule name.
    rule: String,
    /// Its profile directory, relative to the linter's bench.
    profile: String,
}

/// One frame.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrowthFrameSchema {
    /// The text before the segments.
    head: String,
    /// The indentation of each segment line.
    indent: String,
    /// The text after the segments.
    tail: String,
}

/// One operation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrowthOperationSchema {
    /// The operation's name.
    name: String,
    /// The frames it is grown in.
    frames: Vec<String>,
    /// Its segments.
    segments: Vec<GrowthSegmentSchema>,
}

/// One segment.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct GrowthSegmentSchema {
    /// The texts cycled through.
    lines: Vec<String>,
    /// The repetitions at any size.
    fixed: usize,
    /// The repetitions per unit of the size.
    per_size: usize,
}

/// Decode a growth table, refusing one that grows nothing, an odd or zero size, a segment with no line, or an operation
/// in no frame or in a frame the table does not define.
pub(in super::super) fn growth(text: &str) -> Result<GrowthTable, Error> {
    let wire: GrowthTableSchema = toml::from_str(text)?;
    if wire.rules.is_empty() || wire.sizes.is_empty() || wire.operations.is_empty() {
        return Err(Error::Invalid(super::super::constants::GROWTH_EMPTY.into()));
    }
    if wire.sizes.iter().any(|size| *size == 0 || size % 2 != 0) {
        return Err(Error::Invalid(super::super::constants::GROWTH_ODD_SIZE.into()));
    }
    let frames: BTreeMap<TemplateName, GrowthFrame> = wire
        .frames
        .into_iter()
        .map(|(name, frame)| {
            (
                TemplateName(name),
                GrowthFrame {
                    head: TemplateText(frame.head),
                    indent: TemplateText(frame.indent),
                    tail: TemplateText(frame.tail),
                },
            )
        })
        .collect();
    let mut operations = Vec::new();
    for operation in wire.operations {
        let named: Vec<TemplateName> = operation.frames.into_iter().map(TemplateName).collect();
        if named.is_empty()
            || named.iter().any(|frame| !frames.contains_key(frame))
            || operation.segments.iter().any(|segment| segment.lines.is_empty())
        {
            return Err(Error::Invalid(format!(
                "growth operation `{}`: {}",
                operation.name,
                super::super::constants::GROWTH_OPERATION
            )));
        }
        operations.push(GrowthOperation {
            name: TemplateName(operation.name),
            frames: named,
            segments: operation
                .segments
                .into_iter()
                .map(|segment| GrowthSegment {
                    lines: segment.lines.into_iter().map(TemplateText).collect(),
                    repetitions: Repetitions {
                        fixed: GrowthSize(segment.fixed),
                        per_size: GrowthSize(segment.per_size),
                    },
                })
                .collect(),
        });
    }
    Ok(GrowthTable {
        file: SourceFileName(wire.file),
        rules: wire
            .rules
            .into_iter()
            .map(|rule| {
                Ok(GrowthRule {
                    rule: ryo::linter::RuleId::try_from(rule.rule.as_str())
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                    profile: rule.profile.into(),
                })
            })
            .collect::<Result<_, Error>>()?,
        sizes: wire.sizes.into_iter().map(GrowthSize).collect(),
        frames,
        operations,
    })
}
