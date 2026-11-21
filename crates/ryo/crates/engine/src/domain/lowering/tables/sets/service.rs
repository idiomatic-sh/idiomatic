//! The services of the `sets` part: the literals a comparison holds its left side to, when it holds it to a set of them (an
//! `in` list, a glob-any or a regular expression whose patterns are all literal), the cell of a branch that is a node's
//! kind or field, and the branches such a comparison of such a cell gives: one for each literal, the cell pinned to it.
//! A pin of a kind or a field names a bucket of the node table, so the selection reads the union of the buckets and
//! tests no node's text. A pattern with a wildcard gives no set here: it stays a test of the text.

use super::constants::{
    CAPTURING_GROUP, GLOB_OR, GLOB_SYNTAX, GROUP_CLOSE, GROUP_END, GROUP_OPEN, PLAIN_GROUP,
    REGEX_OR, TEXT_END, TEXT_START, WORD_MARKS,
};
use super::types::BucketCell;
use crate::domain::lowering::definitions::service::text;
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::service::compared;
use crate::domain::lowering::flattening::types::{AtomRows, FlatAtom, FlatBranch, FlatTerm};
use crate::domain::lowering::types::SelectedRule;
use crate::domain::steps::types::TestOperation;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{Column, FactForm, NodeColumn, Relation};
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::plan_parts::types::{
    LiteralText, Operand, ParameterId, PlanComparison, QueryLiteral, Test,
};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// The texts a glob matches, when it matches literals alone: its own text when it holds no glob syntax, or each alternative
/// of one group that is the whole glob when no alternative holds any. None for a glob with a wildcard, a class, an escape,
/// a nested group, text beside its group or an empty alternative.
#[must_use]
pub(crate) fn glob_literals(glob: &LiteralText) -> Option<Vec<LiteralText>> {
    let syntax = |byte| GLOB_SYNTAX.contains(&byte);
    if !glob.0.is_empty() && !glob.0.bytes().any(syntax) {
        return Some(vec![glob.clone()]);
    }
    let inside = glob.0.strip_prefix(GROUP_OPEN)?.strip_suffix(GROUP_CLOSE)?;
    let alternatives = inside.split(GLOB_OR);
    let mut each = alternatives.clone();
    each.all(|text| !text.is_empty() && !text.bytes().any(syntax))
        .then(|| alternatives.map(|text| LiteralText(text.into())).collect())
}

/// The texts a regular expression matches, when it matches literals alone: it is held to the start and to the end of the
/// text, and between them is one word or one group of words with `|` between them. A word is letters, digits, `_` and `-`.
/// None for any other expression: one that searches within the text, or holds any other syntax.
#[must_use]
pub(crate) fn regex_literals(source: &LiteralText) -> Option<Vec<LiteralText>> {
    let marked = |byte| WORD_MARKS.contains(&byte);
    let body = source.0.strip_prefix(TEXT_START)?.strip_suffix(TEXT_END)?;
    let inside = body
        .strip_prefix(PLAIN_GROUP)
        .or_else(|| body.strip_prefix(CAPTURING_GROUP))
        .and_then(|opened| opened.strip_suffix(GROUP_END));
    // One word stands with no group; a group holds every alternative.
    let alternatives = inside.unwrap_or(body).split(REGEX_OR);
    let mut each = alternatives.clone();
    let words = each.all(|text| {
        let mut bytes = text.bytes();
        !text.is_empty() && bytes.all(|byte| byte.is_ascii_alphanumeric() || marked(byte))
    });
    let grouped = inside.is_some() || alternatives.clone().count() == 1;
    (words && grouped).then(|| alternatives.map(|text| LiteralText(text.into())).collect())
}

/// The literals a comparison holds its left side to, each once and in the order written, when the comparison holds it to
/// one of a set of literals: `in` a parameter bound to a list of texts; a glob-any of a glob the query wrote, or of a
/// parameter bound to one glob or to a list of them, when every glob matches literals alone; a regular expression the
/// query wrote that matches literals alone. None for every other comparison, an equality among them: that is a pin already.
#[must_use]
pub(crate) fn literals(
    rule: &SelectedRule<'_>,
    comparison: &PlanComparison,
) -> Option<Vec<LiteralText>> {
    let bound = |parameter: &ParameterId| {
        let place = usize::try_from(parameter.0).unwrap_or(usize::MAX);
        rule.bindings.0.get(place)
    };
    let written: Vec<LiteralText> = match (comparison.test, &comparison.right) {
        (Test::In, Operand::Parameter(parameter)) => match bound(parameter)? {
            ParameterBinding::Texts(texts) => texts.clone(),
            ParameterBinding::Number(_)
            | ParameterBinding::Flag(_)
            | ParameterBinding::Text(_)
            | ParameterBinding::Numbers(_)
            | ParameterBinding::TextSets(_)
            | ParameterBinding::Tables(_) => return None,
        },
        (Test::GlobAny, Operand::Literal(QueryLiteral::Text(glob))) => glob_literals(glob)?,
        (Test::GlobAny, Operand::Parameter(parameter)) => match bound(parameter)? {
            ParameterBinding::Text(glob) => glob_literals(glob)?,
            ParameterBinding::Texts(globs) => {
                let mut every = Vec::with_capacity(globs.len());
                for glob in globs {
                    every.extend(glob_literals(glob)?);
                }
                every
            }
            ParameterBinding::Number(_)
            | ParameterBinding::Flag(_)
            | ParameterBinding::Numbers(_)
            | ParameterBinding::TextSets(_)
            | ParameterBinding::Tables(_) => return None,
        },
        (Test::Matches, Operand::Literal(QueryLiteral::Text(source))) => regex_literals(source)?,
        (
            Test::Equal
            | Test::NotEqual
            | Test::LessThan
            | Test::LessThanOrEqual
            | Test::GreaterThan
            | Test::GreaterThanOrEqual
            | Test::FloatTextGreaterThanOrEqual
            | Test::In
            | Test::NotIn
            | Test::GlobAny
            | Test::NotGlobAny
            | Test::StartsWith
            | Test::EndsWith
            | Test::Matches
            | Test::NumericGreaterThanOrEqual
            | Test::IntegerSuccessor
            | Test::NaturalLessThan,
            Operand::Column(_) | Operand::Parameter(_) | Operand::Literal(_) | Operand::Function(_),
        ) => return None,
    };
    let mut set: Vec<LiteralText> = Vec::with_capacity(written.len());
    for literal in written {
        if !set.contains(&literal) {
            set.push(literal);
        }
    }
    Some(set)
}

/// Which column of the node table a relation's column is, when the relation is of the node-table form and the column is
/// its kind or its field: the two a constant names a bucket of by a small code. None for any other column.
#[must_use]
pub(crate) fn bucket_column(relation: Relation, column: Column) -> Option<NodeColumn> {
    let held = node_column(column).filter(|_| fact_form(relation) == FactForm::NodeTable)?;
    match held {
        NodeColumn::Kind | NodeColumn::Field => Some(held),
        NodeColumn::Id
        | NodeColumn::Parent
        | NodeColumn::Ordinal
        | NodeColumn::Start
        | NodeColumn::End
        | NodeColumn::Span
        | NodeColumn::File
        | NodeColumn::Text => None,
    }
}

/// The cell a comparison's left side is, when it is a column of the branch's row that a node relation read in the branch
/// gives as its kind or its field. None for any other left side: a function, a cell of a definition or of a filled
/// relation, any other column of the node table.
#[must_use]
pub(crate) fn bucket_cell(branch: &FlatBranch, left: &Operand) -> Option<BucketCell> {
    let Operand::Column(column) = left else {
        return None;
    };
    let term = branch.row.get(usize::from(*column)).copied()?;
    let FlatTerm::Variable(_) = term else {
        return None;
    };
    branch.atoms.iter().find_map(|atom| {
        let FlatAtom::Rows {
            rows: AtomRows::Relation(relation),
            cells,
            ..
        } = atom
        else {
            return None;
        };
        let mut held = cells.iter().zip(columns(*relation));
        held.find_map(|(cell, column)| {
            let _bucket = bucket_column(*relation, column).filter(|_| *cell == term)?;
            Some(BucketCell {
                term,
                relation: *relation,
                column,
            })
        })
    })
}

/// The branches of a comparison that holds a node's kind or field to a set of literals: the branch once for each literal,
/// the cell pinned to it, so each reads the literal's bucket; a literal another pin of the cell rules out gives no
/// branch. None when the comparison holds no set or its left side is no such cell: it is then a test like any other.
pub(crate) fn bucket_branches(
    rule: &SelectedRule<'_>,
    kinds: &KindVocabulary,
    drafts: &mut Drafts,
    branch: &FlatBranch,
    comparison: &PlanComparison,
) -> Option<Vec<FlatBranch>> {
    let cell = bucket_cell(branch, &comparison.left)?;
    let set = literals(rule, comparison)?;
    let mut branches = Vec::with_capacity(set.len());
    for literal in &set {
        let pin = FlatTerm::Text(text(drafts, kinds, literal));
        branches.extend(compared(
            branch.clone(),
            TestOperation::Equal,
            cell.term,
            pin,
        ));
    }
    Some(branches)
}
