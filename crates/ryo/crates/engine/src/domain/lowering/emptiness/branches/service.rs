//! The services of the `branches` part: what a file must hold for a branch to give a row, read off the branch itself. Each
//! of its rows atoms needs a row of its source; a cell held to a constant names the value the row holds in that column; and
//! a use of a definition with a cell held to a constant needs what the definition's own branches need with that column
//! held to it: a literal is carried through a definition into the need, and each reader of a shared definition has its own.

use crate::domain::lowered::types::{ByteLiteral, Emptiness, PathTest, RowTest, RowValue};
use crate::domain::lowering::emptiness::service::path_test;
use crate::domain::lowering::definitions::service::{draft, text_word};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::service::pins_of;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable,
};
use crate::domain::lowering::flattening::types::RowMeaning;
use crate::domain::steps::types::{CallMeaning, DefinitionId, Polarity, TestOperation};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::catalog::service::{fact_form, node_column, node_extent};
use ryo_relations::domain::relations::service::{columns, kind};
use ryo_relations::domain::relations::types::{CellKind, Column, FactForm, NodeColumn, NodeExtent, Relation};
use std::collections::HashMap;

/// How deep a constant is carried through definitions, and how many branches of a definition may be left that can give
/// a row with it: past either, the use needs the definition's own formula. Ten deep: a fragment's output is a chain
/// of definitions (a selection over a union over a view over the relation), and the relation whose column the
/// constant names lies at its end; at four a constant held in a reader's query was seen never to reach a relation
/// read six definitions down. The branches LEFT are counted, not the
/// definition's: a definition of many branches that each hold the column to its own constant (a union of a relation's
/// kinds) is where a reader's constant decides most, leaving the one branch that holds it.
const CARRIED: (usize, usize) = (10, 8);

/// SPIKE: whether the branches a need is read off are printed (`RYO_X_NEED_BRANCHES`, read once): what a branch holds
/// when its need is read, to see why a literal of the query is not in it.
fn traced() -> bool {
    static TRACED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACED.get_or_init(|| std::env::var_os("RYO_X_NEED_BRANCHES").is_some())
}

/// The constant a cell is held to: its own, or the one an equality pins its variable to.
fn held(pins: &HashMap<FlatVariable, FlatTerm>, cell: FlatTerm) -> Option<FlatTerm> {
    match cell {
        FlatTerm::Text(_) | FlatTerm::Number(_) => Some(cell),
        FlatTerm::Variable(variable) => pins.get(&variable).copied(),
        FlatTerm::File => None,
    }
}

/// The word a constant is in a column of a kind: a text in a symbol column, a number in a number column. None for any
/// other pair: nothing is said of a value the row may hold in another form.
fn word(kind: CellKind, constant: FlatTerm) -> Option<CellWord> {
    match (kind, constant) {
        (CellKind::Symbol, FlatTerm::Text(number)) => Some(text_word(number)),
        // A number in a symbol column is a closed set's word, held as its own symbol (`tables::literals`).
        (CellKind::Number | CellKind::Symbol, FlatTerm::Number(number)) => {
            Some(CellWord(number.0.cast_unsigned()))
        }
        (
            CellKind::Node
            | CellKind::Symbol
            | CellKind::Span
            | CellKind::Number
            | CellKind::Flag
            | CellKind::File,
            FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::Variable(_) | FlatTerm::File,
        ) => None,
    }
}

/// What a row of a relation with these cells needs: for the node table, a node of the kind or a child in the field a cell
/// is held to; for a filled relation, a row, with each value a cell is held to.
fn relation_need(
    relation: Relation,
    cells: &[Option<FlatTerm>],
    pins: &HashMap<FlatVariable, FlatTerm>,
    parts: &mut Vec<Emptiness>,
) {
    let filled = matches!(fact_form(relation), FactForm::Filled(_));
    let mut valued = false;
    for (column, cell) in columns(relation).into_iter().zip(cells) {
        // A node relation's row is asked by its kind, its field, or the token text it holds: each a set of the file's
        // read once a file.
        let decided = match (filled, node_column(column)) {
            (true, _) | (false, Some(NodeColumn::Kind | NodeColumn::Field | NodeColumn::Text)) => true,
            (false, _) => false,
        };
        let value = cell
            .and_then(|cell| held(pins, cell))
            .and_then(|constant| word(kind(column), constant));
        if let (true, Some(value)) = (decided, value) {
            parts.push(Emptiness::Row(relation, Some(RowValue { column, value })));
            valued = true;
        }
    }
    if filled && !valued {
        parts.push(Emptiness::Row(relation, None));
    }
}

/// What a row of a definition with these cells needs: the definition's own formula; or, with a cell held to a constant,
/// what some branch of it needs with that column held to it.
fn definition_need(
    drafts: &Drafts,
    definition: DefinitionId,
    cells: &[Option<FlatTerm>],
    pins: &HashMap<FlatVariable, FlatTerm>,
    depth: usize,
) -> Emptiness {
    let constants: Vec<(usize, FlatTerm)> = cells
        .iter()
        .enumerate()
        .filter_map(|(place, cell)| {
            cell.and_then(|cell| held(pins, cell))
                .map(|held| (place, held))
        })
        .collect();
    let branches = draft(drafts, definition).map(|draft| &draft.branches);
    if traced() {
        eprintln!(
            "ryo x: need-use definition={} depth={depth} constants={constants:?} cells={cells:?} branches={}",
            definition.0,
            branches.map_or(0, Vec::len)
        );
    }
    let Some(branches) = branches.filter(|_| !constants.is_empty() && depth < CARRIED.0) else {
        return Emptiness::Definition(definition);
    };
    let mut some = Vec::new();
    for branch in branches {
        let mut carried = HashMap::new();
        let mut possible = true;
        for (place, constant) in &constants {
            match branch.row.get(*place) {
                Some(FlatTerm::Variable(variable)) => {
                    let _old = carried.insert(*variable, *constant);
                }
                Some(own @ (FlatTerm::Text(_) | FlatTerm::Number(_))) => {
                    possible &= own == constant
                }
                Some(FlatTerm::File) | None => {}
            }
        }
        if possible {
            let need = needed(drafts, branch, &carried, depth.saturating_add(1));
            // A branch that holds the column to another constant gives no row: it is not a way to have one.
            if !never(&need) {
                some.push(need);
            }
        }
        if some.len() > CARRIED.1 {
            return Emptiness::Definition(definition);
        }
    }
    Emptiness::Any(some.into())
}

/// The variables of a branch that hold a text of the file's own tree: a cell of a node relation's `Text` column, in a
/// relation whose rows are the file's own tree (`NodeExtent::Own`). Such a text is a slice of the file's bytes, or a
/// text the language decoded for the element when the table was built (kept among the file's derived names): the
/// node table's declaration (`NodeColumn::Text`). An embedded segment's logical text is not counted: it may be made
/// after the file's first decision.
fn own_texts(branch: &FlatBranch) -> Vec<FlatVariable> {
    let mut texts = Vec::new();
    for atom in &branch.atoms {
        let (relation, cells): (Relation, Vec<Option<FlatTerm>>) = match atom {
            FlatAtom::Rows {
                rows: AtomRows::Relation(relation),
                cells,
                ..
            } => (*relation, cells.iter().copied().map(Some).collect()),
            FlatAtom::Decided {
                polarity: Polarity::Present,
                rows: AtomRows::Relation(relation),
                cells,
            } => (*relation, cells.clone()),
            FlatAtom::Rows { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Latest { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. } => continue,
        };
        if !matches!(fact_form(relation), FactForm::NodeTable) || node_extent(relation) != Some(NodeExtent::Own) {
            continue;
        }
        for (column, cell) in columns(relation).into_iter().zip(cells) {
            if let (Some(NodeColumn::Text), Some(FlatTerm::Variable(variable))) = (node_column(column), cell) {
                texts.push(variable);
            }
        }
    }
    texts
}

/// The text cells of a branch's rows of filled relations: each variable bound to a symbol column of a relation the
/// language fills, with that relation and column. A test such a cell must pass is a test some row of the relation
/// passes in the file.
fn filled_texts(branch: &FlatBranch) -> HashMap<FlatVariable, (Relation, Column)> {
    let mut texts = HashMap::new();
    for atom in &branch.atoms {
        let (relation, cells): (Relation, Vec<Option<FlatTerm>>) = match atom {
            FlatAtom::Rows {
                rows: AtomRows::Relation(relation),
                cells,
                ..
            } => (*relation, cells.iter().copied().map(Some).collect()),
            FlatAtom::Decided {
                polarity: Polarity::Present,
                rows: AtomRows::Relation(relation),
                cells,
            } => (*relation, cells.clone()),
            FlatAtom::Rows { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Latest { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. } => continue,
        };
        if !matches!(fact_form(relation), FactForm::Filled(_)) {
            continue;
        }
        for (column, cell) in columns(relation).into_iter().zip(cells) {
            if let (CellKind::Symbol, Some(FlatTerm::Variable(variable))) = (kind(column), cell) {
                let _first = texts.entry(variable).or_insert((relation, column));
            }
        }
    }
    texts
}

/// The literal a test of a text against a constant needs the text to hold: the constant's own for a prefix or a
/// suffix, the expression's for a match. None for any other test: an equality is the spelled term's (a token of the
/// exact text), a glob or a negated test needs nothing.
fn test_literal(test: TestOperation, constant: CellWord) -> Option<ByteLiteral> {
    match test {
        TestOperation::StartsWith | TestOperation::EndsWith => Some(ByteLiteral::Text(constant)),
        TestOperation::Matches(regex) => Some(ByteLiteral::Regex(regex)),
        TestOperation::Equal
        | TestOperation::NotEqual
        | TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::In
        | TestOperation::NotIn
        | TestOperation::Glob
        | TestOperation::NotGlob
        | TestOperation::NumericAtLeast(_)
        | TestOperation::NaturalLess
        | TestOperation::Successor(_) => None,
    }
}

/// What a file must hold for a branch to give a row, with `carried` variables of it held to constants by its reader.
fn needed(
    drafts: &Drafts,
    branch: &FlatBranch,
    carried: &HashMap<FlatVariable, FlatTerm>,
    depth: usize,
) -> Emptiness {
    let mut pins = pins_of(branch);
    for (variable, constant) in carried {
        // A variable its own branch holds to another constant gives no row.
        if pins
            .insert(*variable, *constant)
            .is_some_and(|own| own != *constant)
        {
            return Emptiness::Any(Box::default());
        }
    }
    let (mut parts, mut uses) = (Vec::new(), Vec::new());
    let texts = own_texts(branch);
    let filled = filled_texts(branch);
    for atom in &branch.atoms {
        let (rows, cells): (AtomRows, Vec<Option<FlatTerm>>) = match atom {
            FlatAtom::Rows { rows, cells, .. } => {
                (*rows, cells.iter().copied().map(Some).collect())
            }
            FlatAtom::Decided {
                polarity: Polarity::Present,
                rows,
                cells,
            } => (*rows, cells.clone()),
            FlatAtom::Decided {
                polarity: Polarity::Absent,
                ..
            }
            | FlatAtom::Latest { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. } => continue,
            // A search of the file's bytes for an expression: no occurrence, no row, in a file that holds none of
            // the texts every match of it holds.
            FlatAtom::Called {
                meaning: RowMeaning::Call(CallMeaning::SourceMatches(regex, _)),
                ..
            } => {
                parts.push(Emptiness::Bytes(ByteLiteral::Regex(*regex)));
                continue;
            }
            FlatAtom::Called { .. } => continue,
            // A test of a text of the file's own tree against a constant text: the text holds the literal the test
            // needs, so the file's texts do.
            FlatAtom::Test {
                test,
                left: FlatTerm::Variable(variable),
                right,
            } if texts.contains(variable) => {
                if let Some(FlatTerm::Text(number)) = held(&pins, *right)
                    && let Some(literal) = test_literal(*test, text_word(number))
                {
                    parts.push(Emptiness::Bytes(literal));
                }
                continue;
            }
            // A test of a text of a filled relation's row against a constant text: some row of the relation passes
            // it in a file where the branch has a row.
            FlatAtom::Test {
                test,
                left: FlatTerm::Variable(variable),
                right,
            } if filled.contains_key(variable) && path_test(*test) => {
                if let Some(FlatTerm::Text(number)) = held(&pins, *right)
                    && let Some((relation, column)) = filled.get(variable).copied()
                {
                    parts.push(Emptiness::RowTest(
                        relation,
                        RowTest {
                            column,
                            test: *test,
                            value: text_word(number),
                        },
                    ));
                }
                continue;
            }
            // A test of the scanned file's path against a constant text: the branch has no row in a file whose path
            // fails it, whatever else it holds.
            FlatAtom::Test {
                test,
                left: FlatTerm::File,
                right,
            } => {
                if let Some(FlatTerm::Text(number)) = held(&pins, *right)
                    && path_test(*test)
                {
                    parts.push(Emptiness::Path(PathTest {
                        test: *test,
                        value: text_word(number),
                    }));
                }
                continue;
            }
            FlatAtom::Test { .. } => continue,
        };
        match rows {
            AtomRows::Relation(relation) => relation_need(relation, &cells, &pins, &mut parts),
            AtomRows::Definition(definition) => {
                uses.push(definition_need(drafts, definition, &cells, &pins, depth));
            }
            // A made table is asked for a row by making it: the branch's reader decides when.
            AtomRows::Made(_) => {}
        }
    }
    // What a relation says is asked before what another unit decides.
    parts.extend(uses);
    Emptiness::All(parts.into())
}

/// What a file must hold for a branch to give a row, read off the branch.
#[must_use]
pub(crate) fn branch_need(drafts: &Drafts, branch: &FlatBranch) -> Emptiness {
    if traced() {
        eprintln!(
            "ryo x: need-branch row={:?} atoms={:?}",
            branch.row, branch.atoms
        );
    }
    needed(drafts, branch, &HashMap::new(), 0)
}

/// What a file must hold for some of a step's branches to give a row.
#[must_use]
pub(crate) fn branches_need(drafts: &Drafts, branches: &[FlatBranch]) -> Emptiness {
    let each = branches.iter().map(|branch| branch_need(drafts, branch));
    Emptiness::Any(each.collect())
}

/// Whether a formula holds in no file: some part of an `All` never does, or no part of an `Any` can.
fn never(formula: &Emptiness) -> bool {
    match formula {
        Emptiness::All(parts) => parts.iter().any(never),
        Emptiness::Any(parts) => parts.iter().all(never),
        Emptiness::Unknown
        | Emptiness::Kind(_)
        | Emptiness::Row(..)
        | Emptiness::Definition(_)
        | Emptiness::Table(_)
        | Emptiness::Path(_)
        | Emptiness::Bytes(_)
        | Emptiness::RowTest(..) => false,
    }
}

/// Whether a formula holds in every file: it says nothing.
fn always(formula: &Emptiness) -> bool {
    match formula {
        Emptiness::All(parts) => parts.iter().all(always),
        Emptiness::Any(parts) => parts.iter().any(always),
        Emptiness::Unknown
        | Emptiness::Kind(_)
        | Emptiness::Row(..)
        | Emptiness::Definition(_)
        | Emptiness::Table(_)
        | Emptiness::Path(_)
        | Emptiness::Bytes(_)
        | Emptiness::RowTest(..) => false,
    }
}

/// A formula with what the branches need beside it; the formula alone when the branches' need says nothing.
#[must_use]
pub(crate) fn with_need(formula: Emptiness, need: Emptiness) -> Emptiness {
    if always(&need) {
        formula
    } else {
        Emptiness::All([formula, need].into())
    }
}
