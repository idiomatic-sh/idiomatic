//! The test data of `linter lint`, compiled only for tests and under the `fixtures` feature.
#![cfg(any(test, feature = "fixtures"))]

/// The scenarios the rows of `entry_test` run: each builds its own linter and repository and asserts what the entry promises.
#[cfg(test)]
pub(crate) mod scenarios {
    use std::collections::BTreeMap;
    use std::num::NonZeroUsize;

    use crate::domain::capacity::types::{MemoryBudget, WorkerPool};
    use crate::domain::cell::types::{Cell, HostText, Slot};
    use crate::domain::file::types::RelativePath;
    use crate::domain::host::types::{HostColumn, HostColumnName, HostKind, HostRelationName};
    use crate::domain::language::types::{Language, LanguageBuild, LanguageName, Languages};
    use crate::domain::observation::types::{
        ObservationBindingName, ObservationColumn, ObservationColumnName, ObservationKind,
        ObservationName,
    };
    use crate::resources::linter::RuleApplicability;
    use crate::resources::linter::lint::entry::lint_with;
    use crate::resources::linter::lint::errors::LintError;
    use crate::resources::linter::lint::types::{
        Cache, CaseSource, Corpus, GivenProjectInventory, Glob, HostRelation, HostRow, LintReport,
        LintRequest, Linter, ObservationBinding, ObservationProducer, ParameterName, ParameterText,
        ParameterValue, ProjectInventoryCompleteness, ProjectSearchScope, Rule, RuleId, SourceText,
        Workers,
    };
    use crate::resources::query::RyoQlQuery;
    use ryo_relations::domain::relations::types::OptionalCapability;
    use ryo_values::domain::files::types::ExecutablePermission;

    /// What a scenario answers: how many findings, how many files nothing read, whether the repository holds nothing but what it held,
    /// and how many columns the first finding carries.
    pub(crate) type Outcome = (usize, usize, bool, usize);

    /// The host's placed files, read by a rule.
    pub(crate) const HOSTED: &str =
        "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"spec\");";
    /// Every identifier of the files, which the optional identifier capability promises.
    pub(crate) const OPTIONAL_POSITIVE: &str =
        "let names = select * from \"identifier\"; emit names subject file key(span);";
    /// A syntactic raw-format outcome requires its advertised optional capability.
    pub(crate) const RAW_FORMAT_POSITIVE: &str = "let segments = select * from \"raw-format-segment\" where outcome = \"parsed\"; emit segments subject file key(span);";
    /// A syntactic literal outcome requires its advertised optional capability.
    pub(crate) const LITERAL_PARSE_POSITIVE: &str = "let parsed = select * from \"literal-expression-parse\" where outcome = \"parsed\"; emit parsed subject file key(span);";

    /// The two calls the shared-linter row tells apart: one rule id, two queries, two declarations of the host, two sets of globs and
    /// two sets of files given as text.
    fn shared_calls(root: &std::path::Path) -> [LintRequest; 2] {
        let rule = |query: &str, include: &str| Rule {
            observations: Vec::new(),
            query: crate::resources::query::RyoQlQuery::try_from(query).expect("a query"),
            applicability: crate::resources::linter::RuleApplicability::Always,
            parameters: BTreeMap::new(),
            include: vec![
                crate::resources::linter::lint::types::Glob::try_from(include).expect("a glob"),
            ],
            exclude: Vec::new(),
        };
        let placed = |file: &str, spec: &str| HostRelation {
            name: HostRelationName("host.placed".into()),
            columns: vec![
                HostColumn {
                    name: HostColumnName("path".into()),
                    kind: HostKind::File,
                },
                HostColumn {
                    name: HostColumnName("spec".into()),
                    kind: HostKind::Text,
                },
            ],
            rows: vec![HostRow(vec![
                Cell::File(RelativePath(file.into())),
                Cell::Text(HostText(spec.into())),
            ])],
        };
        let given = |file: &str| {
            Corpus::Given(vec![CaseSource {
                executable: Default::default(),
                path: RelativePath(file.into()),
                text: SourceText("text".into()),
            }])
        };
        let request = |query: &str, include: &str, file: &str, spec: &str| LintRequest {
            project_search_scope: Default::default(),
            given_project_inventory: Default::default(),
            source_language: None,
            root: root.to_path_buf(),
            rules: BTreeMap::from([(
                RuleId::try_from("rule").expect("a rule id"),
                rule(query, include),
            )]),
            files: Vec::new(),
            hosted: vec![placed(file, spec)],
            corpus: given(file),
            beside: Vec::new(),
        };
        let other = "let placed = select * from \"host.placed\"; emit placed subject \"path\" key(\"path\", \"spec\");";
        [
            request(HOSTED, "*.txt", "from-memory.txt", "docs"),
            request(other, "*.md", "other.md", "notes"),
        ]
    }

    /// Two different calls on one linter, one after the other, interleaved and in parallel, give exactly what the same calls give on a
    /// fresh linter each: nothing a call leaves in the linter reaches another. The value is the first call's, over a directory holding
    /// one file.
    pub(crate) fn shared() -> Outcome {
        let root = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(root.path().join("notes.txt"), "text").expect("a file");
        let linter = || {
            crate::resources::linter::lint::types::Linter::new(
                Languages::default(),
                crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
                MemoryBudget::Machine,
                Cache::None,
            )
        };
        let fresh = |request: LintRequest| {
            crate::resources::linter::lint::entry::lint_with(&linter(), request).expect("a report")
        };
        let [first, second] = shared_calls(root.path());
        let expected = [fresh(first.clone()), fresh(second.clone())];
        assert_ne!(
            expected[0], expected[1],
            "the two calls tell different things"
        );
        let shared = linter();
        let call = |request: &LintRequest| {
            crate::resources::linter::lint::entry::lint_with(&shared, request.clone())
                .expect("a report")
        };
        for round in 0_u8..3 {
            assert_eq!(call(&first), expected[0], "round {round}, first");
            assert_eq!(call(&second), expected[1], "round {round}, second");
            assert_eq!(call(&second), expected[1], "round {round}, second again");
            assert_eq!(call(&first), expected[0], "round {round}, first again");
        }
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0_usize..16)
                .map(|each| {
                    let (request, want) = if each % 2_usize == 0 {
                        (&first, &expected[0])
                    } else {
                        (&second, &expected[1])
                    };
                    let call = &call;
                    scope.spawn(move || assert_eq!(&call(request), want, "parallel call {each}"))
                })
                .collect();
            for handle in handles {
                handle.join().expect("a call that did not panic");
            }
        });
        let untouched = std::fs::read_dir(root.path()).is_ok_and(|entries| entries.count() == 1);
        let columns = expected[0]
            .findings
            .first()
            .map_or(0, |finding| finding.columns.len());
        (
            expected[0].findings.len(),
            expected[0].unread.len(),
            untouched,
            columns,
        )
    }

    /// A provider with the same name/build whose advertised optional contract changes.
    struct CapabilityProvider(
        &'static [ryo_relations::domain::relations::types::OptionalCapability],
        &'static str,
        &'static str,
    );
    impl ryo_language::inbound::use_cases::parse::types::ParseEngine for CapabilityProvider {
        fn begin_semantics(
            &self,
            _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
            _: &ryo_language::domain::elements::types::Elements,
        ) -> Option<Box<dyn ryo_language::domain::semantics::interface::SemanticSession>> {
            None
        }
        fn capabilities(
            &self,
        ) -> &'static [ryo_relations::domain::relations::types::OptionalCapability] {
            self.0
        }
        fn configuration_identity(
            &self,
        ) -> ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity<'_>
        {
            ryo_language::inbound::use_cases::parse::types::ParseConfigurationIdentity(self.1)
        }
        fn recognition(
            &self,
            _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        ) -> ryo_language::inbound::use_cases::parse::types::Recognition {
            ryo_language::inbound::use_cases::parse::types::Recognition::Reads
        }
        fn skips(
            &self,
        ) -> &'static [ryo_language::inbound::use_cases::parse::types::SkippedDirectory] {
            &[]
        }
        fn kind_count(&self) -> ryo_relations::domain::relations::types::Count {
            ryo_relations::domain::relations::types::Count(1)
        }
        fn kind_names(&self) -> ryo_language::inbound::use_cases::parse::types::KindNames {
            ryo_language::inbound::use_cases::parse::types::KindNames(vec![
                ryo_relations::domain::terms::types::NameText("root"),
            ])
        }
        fn unknown_reasons(&self) -> &'static [ryo_relations::domain::terms::types::UnknownName] {
            &[]
        }
        fn reader_kinds(&self) -> Vec<ryo_relations::domain::relations::types::KindBits> {
            vec![ryo_relations::domain::relations::types::KindBits(0)]
        }
        fn build_file(
            &self,
            request: ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
            table: &mut ryo_language::domain::elements::types::Elements,
            symbols: &mut ryo_language::domain::symbols::types::Symbols,
        ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
            use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder};
            use ryo_relations::domain::relations::types::{KindBits, Offset};
            let mut builder = table_builder(table, symbols, request.bytes);
            enter_node(&mut builder, KindBits(0), Offset(0));
            leave_node(&mut builder, Offset(1));
            ryo_language::domain::completion::service::name_kind(
                &mut ryo_language::domain::completion::service::completion(
                    table,
                    symbols,
                    request.bytes,
                    request.file,
                ),
                KindBits(0),
                ryo_relations::domain::terms::types::NameText("root"),
            );
            Ok(())
        }
        fn baseline_file(
            &self,
            _: &ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
        ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
            Ok(())
        }
        fn read_file(
            &self,
            request: ryo_language::inbound::use_cases::parse::types::ParseRequest<'_>,
            table: &mut ryo_language::domain::elements::types::Elements,
            symbols: &mut ryo_language::domain::symbols::types::Symbols,
            wanted: &[ryo_relations::domain::relations::types::Relation],
            out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
        ) -> Result<(), ryo_language::inbound::use_cases::parse::errors::ParseError> {
            let source = request.bytes;
            let file = request.file;
            self.build_file(request, table, symbols)?;
            self.walk_file(table, symbols, source, file, wanted, out);
            Ok(())
        }
        fn walk_file(
            &self,
            _: &mut ryo_language::domain::elements::types::Elements,
            symbols: &mut ryo_language::domain::symbols::types::Symbols,
            source: ryo_language::domain::symbols::types::ByteSlice<'_>,
            file: ryo_relations::domain::relations::types::FileId,
            wanted: &[ryo_relations::domain::relations::types::Relation],
            out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
        ) {
            use ryo_relations::domain::relations::{
                collections::relations::identifier::item::IdentifierRow,
                types::{
                    Missing, Offset, OptionalCapability, Position, Relation, ReportText, Rows, Span,
                },
            };
            use ryo_relations::domain::terms::types::NameText;
            if wanted.contains(&Relation::Identifier) {
                if self.0.contains(&OptionalCapability::Identifier) {
                    out.write(IdentifierRow {
                        id: ryo_relations::domain::relations::service::node_id(file, Position(0)),
                        name: ryo_language::domain::symbols::service::intern_derived(
                            symbols,
                            source,
                            NameText(self.2),
                        ),
                        file,
                        span: Span {
                            start: Offset(0),
                            end: Offset(1),
                        },
                    });
                } else {
                    <dyn ryo_relations::domain::relations::types::RowWriter as Rows<
                        IdentifierRow,
                    >>::partial_at(
                        out,
                        file,
                        ReportText("unadvertised fixture capability"),
                        Missing::Any,
                    );
                }
            }
            if wanted.contains(&Relation::LiteralExpressionParse) {
                literal_parse_capability_row(
                    self.0.contains(&OptionalCapability::LiteralExpressionParse),
                    source,
                    file,
                    symbols,
                    out,
                );
            }
            if wanted.contains(&Relation::RawFormatSegment) {
                use ryo_relations::domain::relations::collections::relations::raw_format_segment::item::RawFormatSegmentRow;
                if self.0.contains(&OptionalCapability::RawFormatSegment) {
                    let intern =
                        |interned_symbols: &mut ryo_language::domain::symbols::types::Symbols,
                         text: &str| {
                            ryo_language::domain::symbols::service::intern_derived(
                                interned_symbols,
                                source,
                                NameText(text),
                            )
                        };
                    let id = ryo_relations::domain::relations::service::node_id(file, Position(0));
                    out.write(RawFormatSegmentRow {
                        id,
                        segment: id,
                        segment_ordinal: ryo_relations::domain::relations::types::Integer(0),
                        dialect: intern(symbols, "percent-string"),
                        outcome: intern(symbols, "parsed"),
                        error_category: intern(symbols, "none"),
                        error_character: intern(symbols, ""),
                        error_index: ryo_relations::domain::relations::types::Integer(0),
                        positional_consumption: ryo_relations::domain::relations::types::Integer(0),
                        mapping_present: ryo_relations::domain::relations::types::Flag::Lowered,
                        dynamic_width: ryo_relations::domain::relations::types::Flag::Lowered,
                        dynamic_precision: ryo_relations::domain::relations::types::Flag::Lowered,
                        file,
                        span: Span {
                            start: Offset(0),
                            end: Offset(1),
                        },
                    });
                } else {
                    <dyn ryo_relations::domain::relations::types::RowWriter as Rows<
                        RawFormatSegmentRow,
                    >>::partial_at(
                        out,
                        file,
                        ReportText("unadvertised raw format capability"),
                        Missing::Any,
                    );
                }
            }
        }
        fn absence_of(
            &self,
            relation: ryo_relations::domain::relations::types::Relation,
        ) -> Option<ryo_relations::domain::answers::types::AbsenceCause> {
            use ryo_relations::domain::{
                answers::types::AbsenceCause,
                relations::types::{OptionalCapability, Relation},
            };
            ((relation == Relation::Identifier
                && !self.0.contains(&OptionalCapability::Identifier))
                || (relation == Relation::RawFormatSegment
                    && !self.0.contains(&OptionalCapability::RawFormatSegment))
                || (relation == Relation::LiteralExpressionParse
                    && !self.0.contains(&OptionalCapability::LiteralExpressionParse)))
            .then_some(AbsenceCause::NotYetRead)
        }
    }

    /// Emit or explicitly refuse the optional literal fact in the persistent-identity fixture.
    fn literal_parse_capability_row(
        available: bool,
        source: ryo_language::domain::symbols::types::ByteSlice<'_>,
        file: ryo_relations::domain::relations::types::FileId,
        symbols: &mut ryo_language::domain::symbols::types::Symbols,
        out: &mut dyn ryo_relations::domain::relations::types::RowWriter,
    ) {
        use ryo_relations::domain::relations::{
            collections::relations::literal_expression_parse::item::LiteralExpressionParseRow,
            types::{Missing, Offset, Position, ReportText, Rows, Span},
        };
        use ryo_relations::domain::terms::types::NameText;
        if available {
            let dialect = ryo_language::domain::symbols::service::intern_derived(
                symbols,
                source,
                NameText("python-string-annotation"),
            );
            let outcome = ryo_language::domain::symbols::service::intern_derived(
                symbols,
                source,
                NameText("parsed"),
            );
            out.write(LiteralExpressionParseRow {
                id: ryo_relations::domain::relations::service::node_id(file, Position(0)),
                dialect,
                outcome,
                file,
                span: Span {
                    start: Offset(0),
                    end: Offset(1),
                },
            });
        } else {
            <dyn ryo_relations::domain::relations::types::RowWriter as Rows<
                LiteralExpressionParseRow,
            >>::partial_at(
                out,
                file,
                ReportText("unadvertised literal parse capability"),
                Missing::Any,
            );
        }
    }

    /// Same-process promises must not replay findings retained by a different advertised contract.
    #[expect(
        clippy::too_many_lines,
        reason = "this regression keeps cold, warm, and changed-capability cache paths in one scenario"
    )]
    pub(crate) fn capability_cache(query: &str) -> Outcome {
        use ryo_relations::domain::relations::types::OptionalCapability;
        let advertised_capabilities: &'static [OptionalCapability] = if query == RAW_FORMAT_POSITIVE
        {
            &[
                OptionalCapability::RawFormatSegment,
                OptionalCapability::RawFormatField,
            ]
        } else if query == LITERAL_PARSE_POSITIVE {
            &[OptionalCapability::LiteralExpressionParse]
        } else {
            &[OptionalCapability::Identifier]
        };
        let root = tempfile::tempdir().expect("corpus");
        let store = tempfile::tempdir().expect("store");
        std::fs::write(root.path().join("case.txt"), "x").expect("source");
        let language = |caps, configuration, name| {
            crate::domain::language::types::Language::from_parts(
                crate::domain::language::types::LanguageName("fixture".into()),
                crate::domain::language::types::LanguageBuild("same-build".into()),
                std::sync::Arc::new(CapabilityProvider(caps, configuration, name)),
                std::sync::Arc::new(
                    ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
                ),
            )
        };
        let engine = |caps, configuration, name, cache| {
            crate::resources::linter::lint::types::Linter::new(
                Languages(vec![language(caps, configuration, name)]),
                crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
                MemoryBudget::Machine,
                cache,
            )
        };
        let request = LintRequest {
            project_search_scope: Default::default(),
            given_project_inventory: Default::default(),
            source_language: None,
            root: root.path().to_path_buf(),
            rules: BTreeMap::from([(
                RuleId::try_from("optional").expect("rule"),
                Rule {
                    observations: Vec::new(),
                    query: crate::resources::query::RyoQlQuery::try_from(query).expect("query"),
                    applicability: crate::resources::linter::RuleApplicability::Always,
                    parameters: BTreeMap::new(),
                    include: vec![
                        crate::resources::linter::lint::types::Glob::try_from("*.txt")
                            .expect("glob"),
                    ],
                    exclude: vec![],
                },
            )]),
            files: vec![],
            hosted: vec![],
            corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
            beside: vec![],
        };
        let forward = crate::resources::linter::lint::connections::build(&[language(
            &[
                OptionalCapability::Identifier,
                OptionalCapability::QualifiedName,
            ],
            "",
            "x",
        )]);
        let reverse = crate::resources::linter::lint::connections::build(&[language(
            &[
                OptionalCapability::QualifiedName,
                OptionalCapability::Identifier,
            ],
            "",
            "x",
        )]);
        assert_eq!(
            forward, reverse,
            "capability declaration order does not change identity"
        );

        let advertised = engine(
            advertised_capabilities,
            "",
            "x",
            Cache::At(store.path().to_path_buf()),
        );
        let first = crate::resources::linter::lint::entry::lint_with(&advertised, request.clone())
            .expect("advertised report");
        assert_eq!(
            first.findings.len(),
            1,
            "the cache must contain a real optional finding"
        );
        let mut selected = request.clone();
        selected.source_language = Some(crate::domain::language::types::LanguageName(
            "fixture".into(),
        ));
        let explicit =
            crate::resources::linter::lint::entry::lint_with(&advertised, selected.clone())
                .expect("explicit refusal report");
        assert!(
            explicit.findings.is_empty(),
            "cached path-recognized findings cannot bypass explicit opt-in"
        );
        assert_eq!(
            explicit.unread.len(),
            1,
            "a non-opting provider must refuse explicit selection"
        );
        let fresh_explicit = crate::resources::linter::lint::entry::lint_with(
            &engine(advertised_capabilities, "", "x", Cache::None),
            selected.clone(),
        )
        .expect("fresh explicit refusal");
        assert_eq!(
            explicit, fresh_explicit,
            "source selection must be in cache identity"
        );
        assert_eq!(
            crate::resources::linter::lint::entry::lint_with(&advertised, request.clone())
                .expect("ordinary selection restored"),
            first
        );
        selected.source_language = Some(crate::domain::language::types::LanguageName(
            "missing-provider".into(),
        ));
        assert!(matches!(
            crate::resources::linter::lint::entry::lint_with(&advertised, selected),
            Err(crate::resources::linter::lint::errors::LintError::SourceLanguage { .. })
        ));

        let ambiguous = crate::resources::linter::lint::types::Linter::new(
            Languages(vec![
                language(advertised_capabilities, "", "x"),
                language(advertised_capabilities, "", "x"),
            ]),
            crate::domain::capacity::types::WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            Cache::None,
        );
        let mut ambiguous_request = request.clone();
        ambiguous_request.source_language = Some(crate::domain::language::types::LanguageName(
            "fixture".into(),
        ));
        assert!(matches!(
            crate::resources::linter::lint::entry::lint_with(&ambiguous, ambiguous_request),
            Err(crate::resources::linter::lint::errors::LintError::SourceLanguage { .. })
        ));

        let absent = engine(&[], "", "x", Cache::At(store.path().to_path_buf()));
        let replay = crate::resources::linter::lint::entry::lint_with(&absent, request.clone())
            .expect("unadvertised report");
        let fresh = crate::resources::linter::lint::entry::lint_with(
            &engine(&[], "", "x", Cache::None),
            request.clone(),
        )
        .expect("fresh unadvertised report");
        assert_eq!(
            replay, fresh,
            "retained data must not resurrect unadvertised findings"
        );
        assert!(
            fresh.findings.is_empty(),
            "unadvertised means unavailable, not a positive"
        );
        assert!(
            !fresh.unknowns.is_empty(),
            "unavailability remains observable"
        );
        let restored = engine(
            advertised_capabilities,
            "",
            "x",
            Cache::At(store.path().to_path_buf()),
        );
        assert_eq!(
            crate::resources::linter::lint::entry::lint_with(&restored, request.clone())
                .expect("restored report"),
            first,
            "prior unavailable state cannot poison newly advertised capability results"
        );

        if query == OPTIONAL_POSITIVE {
            let target_310 = engine(
                &[OptionalCapability::Identifier],
                "python-target=3.10",
                "x",
                Cache::At(store.path().to_path_buf()),
            );
            let before_target_change =
                crate::resources::linter::lint::entry::lint_with(&target_310, request.clone())
                    .expect("Python 3.10 result");
            let target_314 = engine(
                &[OptionalCapability::Identifier],
                "python-target=3.14",
                "y",
                Cache::At(store.path().to_path_buf()),
            );
            let after_target_change =
                crate::resources::linter::lint::entry::lint_with(&target_314, request.clone())
                    .expect("Python 3.14 result");
            let fresh_314 = crate::resources::linter::lint::entry::lint_with(
                &engine(
                    &[OptionalCapability::Identifier],
                    "python-target=3.14",
                    "y",
                    Cache::None,
                ),
                request.clone(),
            )
            .expect("fresh Python 3.14 result");
            assert_ne!(before_target_change, after_target_change);
            assert_eq!(after_target_change, fresh_314);
            let restored_310 = crate::resources::linter::lint::entry::lint_with(
                &engine(
                    &[OptionalCapability::Identifier],
                    "python-target=3.10",
                    "x",
                    Cache::At(store.path().to_path_buf()),
                ),
                request.clone(),
            )
            .expect("restored Python 3.10 result");
            assert_eq!(restored_310, before_target_change);
            let stable_identity = crate::resources::linter::lint::connections::build(&[language(
                &[OptionalCapability::Identifier],
                "python-target=3.14",
                "y",
            )]);
            assert_eq!(
                stable_identity,
                crate::resources::linter::lint::connections::build(&[language(
                    &[OptionalCapability::Identifier],
                    "python-target=3.14",
                    "y",
                )]),
                "unchanged parser settings keep the same cache identity"
            );
            assert_ne!(
                stable_identity,
                crate::resources::linter::lint::connections::build(&[language(
                    &[OptionalCapability::Identifier],
                    "python-target=3.10",
                    "x",
                )]),
                "changed parser settings change cache identity"
            );
        }
        (1, 0, true, 0)
    }

    /// What a scenario answers of its last report.
    fn summary(report: &LintReport) -> Outcome {
        (
            report.findings.len(),
            report.unread.len(),
            true,
            report
                .findings
                .first()
                .map_or(0, |finding| finding.columns.len()),
        )
    }

    /// A linter over one fixture provider, retaining into `cache`: `build` and `identity` are what its cache identity is made of, `present`
    /// the name its identifier answers.
    fn provider_linter(
        build: &str,
        identity: &'static str,
        capabilities: &'static [OptionalCapability],
        present: &'static str,
        cache: Cache,
    ) -> Linter {
        Linter::new(
            Languages(vec![Language::from_parts(
                LanguageName("fixture".into()),
                LanguageBuild(build.into()),
                std::sync::Arc::new(CapabilityProvider(capabilities, identity, present)),
                std::sync::Arc::new(
                    ryo_language::inbound::use_cases::project::use_case::ProjectUseCaseImpl,
                ),
            )]),
            WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    }

    /// A rule over `query` that observes nothing, binds no parameter and filters no file.
    fn ordinary(query: &str) -> Rule {
        Rule {
            observations: Vec::new(),
            query: RyoQlQuery::try_from(query).expect("a query"),
            applicability: RuleApplicability::Always,
            parameters: BTreeMap::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }

    /// A request over the one listed file `case.txt` of `root`, running `rules` with nothing hosted.
    fn listed(root: &std::path::Path, rules: BTreeMap<RuleId, Rule>) -> LintRequest {
        LintRequest {
            project_search_scope: ProjectSearchScope::Undeclared,
            given_project_inventory: GivenProjectInventory {
                entries: Vec::new(),
                completeness: ProjectInventoryCompleteness::Partial,
            },
            source_language: None,
            root: root.to_owned(),
            files: Vec::new(),
            beside: Vec::new(),
            hosted: Vec::new(),
            corpus: Corpus::Listed(vec![RelativePath("case.txt".into())]),
            rules,
        }
    }

    /// A directory holding the one file `case.txt`.
    fn case_directory() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("corpus");
        std::fs::write(root.path().join("case.txt"), "x").expect("source");
        root
    }

    /// A rule id.
    fn rule_id(text: &str) -> RuleId {
        RuleId::try_from(text).expect("a rule id")
    }

    /// The binding of an observation named `incoming` to the output `shared` of `producer`.
    fn incoming(columns: Vec<ObservationColumn>, producer: RuleId) -> ObservationBinding {
        ObservationBinding {
            name: ObservationBindingName("incoming".into()),
            columns,
            producers: vec![ObservationProducer::Output {
                rule: producer,
                output: ObservationName("shared".into()),
            }],
        }
    }

    /// A column of an observation.
    fn column(name: &str, kind: ObservationKind) -> ObservationColumn {
        ObservationColumn {
            name: ObservationColumnName(name.into()),
            kind,
        }
    }

    /// Bytes that are unchanged and a timestamp that is given never reuse a finding of an old mode.
    pub(crate) fn permission_bytes() -> Outcome {
        let root = tempfile::tempdir().expect("root");
        let cache = tempfile::tempdir().expect("cache");
        let shared = permission_linter(Cache::At(cache.path().join("store")));
        let mut last = (0, 0, true, 0);
        let query = "let files = select path as file from \"filesystem.executable\" where executable = \"true\"; emit files subject file key(file);";
        for executable in [false, true, true, false, false, true] {
            let current =
                permission_request(
                root.path(),
                if executable {
                    ExecutablePermission::PosixExecutable
                } else {
                    ExecutablePermission::PosixNotExecutable
                },
                query,
            );
            let retained = lint_with(&shared, current.clone()).expect("a report");
            let fresh = lint_with(&permission_linter(Cache::None), current).expect("a report");
            assert_eq!(retained.findings, fresh.findings);
            assert_eq!(retained.findings.len(), usize::from(executable));
            assert!(retained.unknowns.is_empty());
            last = summary(&retained);
        }
        last
    }

    /// A permission that cannot be known is neither false nor a proven absence.
    pub(crate) fn permission_unavailable() -> Outcome {
        let root = tempfile::tempdir().expect("root");
        let query = "let files = select path as file from \"filesystem.file\"; let yes = select path as file from \"filesystem.executable\" where executable = \"true\"; let absent = antijoin(left=files,right=yes,left-on=[\"file\"],right-on=[\"file\"]); emit absent subject file key(file);";
        let run = |observation| {
            lint_with(
                &permission_linter(Cache::None),
                permission_request(root.path(), observation, query),
            )
            .expect("a report")
        };
        let unknown = run(ExecutablePermission::Unknown);
        assert!(unknown.findings.is_empty());
        assert!(!unknown.unknowns.is_empty());
        let known_false = run(ExecutablePermission::PosixNotExecutable);
        assert_eq!(known_false.findings.len(), 1);
        assert!(known_false.unknowns.is_empty());
        let known_true = run(ExecutablePermission::PosixExecutable);
        assert!(known_true.findings.is_empty());
        assert!(known_true.unknowns.is_empty());
        let unsupported = run(ExecutablePermission::Unsupported);
        assert_eq!(unsupported.findings.len(), 1);
        assert!(unsupported.unknowns.is_empty());
        summary(&unsupported)
    }

    /// A linter with no language, retaining into `cache`.
    fn permission_linter(cache: Cache) -> Linter {
        Linter::new(
            Languages::default(),
            WorkerPool::from(Workers(NonZeroUsize::MIN)),
            MemoryBudget::Machine,
            cache,
        )
    }

    /// A request over the one given file `case.py`, whose permission is `observation`.
    fn permission_request(
        root: &std::path::Path,
        observation: ExecutablePermission,
        query: &str,
    ) -> LintRequest {
        LintRequest {
            project_search_scope: Default::default(),
            given_project_inventory: Default::default(),
            source_language: None,
            root: root.to_path_buf(),
            rules: BTreeMap::from([(rule_id("permissions"), ordinary(query))]),
            files: Vec::new(),
            hosted: Vec::new(),
            corpus: Corpus::Given(vec![CaseSource {
                executable: observation,
                path: RelativePath("case.py".into()),
                text: SourceText("print(1)\n".into()),
            }]),
            beside: Vec::new(),
        }
    }

    /// Hosted text that collides in the readable wire form invalidates the retained cross-file findings when it changes.
    pub(crate) fn hosted_text_identity() -> Outcome {
        let root = case_directory();
        let store = tempfile::tempdir().expect("store");
        let source = root.path().join("case.txt");
        let modified = std::fs::metadata(&source)
            .expect("metadata")
            .modified()
            .expect("modified");
        let engine = |cache| provider_linter("same-build", "host-identity", &[], "unused", cache);
        let request = |first: &str, second: &str| {
            let text = |name: &str, kind| HostColumn {
                name: HostColumnName(name.into()),
                kind,
            };
            let mut request = listed(
                root.path(),
                BTreeMap::from([(
                    rule_id("host-check"),
                    ordinary(
                        "let found = select file from \"host.values\" where first = \"x\ttext:y\"; emit found subject file key(file);",
                    ),
                )]),
            );
            request.hosted = vec![HostRelation {
                name: HostRelationName("host.values".into()),
                columns: vec![
                    text("file", HostKind::File),
                    text("first", HostKind::Text),
                    text("second", HostKind::Text),
                ],
                rows: vec![HostRow(vec![
                    Cell::File(RelativePath("case.txt".into())),
                    Cell::Text(HostText(first.into())),
                    Cell::Text(HostText(second.into())),
                ])],
            }];
            request
        };
        let left = request("x\ttext:y", "z");
        let right = request("x", "y\ttext:z");
        let fresh_left = lint_with(&engine(Cache::None), left.clone()).expect("a report");
        let fresh_right = lint_with(&engine(Cache::None), right.clone()).expect("a report");
        assert_eq!(fresh_left.findings.len(), 1);
        assert!(fresh_right.findings.is_empty());
        let shared = engine(Cache::At(store.path().to_owned()));
        for expected in [
            (&left, &fresh_left),
            (&left, &fresh_left),
            (&right, &fresh_right),
            (&right, &fresh_right),
            (&left, &fresh_left),
        ] {
            assert_eq!(
                &lint_with(&shared, expected.0.clone()).expect("a report"),
                expected.1
            );
            assert_eq!(
                &lint_with(
                    &engine(Cache::At(store.path().to_owned())),
                    expected.0.clone()
                )
                .expect("a report"),
                expected.1,
                "fresh engine must also validate the persisted identity"
            );
        }
        assert_eq!(
            std::fs::metadata(source)
                .expect("metadata")
                .modified()
                .expect("modified"),
            modified
        );
        summary(&fresh_left)
    }

    /// A later rule that produces an observation publishes it nonempty although it has no finding, and the include and exclude of a
    /// producer govern its diagnostics, not the observation.
    pub(crate) fn zero_finding_producer() -> Outcome {
        let root = case_directory();
        let store = tempfile::tempdir().expect("cache");
        let engine = provider_linter(
            "observation-fixture",
            "observation-fixture",
            &[],
            "unused",
            Cache::At(store.path().to_owned()),
        );
        let producer = rule_id("z-producer");
        let consumer = rule_id("a-consumer");
        let mut consuming =
            ordinary("let found = select path from published; emit found subject path key(path);");
        consuming.observations.push(ObservationBinding {
            name: ObservationBindingName("published".into()),
            columns: vec![column("path", ObservationKind::File)],
            producers: vec![ObservationProducer::Output {
                rule: producer.clone(),
                output: ObservationName("used".into()),
            }],
        });
        let request = listed(
            root.path(),
            BTreeMap::from([
                (consumer.clone(), consuming),
                (
                    producer,
                    ordinary(
                        "let files = select path from \"filesystem.file\"; let empty = select f.path as path from files as f where f.path != f.path; observe files as used; emit empty subject path key(path);",
                    ),
                ),
            ]),
        );
        let mut last = (0, 0, true, 0);
        for _ in 0..2 {
            let answer = lint_with(&engine, request.clone())
                .expect("named producer executes before consumer");
            last = summary(&answer);
            assert_eq!(
                answer.findings.len(),
                1,
                "observation rows are not diagnostics and the producer Emit is genuinely empty"
            );
            assert_eq!(answer.findings[0].rule, consumer);
            assert!(
                answer.unknowns.is_empty(),
                "a real complete publication satisfies the binding"
            );
        }
        // Diagnostic governance is independent of explicitly bound facts. Exercise
        // both include and exclude while retaining cold/warm public entry behavior.
        for include_only in [false, true] {
            let mut configured = request.clone();
            let producing = configured
                .rules
                .get_mut(&rule_id("z-producer"))
                .expect("the producer");
            producing.query = RyoQlQuery::try_from(
            "let files = select path from \"filesystem.file\"; observe files as used; emit files subject path key(path);",
        )
        .expect("a query");
            let pattern = Glob::try_from(if include_only {
                "other.txt"
            } else {
                "case.txt"
            })
            .expect("a glob");
            if include_only {
                producing.include.push(pattern);
            } else {
                producing.exclude.push(pattern);
            }
            for _ in 0..2 {
                let answer = lint_with(&engine, configured.clone()).expect("a report");
                assert!(
                    answer.unknowns.is_empty(),
                    "diagnostic exclusion is not missing observation data"
                );
                assert_eq!(answer.findings.len(), 1);
                assert_eq!(
                    answer.findings[0].rule, consumer,
                    "excluded producer Emit stays suppressed"
                );
            }
        }
        let mut undeclared = request.clone();
        undeclared
            .rules
            .get_mut(&consumer)
            .expect("the consumer")
            .observations
            .clear();
        assert!(
            matches!(lint_with(&engine, undeclared), Err(LintError::Rule { .. })),
            "removing the declaration must reject the source, not reuse a previously bound plan"
        );
        last
    }

    /// A publication re-exported by a second rule survives a cold run, a warm run, a replay of the consumer and a fresh engine.
    pub(crate) fn re_exported_publication() -> Outcome {
        let root = case_directory();
        let store = tempfile::tempdir().expect("store");
        let engine = |cache| {
            provider_linter(
                "cross-observation-v1",
                "cross-observation-v1",
                &[],
                "unused",
                cache,
            )
        };
        let p = rule_id("z-producer");
        let q = rule_id("m-reexport");
        let c = rule_id("a-consumer");
        let columns = vec![
            column("count", ObservationKind::Number),
            column("label", ObservationKind::Symbol),
        ];
        let request = |marker: &str| {
            let producer = ordinary(
                r#"let files = select path from "filesystem.file";
let empty = select f.path as path from files as f where f.path != f.path;
let counted = group-count(from=files, keys=[], count="count");
let published = extend(from=counted, values={label: "escaped λ	\tvalue"});
observe published as shared;
emit empty subject path key(path);"#,
            );
            let mut reexport = ordinary(
                "let copied = select count, label from incoming; let empty = select l.count as count from copied as l where l.count != l.count; observe copied as shared; emit empty subject count key(count);",
            );
            reexport.observations = vec![incoming(columns.clone(), p.clone())];
            let mut consumer = ordinary(
                "param marker: string; let found = select count, label from incoming where label != $marker; emit found subject label key(count);",
            );
            consumer.observations = vec![incoming(columns.clone(), q.clone())];
            consumer.parameters.insert(
                ParameterName("marker".into()),
                ParameterValue::Text(ParameterText(marker.into())),
            );
            listed(
                root.path(),
                BTreeMap::from([
                    (c.clone(), consumer),
                    (q.clone(), reexport),
                    (p.clone(), producer),
                ]),
            )
        };
        let cached = engine(Cache::At(store.path().to_owned()));
        // RyoQL strings preserve their bytes (winnow::string): a literal tab is
        // authored directly, while backslash-t remains a distinct pair of bytes.
        let label = "escaped λ\t\\tvalue";
        let mut last = (0, 0, true, 0);
        for marker in ["never-a", "never-a", label, label, "never-b", "never-a"] {
            let requested = request(marker);
            let fresh = lint_with(&engine(Cache::None), requested.clone()).expect("a report");
            let expected = usize::from(marker != label);
            assert_eq!(fresh.findings.len(), expected);
            if expected != 0 {
                assert_eq!(fresh.findings[0].rule, c);
                assert_eq!(
                    fresh.findings[0].subject,
                    Slot::Is(Cell::Text(HostText(label.into())))
                );
            }
            assert!(fresh.unknowns.is_empty());
            last = summary(&fresh);
            assert_eq!(
                lint_with(&cached, requested.clone()).expect("a report"),
                fresh
            );
            assert_eq!(
                lint_with(&engine(Cache::At(store.path().to_owned())), requested)
                    .expect("a report"),
                fresh
            );
        }
        last
    }

    /// A producer whose population is refused and one whose rows are truly empty stay distinct on a cold and a retained replay.
    pub(crate) fn refused_and_empty_producers() -> Outcome {
        let root = case_directory();
        let mut last = (0, 0, true, 0);
        for known in [true, false] {
            let store = tempfile::tempdir().expect("store");
            let capabilities: &'static [OptionalCapability] = if known {
                &[OptionalCapability::Identifier]
            } else {
                &[]
            };
            let engine = provider_linter(
                "readiness-fixture",
                "readiness-fixture",
                capabilities,
                "present",
                Cache::At(store.path().to_owned()),
            );
            let p = rule_id("producer");
            let c = rule_id("consumer");
            let mut consumer = ordinary(
                "let found = select id, file, span from incoming; emit found subject file key(id);",
            );
            consumer.observations = vec![incoming(
                vec![
                    column("id", ObservationKind::Node),
                    column("file", ObservationKind::File),
                    column("span", ObservationKind::Span),
                ],
                p.clone(),
            )];
            let request = listed(
                root.path(),
                BTreeMap::from([
                    (
                        p,
                        ordinary(
                            "let files = select path from \"filesystem.file\"; let empty = select f.path as path from files as f where f.path != f.path; let rows = select id, file, span from \"identifier\" where name = \"absent\"; observe rows as shared; emit empty subject path key(path);",
                        ),
                    ),
                    (c.clone(), consumer),
                ]),
            );
            for _ in 0..2 {
                let answer = lint_with(&engine, request.clone()).expect("a report");
                assert!(answer.findings.is_empty());
                last = summary(&answer);
                assert_eq!(
                    answer.unknowns.iter().any(|unknown| unknown.rule == c),
                    !known,
                    "filtered Unknown population cannot become a complete empty observation on warm replay"
                );
            }
        }
        last
    }

    /// A selected producer that has no query never becomes a complete empty input of its consumer.
    pub(crate) fn producer_without_query() -> Outcome {
        let root = case_directory();
        let store = tempfile::tempdir().expect("store");
        let engine = provider_linter(
            "no-query-fixture",
            "no-query-fixture",
            &[],
            "unused",
            Cache::At(store.path().to_owned()),
        );
        let consumer = rule_id("consumer");
        let mut rule =
            ordinary("let found = select path from incoming; emit found subject path key(path);");
        rule.observations = vec![ObservationBinding {
            name: ObservationBindingName("incoming".into()),
            columns: vec![column("path", ObservationKind::File)],
            producers: vec![ObservationProducer::NoQuery {
                rule: rule_id("selected-unimplemented"),
            }],
        }];
        let request = listed(root.path(), BTreeMap::from([(consumer.clone(), rule)]));
        let mut last = (0, 0, true, 0);
        for _ in 0..2 {
            let answer = lint_with(&engine, request.clone()).expect("a report");
            assert!(answer.findings.is_empty());
            assert!(
                answer
                    .unknowns
                    .iter()
                    .any(|unknown| unknown.rule == consumer)
            );
            last = summary(&answer);
        }
        last
    }

    /// An export nothing binds demands no reader and changes no ordinary result.
    pub(crate) fn unbound_export() -> Outcome {
        let root = case_directory();
        // This fixture reports Missing::Any if Identifier is demanded. An unused
        // export must be pruned before request wants, not merely ignored at rendering.
        let store = tempfile::tempdir().expect("store");
        let engine = provider_linter(
            "ordinary-unbound-fixture",
            "ordinary-unbound-fixture",
            &[],
            "unused",
            Cache::At(store.path().to_owned()),
        );
        let request = |query: &str| {
            listed(
                root.path(),
                BTreeMap::from([(rule_id("ordinary"), ordinary(query))]),
            )
        };
        let plain = request(
            "let found = select path from \"filesystem.file\"; emit found subject path key(path);",
        );
        let unbound = request(
            "let found = select path from \"filesystem.file\"; let unavailable = select id from \"identifier\"; observe unavailable as unused; emit found subject path key(path);",
        );
        let expected = lint_with(&engine, plain).expect("a report");
        assert_eq!(expected.findings.len(), 1);
        assert!(expected.unknowns.is_empty());
        for _ in 0..2 {
            assert_eq!(
                lint_with(&engine, unbound.clone()).expect("a report"),
                expected
            );
        }
        summary(&expected)
    }
}
