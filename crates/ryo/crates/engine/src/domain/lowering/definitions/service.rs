//! The services of the `definitions` part: fresh names, the pack's constants, a definition made from a step's branches, a use
//! of one, and a pattern asked of one. A definition is lowered once, whoever reads it; nothing below a use is ever copied.

use super::types::{
    DefinitionDraft, Drafts, Flattened, PatternDraft, PatternPlace, RowEstimate, WaitingPattern,
};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile, TextNumber,
};
use crate::domain::lowering::nodes::types::{NodeHome, StepShape};
use crate::domain::code::types::ShapeChoice;
use crate::domain::steps::types::{
    ColumnMask, ConstantText, DefinitionId, RegexId, Register, RegisterCount, StepRegex,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::query_plans::types::NodeDigest;

/// A variable no branch names yet.
pub(crate) fn fresh_variable(drafts: &mut Drafts) -> FlatVariable {
    let variable = drafts.variables;
    drafts.variables = FlatVariable(variable.0.saturating_add(1));
    variable
}

/// A register no program names yet.
pub(crate) fn fresh_register(drafts: &mut Drafts) -> Register {
    let register = Register(drafts.registers.0);
    drafts.registers = RegisterCount(register.0.saturating_add(1));
    register
}

/// The number of a constant text: each text once, numbered from zero in the order first written.
pub(crate) fn text(drafts: &mut Drafts, written: &LiteralText) -> TextNumber {
    if let Some(number) = drafts.text_numbers.get(written) {
        return *number;
    }
    let number = TextNumber(u32::try_from(drafts.texts.len()).unwrap_or(u32::MAX));
    drafts.texts.push(ConstantText(written.0.clone()));
    let _new = drafts.text_numbers.insert(written.clone(), number);
    number
}

/// The base word of a constant text: the run's base words code the pack's texts from zero in the pack's order.
#[must_use]
pub(crate) fn text_word(number: TextNumber) -> CellWord {
    CellWord(u64::from(number.0))
}

/// The number of a regular expression: each source once.
pub(crate) fn regex(drafts: &mut Drafts, source: &LiteralText) -> RegexId {
    if let Some(number) = drafts.regex_numbers.get(source) {
        return *number;
    }
    let number = RegexId(u32::try_from(drafts.regexes.len()).unwrap_or(u32::MAX));
    drafts.regexes.push(StepRegex(source.0.clone()));
    let _new = drafts.regex_numbers.insert(source.clone(), number);
    number
}

/// The definition of a merged plan node, made from its flattening the first time and found after: lowered once for the pack.
pub(crate) fn defined(
    drafts: &mut Drafts,
    node: NodeDigest,
    home: NodeHome,
    shape: StepShape,
    flattened: Flattened,
    rows: RowEstimate,
) -> DefinitionId {
    if let Some(definition) = drafts.defined.get(&node) {
        return *definition;
    }
    let definition = DefinitionId(u32::try_from(drafts.definitions.len()).unwrap_or(u32::MAX));
    let formals = shape.0.iter().map(|_| fresh_register(drafts)).collect();
    drafts.definitions.push(DefinitionDraft {
        node,
        home,
        shape,
        branches: flattened.branches,
        emptiness: flattened.emptiness,
        keyless: flattened.keyless,
        formals,
        rows,
        patterns: Vec::new(),
        table: None,
    });
    let _new = drafts.defined.insert(node, definition);
    definition
}

/// A definition's draft.
#[must_use]
pub(crate) fn draft(drafts: &Drafts, definition: DefinitionId) -> Option<&DefinitionDraft> {
    drafts
        .definitions
        .get(usize::try_from(definition.0).unwrap_or(usize::MAX))
}

/// One term per column of a shape: the file for a file column, a fresh variable for any other.
pub(crate) fn fresh_cells(drafts: &mut Drafts, shape: &StepShape) -> Vec<FlatTerm> {
    shape
        .0
        .iter()
        .map(|kind| match kind {
            CellKind::File => FlatTerm::File,
            CellKind::Node
            | CellKind::Symbol
            | CellKind::Span
            | CellKind::Number
            | CellKind::Flag => FlatTerm::Variable(fresh_variable(drafts)),
        })
        .collect()
}

/// A use of a definition as one branch of one atom: its row is fresh variables, and the use's whole correlation is its cells.
pub(crate) fn use_branch(drafts: &mut Drafts, definition: DefinitionId) -> FlatBranch {
    let shape = draft(drafts, definition)
        .map(|draft| draft.shape.clone())
        .unwrap_or_default();
    let cells = fresh_cells(drafts, &shape);
    FlatBranch {
        atoms: vec![FlatAtom::Rows {
            rows: AtomRows::Definition(definition),
            cells: cells.clone(),
            per_file: RowsPerFile::Many,
        }],
        row: cells,
    }
}

/// Asks a pattern of a definition: the first time it is asked it waits to be placed. Answers nothing new for a pattern
/// already asked, so a pattern is planned once across the pack.
pub(crate) fn asked(
    drafts: &mut Drafts,
    definition: DefinitionId,
    bound: &ColumnMask,
    collected: &ColumnMask,
) {
    let place = usize::try_from(definition.0).unwrap_or(usize::MAX);
    let Some(draft) = drafts.definitions.get_mut(place) else {
        return;
    };
    let known = draft
        .patterns
        .iter()
        .any(|pattern| pattern.bound == *bound && pattern.collected == *collected);
    if known {
        return;
    }
    draft.patterns.push(PatternDraft {
        bound: bound.clone(),
        collected: collected.clone(),
        form: None,
        shape: ShapeChoice::Decided,
    });
    drafts.waiting.push(WaitingPattern {
        definition,
        pattern: PatternPlace(draft.patterns.len().saturating_sub(1)),
    });
}
