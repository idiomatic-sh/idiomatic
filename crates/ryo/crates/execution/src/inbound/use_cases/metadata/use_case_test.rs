//! Each pure family is exercised through the public inbound implementation.
use crate::inbound::use_cases::metadata::interface::MetadataUseCase as _;
use super::interface::Bearing;
use super::interface::Blocking;
use super::interface::CoverageScope;
use super::interface::Governing;
use super::interface::Lacking;
use super::interface::Mask;
use super::interface::Partial;
use super::interface::PlacedScopes;
use super::interface::Recovery;
use super::interface::RowsFed;
use super::interface::RuleScope;
use super::types::AffectedFiles;
use super::types::CoverageFileCount;
use super::use_case::MetadataUseCaseImpl;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_execution_domain::domain::kind_check::types::KindRefusal;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::files::types::GlobsMatcher;
use ryo_values::domain::files::types::Hits;
use ryo_values::domain::files::types::MatchCandidate;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::run_metadata::types::Feature;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::RuleNumber;
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::selects;
use std::sync::Arc;
use super::fixtures::plan;
use super::fixtures::vocabularies;
use super::fixtures::example_unknown_relations;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    KnownKind,
    UnknownKind,
    Placement,
    Recovered,
    Whole,
    Trust,
    Features,
    Catalog,
    Affected,
    ForeignAffected,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Expected {
    Catalog(std::borrow::Cow<'static, [ryo_values::domain::query_operators::types::QueryOperator]>),
    KnownKind(usize),
    Placement {
        before: usize,
        after: usize,
    },
    Recovery(usize),
    Trust {
        reads: usize,
        unknowns: usize,
    },
    Features {
        work: u64,
        select: u64,
        relation: u64,
    },
    Affected(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Refusal {
    Kind(KindRefusal),
    Coverage(OperatingError),
}
type Input = Scenario;
type Value = Expected;
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_public_operator_has_its_registered_description_and_example"),
        word: Word::Succeeds,
        input: || Scenario::Catalog,
        expect: Expect::Value(Expected::Catalog(std::borrow::Cow::Borrowed(&[
            ryo_values::domain::query_operators::types::QueryOperator::SpanBetween,
            ryo_values::domain::query_operators::types::QueryOperator::SpanBounds,
            ryo_values::domain::query_operators::types::QueryOperator::SourceMatches,
            ryo_values::domain::query_operators::types::QueryOperator::RenderSyntax,
            ryo_values::domain::query_operators::types::QueryOperator::FitsRendered,
            ryo_values::domain::query_operators::types::QueryOperator::Replace,
            ryo_values::domain::query_operators::types::QueryOperator::OrderedMatch,
            ryo_values::domain::query_operators::types::QueryOperator::OrderedDfs,
            ryo_values::domain::query_operators::types::QueryOperator::Select,
            ryo_values::domain::query_operators::types::QueryOperator::Project,
            ryo_values::domain::query_operators::types::QueryOperator::Extend,
            ryo_values::domain::query_operators::types::QueryOperator::Join,
            ryo_values::domain::query_operators::types::QueryOperator::Union,
            ryo_values::domain::query_operators::types::QueryOperator::Intersection,
            ryo_values::domain::query_operators::types::QueryOperator::Difference,
            ryo_values::domain::query_operators::types::QueryOperator::Antijoin,
            ryo_values::domain::query_operators::types::QueryOperator::GroupCount,
            ryo_values::domain::query_operators::types::QueryOperator::GroupDigest,
            ryo_values::domain::query_operators::types::QueryOperator::Capture,
            ryo_values::domain::query_operators::types::QueryOperator::Words,
            ryo_values::domain::query_operators::types::QueryOperator::Rank,
            ryo_values::domain::query_operators::types::QueryOperator::Axis,
            ryo_values::domain::query_operators::types::QueryOperator::Rows,
            ryo_values::domain::query_operators::types::QueryOperator::Closure,
            ryo_values::domain::query_operators::types::QueryOperator::Cycles,
        ]))),
    },
    Case {
        name: CaseName("a linked kind is accepted"),
        word: Word::Succeeds,
        input: || Scenario::KnownKind,
        expect: Expect::Value(Expected::KnownKind(1)),
    },
    Case {
        name: CaseName("unknown kind keeps the first rule and word"),
        word: Word::Refuses,
        input: || Scenario::UnknownKind,
        expect: Expect::Refusal(|failure| {
            selects!(
                matches!(failure, Refusal::Kind(KindRefusal { message, origin: ryo_execution_domain::domain::kind_check::types::KindOrigin::Unrecorded }) if message.0.contains("ghost") && message.0.contains("rust")),
            )
        }),
    },
    Case {
        name: CaseName("one matcher places rules once and unplace clears only one"),
        word: Word::Succeeds,
        input: || Scenario::Placement,
        expect: Expect::Value(Expected::Placement {
            before: 2,
            after: 1,
        }),
    },
    Case {
        name: CaseName("syntax recovery marks code relations partial"),
        word: Word::Succeeds,
        input: || Scenario::Recovered,
        expect: Expect::Value(Expected::Recovery(1)),
    },
    Case {
        name: CaseName("a whole read invents no partial coverage"),
        word: Word::Succeeds,
        input: || Scenario::Whole,
        expect: Expect::Value(Expected::Recovery(0)),
    },
    Case {
        name: CaseName("trusted reads identify the affected project file"),
        word: Word::Succeeds,
        input: || Scenario::Trust,
        expect: Expect::Value(Expected::Trust {
            reads: 1,
            unknowns: 1,
        }),
    },
    Case {
        name: CaseName("feature and step work count the same fed rows"),
        word: Word::Succeeds,
        input: || Scenario::Features,
        expect: Expect::Value(Expected::Features {
            work: 3,
            select: 3,
            relation: 3,
        }),
    },
    Case {
        name: CaseName("repeated affected file is reported once"),
        word: Word::Succeeds,
        input: || Scenario::Affected,
        expect: Expect::Value(Expected::Affected(1)),
    },
    Case {
        name: CaseName("foreign file refuses even beside global cause"),
        word: Word::Refuses,
        input: || Scenario::ForeignAffected,
        expect: Expect::Refusal(|failure| {
            selects!(matches!(
                failure,
                Refusal::Coverage(OperatingError::ObservationIncomplete { .. })
            ))
        }),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    let metadata = MetadataUseCaseImpl;
    match scenario {
        Scenario::Catalog => {
            let described = super::fixtures::contract::check(&metadata);
            assert_eq!(described, metadata.operators().len());
            let mut unknown_relations = 0;
            for operator in metadata.operators() {
                let description = metadata.describe(*operator);
                assert!(!description.name.0.is_empty());
                assert!(!description.summary.0.is_empty());
                assert!(description.example.0.contains("example"));
                unknown_relations += example_unknown_relations(&description.example.0);
            }
            assert_eq!(
                unknown_relations, 0,
                "every example names only actual catalog relations"
            );
            Ok(Expected::Catalog(std::borrow::Cow::Borrowed(
                metadata.operators(),
            )))
        }

        Scenario::KnownKind => {
            let plan = plan(Relation::SyntaxTree, Some("fn"));
            assert!(metadata.kind_refusal(&plan, &vocabularies(), &MatchingUseCaseImpl).is_none());
            assert!(
                metadata
                    .first_unknown(&[plan.clone()], &[Vec::new()], &vocabularies(), &MatchingUseCaseImpl)
                    .is_none()
            );
            Ok(Expected::KnownKind(metadata.kinds_asked(&plan, &[]).len()))
        }
        Scenario::UnknownKind => {
            let plan = plan(Relation::SyntaxTree, Some("ghost"));
            let unknown = metadata
                .first_unknown(&[plan.clone()], &[Vec::new()], &vocabularies(), &MatchingUseCaseImpl)
                .expect("the first unknown kind");
            assert_eq!(unknown.rule.0, 0);
            assert_eq!(&*unknown.word.0, "ghost");
            let refusal = metadata
                .kind_refusal(&plan, &vocabularies(), &MatchingUseCaseImpl)
                .expect("unknown kind refuses");
            assert_eq!(
                refusal,
                metadata.unknown_kind_message(&unknown.word, &vocabularies())
            );
            Err(Refusal::Kind(refusal))
        }
        Scenario::Placement => {
            let placement = metadata.decide(&[
                RuleScope {
                    include: vec![],
                    exclude: vec![],
                },
                RuleScope {
                    include: vec![Glob("src/*.rs".into())],
                    exclude: vec![],
                },
            ]);
            assert_eq!(placement.globs, vec![Glob("src/*.rs".into())]);
            let placed = PlacedScopes {
                placement,
                matcher: GlobsMatcher {
                    matches: Arc::new(|candidate: MatchCandidate<'_>, hits: &mut Hits| {
                        if candidate.0 == "src/lib.rs" {
                            hits.0.push(0);
                        }
                    }),
                },
            };
            let (mut mask, mut hits) = (Mask::default(), Hits::default());
            metadata.place(
                &placed,
                &FilePath("src/lib.rs".into()),
                &mut mask,
                &mut hits,
            );
            let before = (0..2)
                .filter(|rule| metadata.governs(&mask, RuleNumber(*rule)) == Governing::Governs)
                .count();
            metadata.unplace(&mut mask, RuleNumber(1));
            assert_eq!(metadata.governed(&mask), Governing::Governs);
            let after = (0..2)
                .filter(|rule| metadata.governs(&mask, RuleNumber(*rule)) == Governing::Governs)
                .count();
            Ok(Expected::Placement { before, after })
        }
        Scenario::Recovered | Scenario::Whole => {
            let recovery = if matches!(scenario, Scenario::Recovered) {
                Recovery::SyntaxErrors
            } else {
                Recovery::Whole
            };
            let partial = metadata.partial_of(&[], recovery, &[Relation::Function]);
            if let Some(first) = partial.first() {
                assert_eq!(first.relation, Relation::Function);
            }
            Ok(Expected::Recovery(partial.len()))
        }
        Scenario::Trust => {
            let plan = plan(Relation::Function, None);
            let needs = metadata.scoped_needs(&plan);
            assert_eq!(needs.reads[0].relation, Relation::Function);
            assert!(metadata.observed_reads(&plan).is_empty());
            let partial = Partial {
                relation: Relation::Function,
                cause: PartialCause("failed read".into()),
                lacking: Lacking::Any,
                at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(0)),
            };
            assert_eq!(
                metadata.matters(&needs.reads[0], &partial),
                Bearing::Matters
            );
            assert_eq!(
                metadata.blocked(&needs.reads, &[partial.clone()]),
                Blocking::Blocked
            );
            let named = [FilePath("src/lib.rs".into())];
            let unknowns = metadata.project_unknowns(
                &[partial.clone()],
                &named,
                &[needs.clone()],
                &[RuleId("r".into())],
            );
            assert_eq!(unknowns[0].path, named[0]);
            assert_eq!(unknowns[0].rule, RuleId("r".into()));
            assert_eq!(
                metadata.unknown_message(&partial, CoverageScope::File(&named[0])),
                unknowns[0].message
            );
            Ok(Expected::Trust {
                reads: needs.reads.len(),
                unknowns: unknowns.len(),
            })
        }
        Scenario::Features => {
            let plan = plan(Relation::Function, None);
            let rules = ryo_values::domain::run_metadata::service::from_plans(&[plan]);
            let fed = [vec![RowsFed([3, 0, 0])]];
            let work = metadata.work_over(&rules, &fed)[0][0].rows.0;
            let rows = metadata.fed_over(&rules, &fed);
            let count = |feature| {
                rows.iter()
                    .find(|item| item.feature == feature)
                    .map_or(0, |item| item.rows.0)
            };
            assert!(
                metadata
                    .feature_catalog()
                    .contains(&Feature::Operation(OperationKind::Select))
            );
            Ok(Expected::Features {
                work,
                select: count(Feature::Operation(OperationKind::Select)),
                relation: count(Feature::Relation(Relation::Function)),
            })
        }
        Scenario::Affected | Scenario::ForeignAffected => {
            let affected: &[Option<FileId>] = if matches!(scenario, Scenario::Affected) {
                &[Some(FileId(1)), Some(FileId(1))]
            } else {
                &[None, Some(FileId(9))]
            };
            let ordinals = metadata
                .affected_ordinals(AffectedFiles {
                    affected,
                    file_count: CoverageFileCount(3),
                })
                .map_err(Refusal::Coverage)?;
            assert_eq!(ordinals.0, vec![1]);
            Ok(Expected::Affected(ordinals.0.len()))
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
