//! The services of the `flattening` part: the equalities of a branch resolved in one union-find and one rewrite, a comparison
//! added to a branch, and the columns of a row picked. Nothing here copies a sub-plan: a branch holds uses, never bodies.

use super::types::{EqualTerms, FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowColumn};
use crate::domain::steps::types::TestOperation;
use std::collections::HashMap;

/// Visits every term of an atom, in place.
pub(crate) fn each_term(atom: &mut FlatAtom, visit: &mut impl FnMut(&mut FlatTerm)) {
    match atom {
        FlatAtom::Rows { cells, .. } => cells.iter_mut().for_each(visit),
        FlatAtom::Latest {
            key, bound, cells, ..
        } => {
            key.iter_mut().for_each(&mut *visit);
            visit(bound);
            cells.iter_mut().for_each(visit);
        }
        FlatAtom::Decided { cells, .. } => cells.iter_mut().flatten().for_each(visit),
        FlatAtom::Walk { start, reached, .. } | FlatAtom::Nearest { start, reached, .. } => {
            visit(start);
            visit(reached);
        }
        FlatAtom::Test { left, right, .. } => {
            visit(left);
            visit(right);
        }
    }
}

/// The variables of an atom, each as often as it is written.
#[must_use]
pub(crate) fn variables(atom: &FlatAtom) -> Vec<FlatVariable> {
    let mut found = Vec::new();
    let mut atom = atom.clone();
    each_term(&mut atom, &mut |term| {
        if let FlatTerm::Variable(variable) = term {
            found.push(*variable);
        }
    });
    found
}

/// What a term stands for under the classes made so far: the class's constant when it has one, else its one variable.
fn found(classes: &HashMap<FlatVariable, FlatTerm>, term: FlatTerm) -> FlatTerm {
    let mut at = term;
    while let FlatTerm::Variable(variable) = at {
        match classes.get(&variable) {
            Some(next) => at = *next,
            None => break,
        }
    }
    at
}

/// The constant each pinned variable of a branch is held to: the equalities of a variable and a constant among its tests.
#[must_use]
pub(crate) fn pins_of(branch: &FlatBranch) -> HashMap<FlatVariable, FlatTerm> {
    let pins = branch.atoms.iter().filter_map(|atom| match atom {
        FlatAtom::Test {
            test: TestOperation::Equal,
            left: FlatTerm::Variable(variable),
            right: right @ (FlatTerm::Text(_) | FlatTerm::Number(_)),
        } => Some((*variable, *right)),
        FlatAtom::Test { .. }
        | FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Walk { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Decided { .. } => None,
    });
    pins.collect()
}

/// The branch with each pair of terms made equal: every pair goes into one union-find over the branch's terms, then the
/// branch is rewritten once. A class keeps its variable: a constant it is equal to is one test of it (a pin), so a key of
/// the plan stays a key when one side of it is a constant. None when the branch then holds of no row: two different
/// constants equal, or a constant equal to the file.
#[must_use]
pub(crate) fn unified(mut branch: FlatBranch, pairs: &[EqualTerms]) -> Option<FlatBranch> {
    let mut classes: HashMap<FlatVariable, FlatTerm> = HashMap::new();
    let mut pinned: Vec<(FlatVariable, FlatTerm)> = Vec::new();
    for pair in pairs {
        let (left, right) = (found(&classes, pair.left), found(&classes, pair.right));
        match (left, right) {
            _ if left == right => {}
            (FlatTerm::Variable(variable), other @ (FlatTerm::Variable(_) | FlatTerm::File))
            | (other @ FlatTerm::File, FlatTerm::Variable(variable)) => {
                let _old = classes.insert(variable, other);
            }
            (
                FlatTerm::Variable(variable),
                constant @ (FlatTerm::Text(_) | FlatTerm::Number(_)),
            )
            | (
                constant @ (FlatTerm::Text(_) | FlatTerm::Number(_)),
                FlatTerm::Variable(variable),
            ) => {
                pinned.push((variable, constant));
            }
            (
                FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File,
                FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File,
            ) => return None,
        }
    }
    if !classes.is_empty() {
        for atom in &mut branch.atoms {
            each_term(atom, &mut |term| *term = found(&classes, *term));
        }
        for term in &mut branch.row {
            *term = found(&classes, *term);
        }
    }
    for (variable, constant) in pinned {
        match found(&classes, FlatTerm::Variable(variable)) {
            FlatTerm::Variable(left) => branch.atoms.push(FlatAtom::Test {
                test: TestOperation::Equal,
                left: FlatTerm::Variable(left),
                right: constant,
            }),
            FlatTerm::File | FlatTerm::Text(_) | FlatTerm::Number(_) => return None,
        }
    }
    // One pin a variable: the same pin written twice is one, and two constants for one variable hold of no row. A pin
    // whose two sides the rewrite made constants is decided here.
    let mut pins: HashMap<FlatVariable, FlatTerm> = HashMap::new();
    let mut kept = Vec::with_capacity(branch.atoms.len());
    for atom in branch.atoms {
        match atom {
            FlatAtom::Test {
                test: TestOperation::Equal,
                left: FlatTerm::Variable(variable),
                right: right @ (FlatTerm::Text(_) | FlatTerm::Number(_)),
            } => match pins.insert(variable, right) {
                Some(first) if first != right => return None,
                Some(_) => {}
                None => kept.push(atom),
            },
            FlatAtom::Test {
                test: TestOperation::Equal,
                left: FlatTerm::File,
                right: FlatTerm::Text(_) | FlatTerm::Number(_),
            } => return None,
            FlatAtom::Test { .. }
            | FlatAtom::Rows { .. }
            | FlatAtom::Latest { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Decided { .. } => kept.push(atom),
        }
    }
    branch.atoms = kept;
    Some(branch)
}

/// The branch with one more comparison. An equality binds at once: it is one more pair of the union-find. An equality or an
/// inequality of two constants is decided here; any other comparison is a test of the branch. None when the branch then
/// holds of no row.
#[must_use]
pub(crate) fn compared(
    mut branch: FlatBranch,
    test: TestOperation,
    left: FlatTerm,
    right: FlatTerm,
) -> Option<FlatBranch> {
    match test {
        TestOperation::Equal => unified(branch, &[EqualTerms { left, right }]),
        TestOperation::NotEqual
            if matches!(left, FlatTerm::Text(_) | FlatTerm::Number(_))
                && matches!(right, FlatTerm::Text(_) | FlatTerm::Number(_)) =>
        {
            (left != right).then_some(branch)
        }
        TestOperation::NotEqual if left == right => None,
        TestOperation::NotEqual
        | TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::In
        | TestOperation::NotIn
        | TestOperation::Matches(_)
        | TestOperation::Glob
        | TestOperation::NotGlob
        | TestOperation::StartsWith
        | TestOperation::EndsWith
        | TestOperation::NumericAtLeast
        | TestOperation::NaturalLess
        | TestOperation::Successor => {
            branch.atoms.push(FlatAtom::Test { test, left, right });
            Some(branch)
        }
    }
}

/// The terms of a row at the given columns, in their order; none when a column is past the row.
#[must_use]
pub(crate) fn picked(
    row: &[FlatTerm],
    columns: impl IntoIterator<Item = RowColumn>,
) -> Option<Vec<FlatTerm>> {
    let cells = columns.into_iter().map(|column| row.get(column.0).copied());
    cells.collect()
}
