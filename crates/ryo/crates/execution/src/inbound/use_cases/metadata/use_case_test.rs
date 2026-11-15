//! Each pure family is exercised through the public inbound implementation.
use super::fixtures::{Expected, Refusal, example_unknown_relations, plan, vocabularies};
use super::interface::{
    Bearing, Blocking, CoverageScope, Governing, Lacking, Mask, Partial, PlacedScopes, Recovery,
    RowsFed, RuleScope,
};
use super::types::{AffectedFiles, CoverageFileCount};
use super::use_case::MetadataUseCaseImpl;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::metadata::interface::{KindOrigin, MetadataUseCase as _};
use ryo_language::domain::files::types::FilePath;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::{Glob, GlobsMatcher, Hits, MatchCandidate};
use ryo_values::domain::query_operators::types::{OperationKind, QueryOperator};
use ryo_values::domain::run_metadata::types::Feature;
use ryo_values::domain::runs::types::{RuleId, RuleNumber};
use ryo_values::domain::stored_facts::types::PartialCause;
use std::borrow::Cow;
use std::sync::Arc;

/// What is asked of the metadata use case.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The kinds a plan selecting syntax nodes of kind `fn` asks for, and whether it is refused.
    KnownKind,
    /// The same for the kind `ghost`, which no linked language has.
    UnknownKind,
    /// Places two rule scopes, one including `src/*.rs`, on `src/lib.rs`, then unplaces rule 1.
    Placement,
    /// The partial coverage of a read with syntax errors recovered.
    Recovered,
    /// The partial coverage of a whole read.
    Whole,
    /// A plan reading the function relation, a failed read of it, and the project unknowns.
    Trust,
    /// The work and fed rows of a select fed three rows.
    Features,
    /// Every operator's description.
    Catalog,
    /// Coverage of three files with file 1 affected twice.
    Affected,
    /// Coverage of three files affected by a global cause and the foreign file 9.
    ForeignAffected,
}
type Input = Asked;
type Value = Expected;
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_public_operator_has_its_registered_description_and_example"),
        word: Word::Succeeds,
        input: || Asked::Catalog,
        expect: Expect::Value(Expected::Catalog {
            operators: Cow::Borrowed(&[
                QueryOperator::SpanBetween,
                QueryOperator::SpanBounds,
                QueryOperator::SourceMatches,
                QueryOperator::RenderSyntax,
                QueryOperator::FitsRendered,
                QueryOperator::Replace,
                QueryOperator::OrderedMatch,
                QueryOperator::OrderedDfs,
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
                QueryOperator::Capture,
                QueryOperator::Words,
                QueryOperator::Rank,
                QueryOperator::Axis,
                QueryOperator::Rows,
                QueryOperator::Closure,
                QueryOperator::Cycles,
            ]),
            described: 25,
            incomplete: 0,
            unknown_relations: 0,
        }),
    },
    Case {
        name: CaseName("a linked kind is accepted"),
        word: Word::Succeeds,
        input: || Asked::KnownKind,
        expect: Expect::Value(Expected::KnownKind {
            asked: 1,
            refused: false,
            unknown: false,
        }),
    },
    Case {
        name: CaseName("unknown kind keeps the first rule and word"),
        word: Word::Refuses,
        input: || Asked::UnknownKind,
        expect: Expect::Error(Refusal::Kind {
            message: Cow::Borrowed("kind `ghost` names no kind of rust"),
            origin: KindOrigin::Unrecorded,
            rule: 0,
            word: Cow::Borrowed("ghost"),
            as_message: true,
        }),
    },
    Case {
        name: CaseName("one matcher places rules once and unplace clears only one"),
        word: Word::Succeeds,
        input: || Asked::Placement,
        expect: Expect::Value(Expected::Placement {
            globs: Cow::Borrowed(&[Cow::Borrowed("src/*.rs")]),
            before: 2,
            after: 1,
            governed: true,
        }),
    },
    Case {
        name: CaseName("syntax recovery marks code relations partial"),
        word: Word::Succeeds,
        input: || Asked::Recovered,
        expect: Expect::Value(Expected::Recovery(Cow::Borrowed(&[Relation::Function]))),
    },
    Case {
        name: CaseName("a whole read invents no partial coverage"),
        word: Word::Succeeds,
        input: || Asked::Whole,
        expect: Expect::Value(Expected::Recovery(Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("trusted reads identify the affected project file"),
        word: Word::Succeeds,
        input: || Asked::Trust,
        expect: Expect::Value(Expected::Trust {
            reads: Cow::Borrowed(&[Relation::Function]),
            observed: 0,
            bearing: Bearing::Matters,
            blocking: Blocking::Blocked,
            unknowns: Cow::Borrowed(&[(Cow::Borrowed("src/lib.rs"), Cow::Borrowed("r"), true)]),
        }),
    },
    Case {
        name: CaseName("feature and step work count the same fed rows"),
        word: Word::Succeeds,
        input: || Asked::Features,
        expect: Expect::Value(Expected::Features {
            work: 3,
            select: 3,
            relation: 3,
            catalogued: true,
        }),
    },
    Case {
        name: CaseName("repeated affected file is reported once"),
        word: Word::Succeeds,
        input: || Asked::Affected,
        expect: Expect::Value(Expected::Affected(Cow::Borrowed(&[1]))),
    },
    Case {
        name: CaseName("foreign file refuses even beside global cause"),
        word: Word::Refuses,
        input: || Asked::ForeignAffected,
        expect: Expect::Error(Refusal::Incomplete(Cow::Borrowed(
            "partial project coverage cannot be placed on its declared files",
        ))),
    },
];
/// Asks the metadata use case.
fn subject(asked: Input) -> Result<Value, Failure> {
    let metadata = MetadataUseCaseImpl;
    match asked {
        Asked::Catalog => {
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
            Ok(Expected::Catalog {
                operators: Cow::Borrowed(metadata.operators()),
                described,
                incomplete,
                unknown_relations,
            })
        }

        Asked::KnownKind => {
            let plan = plan(Relation::SyntaxTree, Some("fn"));
            let vocabularies = vocabularies();
            let refusal = metadata.kind_refusal(&plan, &vocabularies, &MatchingUseCaseImpl);
            let plans = [&plan];
            let unknown = metadata.first_unknown(
                &plans,
                (&[Vec::new()][..]).into(),
                &vocabularies,
                &MatchingUseCaseImpl,
            );
            Ok(Expected::KnownKind {
                asked: metadata.kinds_asked(&plan, &[]).len(),
                refused: refusal.is_some(),
                unknown: unknown.is_some(),
            })
        }
        Asked::UnknownKind => {
            let plan = plan(Relation::SyntaxTree, Some("ghost"));
            let vocabularies = vocabularies();
            let plans = [&plan];
            let unknown = metadata.first_unknown(
                &plans,
                (&[Vec::new()][..]).into(),
                &vocabularies,
                &MatchingUseCaseImpl,
            );
            let refusal = metadata.kind_refusal(&plan, &vocabularies, &MatchingUseCaseImpl);
            let (Some(unknown), Some(refusal)) = (unknown.0, refusal.0) else {
                return Ok(Expected::KnownKind {
                    asked: metadata.kinds_asked(&plan, &[]).len(),
                    refused: false,
                    unknown: false,
                });
            };
            Err(Refusal::Kind {
                as_message: refusal == metadata.unknown_kind_message(&unknown.word, &vocabularies),
                message: Cow::Owned(refusal.message.0.to_string()),
                origin: refusal.origin,
                rule: unknown.rule.0,
                word: Cow::Owned(unknown.word.0.to_string()),
            })
        }
        Asked::Placement => {
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
            let globs = placement
                .globs
                .iter()
                .map(|glob| Cow::Owned(glob.0.to_string()));
            let globs = Cow::Owned(globs.collect());
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
            let governed = metadata.governed(&mask) == Governing::Governs;
            let after = (0..2)
                .filter(|rule| metadata.governs(&mask, RuleNumber(*rule)) == Governing::Governs)
                .count();
            Ok(Expected::Placement {
                globs,
                before,
                after,
                governed,
            })
        }
        Asked::Recovered | Asked::Whole => {
            let recovery = if matches!(asked, Asked::Recovered) {
                Recovery::SyntaxErrors
            } else {
                Recovery::Whole
            };
            let partial = metadata.partial_of(&[], recovery, &[Relation::Function]);
            Ok(Expected::Recovery(Cow::Owned(
                partial.iter().map(|read| read.relation).collect(),
            )))
        }
        Asked::Trust => {
            let plan = plan(Relation::Function, None);
            let needs = metadata.scoped_needs(&plan);
            let partial = Partial {
                relation: Relation::Function,
                cause: PartialCause("failed read".into()),
                lacking: Lacking::Any,
                at: crate::inbound::use_cases::metadata::interface::PartialPlace::File(FileId(0)),
            };
            let named = [FilePath("src/lib.rs".into())];
            let partials = [partial.clone()];
            let rules = [RuleId("r".into())];
            let unknowns = metadata.project_unknowns(&partials, &named, &[needs.clone()], &rules);
            let message = metadata.unknown_message(&partial, CoverageScope::File(&named[0]));
            Ok(Expected::Trust {
                reads: Cow::Owned(needs.reads.iter().map(|read| read.relation).collect()),
                observed: metadata.observed_reads(&plan).len(),
                bearing: metadata.matters(&needs.reads[0], &partial),
                blocking: metadata.blocked(&needs.reads, &partials),
                unknowns: Cow::Owned(
                    unknowns
                        .iter()
                        .map(|unknown| {
                            let path = Cow::Owned(unknown.path.0.to_string());
                            let rule = Cow::Owned(unknown.rule.0.to_string());
                            (path, rule, unknown.message == message)
                        })
                        .collect(),
                ),
            })
        }
        Asked::Features => {
            let plan = plan(Relation::Function, None);
            let rules = ryo_values::domain::run_metadata::service::from_plans(&[plan]);
            let fed = [vec![RowsFed([3, 0, 0])]];
            let work = metadata.work_over(&rules, (&fed[..]).into())[0][0].rows.0;
            let rows = metadata.fed_over(&rules, (&fed[..]).into());
            let count = |feature| {
                rows.iter()
                    .find(|item| item.feature == feature)
                    .map_or(0, |item| item.rows.0)
            };
            Ok(Expected::Features {
                catalogued: metadata
                    .feature_catalog()
                    .contains(&Feature::Operation(OperationKind::Select)),
                work,
                select: count(Feature::Operation(OperationKind::Select)),
                relation: count(Feature::Relation(Relation::Function)),
            })
        }
        Asked::Affected | Asked::ForeignAffected => {
            let affected: &[Option<FileId>] = if matches!(asked, Asked::Affected) {
                &[Some(FileId(1)), Some(FileId(1))]
            } else {
                &[None, Some(FileId(9))]
            };
            let ordinals = metadata
                .affected_ordinals(AffectedFiles {
                    affected,
                    file_count: CoverageFileCount(3),
                })
                .map_err(|error| {
                    if let OperatingError::ObservationIncomplete { reason } = error {
                        Refusal::Incomplete(Cow::Owned(reason))
                    } else {
                        Refusal::Coverage(error)
                    }
                })?;
            Ok(Expected::Affected(Cow::Owned(ordinals.0)))
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
