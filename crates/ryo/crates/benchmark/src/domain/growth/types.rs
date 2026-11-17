//! The plan of a growth run, its bounds and what it observed.
use ryo_benchmark_contracts::domain::growth::types::{GrowthSize, SourceFileName};
use ryo_benchmark_contracts::domain::measurements::types::ResidentBytes;
use ryo_benchmark_contracts::domain::measurements::types::{DoublingRatio, GrowthShape};
use std::collections::BTreeMap;

/// A shape's program at one size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrownProgram(pub String);

/// A query module the rule imports, by its name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GrowthModule(pub String);

/// The query modules a rule needs beside the program.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleModules(pub BTreeMap<GrowthModule, std::path::PathBuf>);

/// The program of one shape at the half and the whole size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapePrograms {
    /// The rule linted.
    pub rule: ryo::linter::RuleId,
    /// The modules the rule imports.
    pub modules: RuleModules,
    /// The half-size program.
    pub half: GrownProgram,
    /// The whole-size program.
    pub whole: GrownProgram,
    /// The whole size.
    pub size: GrowthSize,
}

/// The CPU time under which a half-size time counts as this much, so a fixed startup cost does not decide a ratio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuFloor(pub std::time::Duration);

/// How much memory a grown program may hold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemoryBound {
    /// What every run may hold before its program counts (Ruff: 160 MB).
    pub base: ResidentBytes,
    /// What each thousand of the whole size may add (Ruff: 100 MB).
    pub per_thousand: ResidentBytes,
    /// The most the peak may grow when the program doubles (Ruff: 2.4).
    pub doubling: DoublingRatio,
    /// The half-size peak under which it counts as this much (Ruff: 120 MB).
    pub floor: ResidentBytes,
}

/// Every shape a language grows its rules over, with the one floor and the memory bound.
#[derive(Clone, Debug)]
pub struct GrowthPlan {
    /// The CPU floor (Ruff's 0.3 s; every language's).
    pub floor: CpuFloor,
    /// The memory bound of every shape.
    pub memory: MemoryBound,
    /// The program's source file name.
    pub file: SourceFileName,
    /// The request each program is linted with: one source file, its root replaced per scene.
    pub request: ryo_benchmark_contracts::domain::requests::types::WorkerRequest,
    /// The shapes.
    pub shapes: BTreeMap<GrowthShape, ShapePrograms>,
}

/// Where a language's growth is read from, and what its programs are linted with.
#[derive(Clone, Copy, Debug)]
pub struct GrowthSource<'a> {
    /// The linter's bench directory: the rules' profiles and query modules are below it.
    pub bench: &'a std::path::Path,
    /// The bench's rules directory.
    pub rules: &'a std::path::Path,
    /// The language's `growth.toml`.
    pub table: &'a std::path::Path,
    /// The parser configuration every program is linted with.
    pub parser: &'a ryo_benchmark_contracts::domain::requests::types::ParserSelection,
}

/// Whether a language grows its rules, and where from.
#[derive(Clone, Copy, Debug)]
pub enum CorpusGrowth<'a> {
    /// The language's growth table and what its programs are linted with.
    Grown(GrowthSource<'a>),
    /// The language grows no rule.
    Ungrown,
}
