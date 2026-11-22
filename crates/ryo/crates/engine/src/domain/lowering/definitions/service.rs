//! The services of the `definitions` part: fresh names, the pack's constants, a definition made from a step's branches, a use
//! of one, and a pattern asked of one. A definition is lowered once, whoever reads it; nothing below a use is ever copied.

use super::types::{
    AskSite, AskedColumns, DefinitionDraft, Drafts, Flattened, PatternDraft, PatternPlace,
    WaitingPattern,
};
use crate::domain::code::types::ShapeChoice;
use crate::domain::lowering::emptiness::branches::service::{branches_need, with_need};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile, TextNumber,
};
use crate::domain::lowering::nodes::types::{NodeHome, StepShape};
use crate::domain::steps::types::{
    ColumnMask, ConstantText, DefinitionId, RegexId, Register, RegisterCount, StepRegex,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{CellKind, KindBits};
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::query_plans::types::NodeDigest;
use std::sync::OnceLock;

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

/// The number of a constant text: its base word. The run's base words are the empty text (word 0), the language's fixed
/// names at their places, then the pack's constant texts in their order: so the empty text and a fixed name are their own
/// words and never enter the pack's texts, and every other text enters them once. The pack's texts are thereby distinct,
/// non-empty and no fixed name, which is what the numbering needs.
pub(crate) fn text(
    drafts: &mut Drafts,
    kinds: &KindVocabulary,
    written: &LiteralText,
) -> TextNumber {
    if let Some(number) = drafts.text_numbers.get(written) {
        return *number;
    }
    let fixed = kinds.fixed.iter().position(|name| name.0 == written.0);
    let word = match (written.0.is_empty(), fixed) {
        (true, _) => 0,
        (false, Some(place)) => place.saturating_add(1),
        (false, None) => {
            drafts.texts.push(ConstantText(written.0.clone()));
            kinds.fixed.len().saturating_add(drafts.texts.len())
        }
    };
    let number = TextNumber(u32::try_from(word).unwrap_or(u32::MAX));
    let _new = drafts.text_numbers.insert(written.clone(), number);
    let _named = drafts
        .written
        .insert(number, ConstantText(written.0.clone()));
    number
}

/// The base word of a constant text: its number.
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
) -> DefinitionId {
    if let Some(definition) = drafts.defined.get(&node) {
        return *definition;
    }
    let definition = DefinitionId(u32::try_from(drafts.definitions.len()).unwrap_or(u32::MAX));
    let formals = shape.0.iter().map(|_| fresh_register(drafts)).collect();
    // What the operations say of the step, and what its branches themselves need: a literal a branch holds a column to is
    // carried into the need, through the definitions the branch reads.
    let need = branches_need(drafts, &flattened.branches);
    drafts.definitions.push(DefinitionDraft {
        node,
        home,
        shape,
        branches: flattened.branches,
        emptiness: with_need(flattened.emptiness, need),
        keyless: flattened.keyless,
        formals,
        rows: OnceLock::new(),
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
    let serial = drafts.asking.serial;
    let site = drafts.asking.atom.as_ref().map(|(atom, rows)| AskSite {
        prefix: drafts.asking.prefix.clone(),
        atom: atom.clone(),
        rows: *rows,
        origin: drafts.asking.origin,
        serial,
    });
    if site.is_some() {
        drafts.asking.serial = serial.saturating_add(1);
    }
    let Some(draft) = drafts.definitions.get_mut(place) else {
        return;
    };
    let known = draft
        .patterns
        .iter_mut()
        .find(|pattern| pattern.asked.bound == *bound && pattern.asked.collected == *collected);
    if let Some(pattern) = known {
        match site {
            Some(site) => pattern.sites.push(site),
            None => pattern.opaque = true,
        }
        return;
    }
    draft.patterns.push(PatternDraft {
        asked: AskedColumns {
            bound: bound.clone(),
            collected: collected.clone(),
        },
        form: None,
        shape: ShapeChoice::Decided,
        opaque: site.is_none(),
        sites: site.into_iter().collect(),
    });
    drafts.waiting.push(WaitingPattern {
        definition,
        pattern: PatternPlace(draft.patterns.len().saturating_sub(1)),
    });
}

/// The kind a written name is in a language's vocabulary: its place there. None when the vocabulary does not hold the name,
/// and for a place past the sixteen bits a kind is counted in.
#[must_use]
pub(crate) fn kind_bits(kinds: &KindVocabulary, written: &ConstantText) -> Option<KindBits> {
    let place = kinds.kinds.iter().position(|kind| kind.0 == written.0)?;
    match u16::try_from(place) {
        Ok(bits) => Some(KindBits(bits)),
        Err(_) => None,
    }
}
