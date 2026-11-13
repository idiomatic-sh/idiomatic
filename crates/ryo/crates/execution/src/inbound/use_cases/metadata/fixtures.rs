//! Shared inbound contract rows for pure metadata decisions.
#[cfg(test)]
use super::interface::{Bearing, Blocking, KindOrigin};
#[cfg(test)]
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::KindVocabularies;
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::descriptions::types::KindWord;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::plans::types::PlanStep;
#[cfg(test)]
use ryo_values::domain::query_operators::types::QueryOperator;
#[cfg(test)]
use std::borrow::Cow;

pub mod contract;

/// A plan that selects `relation`, filtered by the kind `word` when one is given.
pub fn plan(relation: Relation, word: Option<&str>) -> Plan {
    let comparisons = word
        .map(|text| PlanComparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        })
        .into_iter()
        .collect();
    Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(relation),
                comparisons,
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
    }
}
/// The kind vocabularies the metadata proofs read.
pub fn vocabularies() -> KindVocabularies {
    KindVocabularies(vec![KindVocabulary {
        language: LanguageName("rust".into()),
        kinds: vec![KindWord("fn".into())],
    }])
}
/// Retain the former operator-service example/catalog assertion through public descriptors.
pub fn example_unknown_relations(example: &str) -> usize {
    use ryo_relations::domain::labels::types::NameRef;
    use ryo_relations::domain::relations::service::relation_named;
    let fields = [
        "from",
        "from=",
        "left=",
        "right=",
        "edges=",
        "relation=",
        "domain=",
    ];
    example
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|quoted| {
            let before = example
                .split(&format!("\"{quoted}\""))
                .next()
                .unwrap_or_default()
                .trim_end();
            fields.iter().any(|field| before.ends_with(field))
                && relation_named(NameRef(quoted)).is_none()
        })
        .count()
}
/// What a metadata question answers.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Expected {
    /// The operators, those the contract described, those whose description lacks a name, a
    /// summary or an example, and the relations their examples name that the catalog lacks.
    Catalog {
        operators: Cow<'static, [QueryOperator]>,
        described: usize,
        incomplete: usize,
        unknown_relations: usize,
    },
    /// The kinds asked, whether the plan is refused, and whether a first unknown kind is found.
    KnownKind {
        asked: usize,
        refused: bool,
        unknown: bool,
    },
    /// The globs placed, the rules governing before and after the unplace, and whether the file is governed.
    Placement {
        globs: Cow<'static, [Cow<'static, str>]>,
        before: usize,
        after: usize,
        governed: bool,
    },
    /// The relations read in part.
    Recovery(Cow<'static, [Relation]>),
    /// The relations read, the observed reads, how the failed read bears and blocks, and each
    /// unknown's path and rule and whether its message is the file's unknown message.
    Trust {
        reads: Cow<'static, [Relation]>,
        observed: usize,
        bearing: Bearing,
        blocking: Blocking,
        unknowns: Cow<'static, [(Cow<'static, str>, Cow<'static, str>, bool)]>,
    },
    /// The step's work, the rows fed to the select and to the relation, and whether the feature
    /// catalog lists the select.
    Features {
        work: u64,
        select: u64,
        relation: u64,
        catalogued: bool,
    },
    /// The affected file ordinals.
    Affected(Cow<'static, [usize]>),
}
/// What a metadata question is refused with.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    /// The refusal's message and origin, the first unknown's rule and word, and whether the
    /// refusal is the unknown kind message.
    Kind {
        message: Cow<'static, str>,
        origin: KindOrigin,
        rule: usize,
        word: Cow<'static, str>,
        as_message: bool,
    },
    /// An observation's incompleteness, with its reason.
    Incomplete(Cow<'static, str>),
    /// Any other coverage refusal.
    Coverage(OperatingError),
}
