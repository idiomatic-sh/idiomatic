//! The rules the engine port's fake runs: which elements of a file a rule finds a row for, what each column of its rows holds,
//! the fixture's lowering of a plan (the lowering fixture's one tree rule, or a compiled rule of the same shape), and how a
//! fake's pack names its rules. A compiled rule names a kind by its word, and the fixture takes the word's place among the
//! language's kinds as the kind's code.

use ryo_engine::domain::lowering::fixtures::named;
use ryo_engine::domain::lowering::types::{PlanNode, PlanShape, SelectedRule};
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{Column, FactForm, KindBits, NodeColumn};
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::plan_parts::types::{Input, Operand, PlanComparison, QueryLiteral, Test};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// What one column of a rule's found row holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Held {
    /// Where the word starts, as a number.
    Start,
    /// Where the word ends, as a number.
    End,
    /// The word's text.
    Word,
    /// The word's element.
    Node,
    /// The file.
    File,
    /// The word's span.
    Span,
}

/// Every column a row may hold, in the order a pack numbers them.
pub(super) const HELD: [Held; 6] = [
    Held::Start,
    Held::End,
    Held::Word,
    Held::Node,
    Held::File,
    Held::Span,
];

/// Which elements of a file a rule finds a row for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Anchor {
    /// Every token: the lowering fixture's one tree rule.
    Tokens,
    /// Every node and token: a compiled rule with no comparison.
    Every,
    /// The nodes and tokens of one kind: a compiled rule that compares the kind with a word.
    Kind(KindBits),
}

/// A rule the fake runs: its ordinal, the elements it finds a row for, the columns of its rows, and whether each row is handed
/// to its frontier too.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FixtureRule {
    /// The rule.
    pub(super) rule: SelectedRuleOrdinal,
    /// The elements it finds a row for.
    pub(super) anchor: Anchor,
    /// The columns of its rows.
    pub(super) cells: Vec<Held>,
    /// Whether each word is a frontier row too.
    pub(super) frontier: bool,
}

/// The kind a comparison of a step over `relation` names by its word: the comparison is an equality of the relation's kind
/// column with a written text, and the word is one of the language's kinds, whose place among them is its code.
fn kind_named(
    comparison: &PlanComparison,
    all: &[Column],
    kinds: &KindVocabulary,
) -> Option<KindBits> {
    let (Test::Equal, Operand::Column(place), Operand::Literal(QueryLiteral::Text(word))) =
        (comparison.test, &comparison.left, &comparison.right)
    else {
        return None;
    };
    let column = all.get(usize::from(*place)).copied()?;
    let code = kinds.kinds.iter().position(|kind| *kind.0 == *word.0)?;
    (node_column(column) == Some(NodeColumn::Kind))
        .then(|| u16::try_from(code).ok().map(KindBits))
        .flatten()
}

/// The elements and the columns of a compiled rule of the one tree rule's shape: one step that selects columns of a relation
/// read in place from the node table (the element, its file, its span, a token's text), of every element or of the elements of
/// one kind. None for another plan.
fn tree_rule(plan: &Plan, kinds: &KindVocabulary) -> Option<(Anchor, Vec<Held>)> {
    let [step] = plan.steps.as_slice() else {
        return None;
    };
    let Operation::Select {
        from: Input::Relation(relation),
        comparisons,
        kept,
    } = &step.operation
    else {
        return None;
    };
    if !plan.frontier.is_empty() || kept.is_empty() || fact_form(*relation) != FactForm::NodeTable {
        return None;
    }
    let all = columns(*relation);
    let anchor = match comparisons.as_slice() {
        [] => Anchor::Every,
        [one] => Anchor::Kind(kind_named(one, &all, kinds)?),
        _ => return None,
    };
    let cells: Option<Vec<Held>> = kept
        .iter()
        .map(|place| {
            let column = all.get(usize::from(*place)).copied()?;
            match node_column(column)? {
                NodeColumn::Id => Some(Held::Node),
                NodeColumn::File => Some(Held::File),
                NodeColumn::Span => Some(Held::Span),
                NodeColumn::Text => Some(Held::Word),
                NodeColumn::Kind
                | NodeColumn::Parent
                | NodeColumn::Field
                | NodeColumn::Ordinal
                | NodeColumn::Start
                | NodeColumn::End => None,
            }
        })
        .collect();
    Some((anchor, cells?))
}

/// The fixture's lowering of one rule against a language's kinds.
pub(super) fn lowered(
    rule: &SelectedRule<'_>,
    kinds: &KindVocabulary,
) -> Result<FixtureRule, LoadError> {
    if *rule.plan == named("program over budget") {
        return Err(LoadError::Budget { rule: rule.rule });
    }
    if *rule.plan == named("one tree rule") {
        return Ok(FixtureRule {
            rule: rule.rule,
            anchor: Anchor::Tokens,
            cells: vec![Held::Start, Held::End, Held::Word],
            frontier: true,
        });
    }
    match tree_rule(rule.plan, kinds) {
        Some((anchor, cells)) => Ok(FixtureRule {
            rule: rule.rule,
            anchor,
            cells,
            frontier: false,
        }),
        None => Err(LoadError::Unlowerable {
            rule: rule.rule,
            shape: PlanShape::Step(PlanNode(0)),
        }),
    }
}

/// The rules a fake's pack names after the engine's header, each as little-endian words of four bytes: its ordinal, which
/// elements it finds (0 the tokens, 1 every element, 2 the elements of the kind the next word says), that kind, whether its
/// rows are frontier rows too, how many columns its rows have, then each column's number among [`HELD`].
pub(super) fn named_rules(tail: &[u8]) -> Result<Vec<FixtureRule>, LoadError> {
    let (words, rest) = tail.as_chunks::<{ size_of::<u32>() }>();
    if !rest.is_empty() {
        return Err(LoadError::PackCorrupt);
    }
    let mut words = words.iter().map(|word| u32::from_le_bytes(*word));
    let mut rules = Vec::new();
    while let Some(rule) = words.next() {
        let head = (words.next(), words.next(), words.next(), words.next());
        let (Some(found), Some(kind), Some(frontier), Some(count)) = head else {
            return Err(LoadError::PackCorrupt);
        };
        let anchor = match (found, u16::try_from(kind)) {
            (0, _) => Anchor::Tokens,
            (1, _) => Anchor::Every,
            (2, Ok(kind)) => Anchor::Kind(KindBits(kind)),
            _ => return Err(LoadError::PackCorrupt),
        };
        let mut cells = Vec::new();
        for _ in 0..count {
            let held = words
                .next()
                .and_then(|code| HELD.get(usize::try_from(code).unwrap_or(usize::MAX)))
                .ok_or(LoadError::PackCorrupt)?;
            cells.push(*held);
        }
        rules.push(FixtureRule {
            rule: SelectedRuleOrdinal(rule),
            anchor,
            cells,
            frontier: frontier != 0,
        });
    }
    Ok(rules)
}

/// The words a rule is written as in a fake's pack: what [`named_rules`] reads back.
pub(super) fn written(rule: &FixtureRule) -> Vec<u32> {
    let code = |held: &Held| {
        u32::try_from(
            HELD.iter()
                .position(|each| each == held)
                .unwrap_or_default(),
        )
        .unwrap_or_default()
    };
    let (found, kind) = match rule.anchor {
        Anchor::Tokens => (0, 0),
        Anchor::Every => (1, 0),
        Anchor::Kind(kind) => (2, u32::from(kind.0)),
    };
    let mut words = vec![
        rule.rule.0,
        found,
        kind,
        u32::from(rule.frontier),
        u32::try_from(rule.cells.len()).unwrap_or(u32::MAX),
    ];
    words.extend(rule.cells.iter().map(code));
    words
}
