//! The operator catalog through the public inbound implementation: every public operator has its registered description and example.
use super::fixtures::example_unknown_relations;
use super::interface::MetadataUseCase as _;
use super::use_case::MetadataUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::QueryOperator;
use std::borrow::Cow;

/// What the catalog answers.
#[derive(Debug, PartialEq, Eq)]
struct Catalog {
    /// The operators, in the order listed.
    operators: Cow<'static, [QueryOperator]>,
    /// How many distinct, nonempty names they have.
    described: usize,
    /// How many have no name, summary or example.
    incomplete: usize,
    /// How many relations their examples name that no relation catalog has.
    unknown_relations: usize,
}
type Input = ();
type Value = Catalog;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_public_operator_has_its_registered_description_and_example"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(Catalog {
            operators: Cow::Borrowed(&[
                QueryOperator::SpanBetween,
                QueryOperator::SpanBounds,
                QueryOperator::SourceMatches,
                QueryOperator::RenderSyntax,
                QueryOperator::FitsRendered,
                QueryOperator::Replace,
                QueryOperator::OrderedMatch,
                QueryOperator::AsOf,
                QueryOperator::OrderedDfs,
                QueryOperator::Worklist,
                QueryOperator::Select,
                QueryOperator::Project,
                QueryOperator::Extend,
                QueryOperator::Join,
                QueryOperator::Union,
                QueryOperator::Intersection,
                QueryOperator::Difference,
                QueryOperator::Antijoin,
                QueryOperator::GroupCount,
                QueryOperator::GroupDigest,
                QueryOperator::GroupMin,
                QueryOperator::GroupMax,
                QueryOperator::GroupOrdinal,
                QueryOperator::Capture,
                QueryOperator::Words,
                QueryOperator::Rank,
                QueryOperator::Axis,
                QueryOperator::Rows,
                QueryOperator::Closure,
                QueryOperator::Follow,
                QueryOperator::Cycles,
                QueryOperator::Order,
                QueryOperator::Prefixes,
                QueryOperator::SubtreeClass,
            ]),
            described: 34,
            incomplete: 0,
            unknown_relations: 0,
        }),
    },
];

fn subject((): Input) -> Result<Value, Failure> {
    let metadata = MetadataUseCaseImpl;
            let described = super::fixtures::contract::check(&metadata);
            let (mut incomplete, mut unknown_relations) = (0, 0);
            for operator in metadata.operators() {
                let description = metadata.describe(*operator);
                incomplete += usize::from(
                    description.name.0.is_empty()
                        || description.summary.0.is_empty()
                        || !description.example.0.contains("example"),
                );
                unknown_relations += example_unknown_relations(&description.example.0);
            }
            Ok(Catalog {
                operators: Cow::Borrowed(metadata.operators()),
                described,
                incomplete,
                unknown_relations,
            })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}