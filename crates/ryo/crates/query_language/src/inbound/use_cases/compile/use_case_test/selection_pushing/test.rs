//! Where the compile inbound API decides the comparisons of a `where` over a join: those of its infallible prefix that test one
//! side on that side, a right one riding the walk of the tree when the join finds the element by its id, and the rest after the
//! join. Each row's text copies one rule's step, with its left read straight from the relation where the rule reads a step of
//! it: `eslint.constructor-super/query.ryoql:29` and `deprecated_clippy_cfg_attr/query.ryoql:4` (the edges for
//! `atom_children`), and a node-keyed join of the edges with the elements on both sides.

use super::super::compiler_over_the_language;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::compile::errors::CompileError;
use crate::inbound::use_cases::compile::interface::CompileUseCase as _;
use ryo_values::domain::plan_parts::types::{
    Input as Reads, Operand, PlanComparison, QueryLiteral, QueryText,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_requests::types::QueryRequest;
use ryo_values::domain::query_tokens::types::QueryBundle;
use std::borrow::Cow;

/// `eslint.constructor-super/query.ryoql:29`: a `where` on both sides of a join of the semantic edges with the semantic elements.
const OPERATORS: &str = r#"
let operators = select l.parent as id from "semantic-child" as l join "semantic-syntax" as r on l.child = r.id where l.field = "operator" and r.text = "||";
emit operators subject id key(id);
"#;

/// A join of the edges with the elements on the element's id, a `where` on the field of the edge and the kind of the element.
const BODY_RETURNS: &str = r#"
let returns = select l.parent as id from "syntax-child" as l join "syntax-tree" as r on l.child = r.id where l.field = "body" and r.kind = "return-statement";
emit returns subject id key(id);
"#;

/// `deprecated_clippy_cfg_attr/query.ryoql:4`: a kind, then a pattern of the element's text.
const FEATURE_KEYS: &str = r#"
let feature_keys = select l.parent as id from "syntax-child" as l join "syntax-tree" as r on l.child = r.id where r.kind = "ident" and matches(r.text, "^(?:r#)?feature$");
emit feature_keys subject id key(id);
"#;

/// The text a row compiles.
type Input = &'static str;
/// The plan's one join, written as what it reads on each side, what it walks with, and the filters of its answer.
type Value = Cow<'static, str>;
type Failure = CompileError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_where_on_both_sides_of_a_join_of_two_relations_is_a_filter_of_each_side"),
        word: Word::Succeeds,
        input: || OPERATORS,
        expect: Expect::Value(Cow::Borrowed(
            "SemanticChild where [#3 Equal \"operator\"] join SemanticSyntax where [#2 Equal \"||\"] walking [] then []",
        )),
    },
    Case {
        name: CaseName(
            "a_where_of_the_elements_joined_by_their_id_rides_the_walk_and_the_edges_are_filtered",
        ),
        word: Word::Succeeds,
        input: || BODY_RETURNS,
        expect: Expect::Value(Cow::Borrowed(
            "SyntaxChild where [#3 Equal \"body\"] join SyntaxTree walking [#1 Equal \"return-statement\"] then []",
        )),
    },
    Case {
        name: CaseName(
            "a_pattern_of_the_elements_stays_a_filter_of_the_join_and_their_kind_rides_the_walk",
        ),
        word: Word::Succeeds,
        input: || FEATURE_KEYS,
        expect: Expect::Value(Cow::Borrowed(
            "SyntaxChild join SyntaxTree walking [#1 Equal \"ident\"] then [#2 Matches \"^(?:r#)?feature$\"]",
        )),
    },
];

/// An operand as the answer writes it: a column by its place, a text quoted.
fn operand(operand: &Operand) -> String {
    match operand {
        Operand::Column(column) => format!("#{}", column.0),
        Operand::Literal(QueryLiteral::Text(text)) => format!("{:?}", text.0),
        Operand::Literal(_) | Operand::Parameter(_) | Operand::Function(_) => {
            format!("{operand:?}")
        }
    }
}

/// Comparisons as the answer writes them, in their order.
fn comparisons(each: &[PlanComparison]) -> String {
    each.iter()
        .map(|comparison| {
            format!(
                "{} {:?} {}",
                operand(&comparison.left),
                comparison.test,
                operand(&comparison.right)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// What a join reads on one side: a relation, or a filter of one with its comparisons, or another step.
fn side(steps: &[PlanStep], input: Reads) -> String {
    match input {
        Reads::Relation(relation) => format!("{relation:?}"),
        Reads::Step(id) => match steps
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .map(|step| &step.operation)
        {
            Some(Operation::Select {
                from: Reads::Relation(relation),
                comparisons: tested,
                ..
            }) => format!("{relation:?} where [{}]", comparisons(tested)),
            _ => "a step".to_owned(),
        },
        Reads::BoundObservation(_) => "an observation".to_owned(),
    }
}

/// The plan of `text` from the compile inbound API, its one join written out.
fn subject(text: Input) -> Result<Value, Failure> {
    let steps = compiler_over_the_language()
        .compile(QueryRequest {
            text: QueryBundle::from(QueryText(text.into())),
            hosted: Vec::new(),
            observations: Vec::new(),
            modules: ryo_values::domain::program::types::ParsedModules::default(),
        })?
        .plan
        .steps;
    let joins: Vec<String> = steps
        .iter()
        .enumerate()
        .filter_map(|(at, step)| {
            let Operation::Join {
                left,
                right,
                filter,
                ..
            } = &step.operation
            else {
                return None;
            };
            let after: Vec<String> = steps
                .iter()
                .filter_map(|reader| {
                    let Operation::Select {
                        from: Reads::Step(id),
                        comparisons: tested,
                        ..
                    } = &reader.operation
                    else {
                        return None;
                    };
                    (usize::try_from(id.0).unwrap_or(usize::MAX) == at).then(|| comparisons(tested))
                })
                .collect();
            Some(format!(
                "{} join {} walking [{}] then [{}]",
                side(&steps, *left),
                side(&steps, *right),
                comparisons(filter),
                after.join("; ")
            ))
        })
        .collect();
    Ok(Cow::Owned(joins.join(" | ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
