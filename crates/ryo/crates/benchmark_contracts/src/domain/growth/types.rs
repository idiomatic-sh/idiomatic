//! A language's growth table, as its `growth.toml` writes it: the rules, the sizes, the frames and the operations a
//! growth run expands into programs.
use std::collections::BTreeMap;

/// How many repetitions (or statements) a program holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrowthSize(pub usize);

/// The name the program's one source file is written under, the language's (`case.py`, `case.ts`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFileName(pub String);

/// One text of a growth template; `{i}` stands for the repetition's index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateText(pub String);

/// The name of a frame or an operation, as a shape names it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemplateName(pub String);

/// How many times a segment repeats: a fixed count plus a count per unit of the program's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Repetitions {
    /// The repetitions at any size.
    pub fixed: GrowthSize,
    /// The repetitions each unit of the size adds.
    pub per_size: GrowthSize,
}

/// A run of lines repeated: repetition `i` writes `lines[i % lines.len()]` with `{i}` replaced by `i`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowthSegment {
    /// The texts cycled through, one per repetition.
    pub lines: Vec<TemplateText>,
    /// How many repetitions the segment writes.
    pub repetitions: Repetitions,
}

/// Where an operation's segments sit: the text before them, the indentation of each of their lines, and the text after.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowthFrame {
    /// The text before the segments.
    pub head: TemplateText,
    /// What each line of the segments is indented with.
    pub indent: TemplateText,
    /// The text after the segments.
    pub tail: TemplateText,
}

/// One operation grown: its segments, in each frame it names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowthOperation {
    /// The operation as a shape names it.
    pub name: TemplateName,
    /// The frames it is grown in.
    pub frames: Vec<TemplateName>,
    /// Its segments, in order.
    pub segments: Vec<GrowthSegment>,
}

/// A rule grown, and the profile directory, under the linter's bench, that registers its query modules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowthRule {
    /// The rule linted.
    pub rule: ryo::linter::RuleId,
    /// Its profile directory, relative to the linter's bench.
    pub profile: std::path::PathBuf,
}

/// A language's growth table, as `languages/<lang>/benchmarks/growth.toml` writes it: every rule over every operation in
/// every frame it names, at each whole size and its half.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowthTable {
    /// The program's source file name.
    pub file: SourceFileName,
    /// The rules grown.
    pub rules: Vec<GrowthRule>,
    /// The whole sizes; each shape's half is half of one.
    pub sizes: Vec<GrowthSize>,
    /// The frames, by name.
    pub frames: BTreeMap<TemplateName, GrowthFrame>,
    /// The operations.
    pub operations: Vec<GrowthOperation>,
}
