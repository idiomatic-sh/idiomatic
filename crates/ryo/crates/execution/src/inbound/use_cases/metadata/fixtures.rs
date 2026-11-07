//! Shared inbound contract rows for pure metadata decisions.
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::KindVocabularies;
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::descriptions::types::KindWord;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::plans::types::PlanStep;

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
    use ryo_relations::domain::relations::service::relation_named;
    use ryo_relations::domain::labels::types::NameRef;
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
