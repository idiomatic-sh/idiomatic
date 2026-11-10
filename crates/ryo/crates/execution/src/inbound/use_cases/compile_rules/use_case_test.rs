//! Compilation compiles each pattern and each module once per session, and refuses as a lone compile refuses.
use super::interface::{CompileRulesUseCase as _, CompiledRules, RuleCompilationError};
use super::types::CompileRequest;
use super::use_case::CompileRulesUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution_domain::domain::matching::types::PatternMatcher;
use ryo_execution_domain::domain::matching::types::prepared::value::PreparedMatchers;
use ryo_execution_domain::domain::table_texts::types::TextView;
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl as QueryCompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_query_language::outbound::syntax::errors::SyntaxError;
use ryo_query_language::outbound::syntax::interface::SyntaxPort;
use ryo_values::domain::descriptions::types::{Grammar, KindVocabularies};
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types::{ParsedQueryBundle, WrittenProgram};
use ryo_values::domain::query_tokens::types::{QueryBundle, QuerySpelling, QueryView};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The rules compiled through the use case over `compiler`, with no hosted relation and no linked language.
fn compiled(
    compiler: Arc<dyn QueryLanguagePort>,
    rules: &[RuleSpec],
) -> Result<CompiledRules, RuleCompilationError> {
    CompileRulesUseCaseImpl::new(
        compiler,
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    )
    .compile_rules(CompileRequest {
        rules,
        hosted: &[],
        vocabularies: &KindVocabularies(Vec::new()),
    })
}
/// What a row of the cases asks of compilation.
#[derive(Clone, Copy)]
enum Scenario {
    /// 413 rules, each with its own pattern: the matchers built after compiling, then after every pattern was matched twice.
    PatternBuilds,
    /// 413 rules over one catalog of 60 modules: the modules parsed and the plans made.
    SessionParses,
    /// A rule that writes a word for a kind no linked language has: whether it is refused by its id, and the plans made.
    UnknownKind,
    /// An index the compiler reports past the request's rules: the index and the rule count the refusal carries.
    UnknownIndex,
}
/// What a row of the cases asks.
type Input = Scenario;
/// Two counts each row answers.
type Value = [usize; 2];
type Failure = RuleCompilationError;

/// The rows of compiling a 413-rule program, as many rules as the Ruff program has, and of a refused kind word.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        // The matchers built after compiling and preparing, then after every pattern was matched twice.
        name: CaseName("compiles_each_pattern_once"),
        word: Word::Succeeds,
        input: || Scenario::PatternBuilds,
        expect: Expect::Value([0, 413]),
    },
    Case {
        // The modules parsed, and the plans made, over one catalog of 60 modules.
        name: CaseName("each_module_is_parsed_once_per_session"),
        word: Word::Succeeds,
        input: || Scenario::SessionParses,
        expect: Expect::Value([60, 413]),
    },
    Case {
        // Refused by the rule's id with the word, and no plan handed on.
        name: CaseName("a_word_for_a_kind_no_linked_language_has_is_refused_before_any_file"),
        word: Word::Succeeds,
        input: || Scenario::UnknownKind,
        expect: Expect::Value([1, 0]),
    },
    Case {
        // Refused with the index and the rule count, never reported under an empty id.
        name: CaseName("an_index_past_the_rules_is_refused_with_the_count"),
        word: Word::Succeeds,
        input: || Scenario::UnknownIndex,
        expect: Expect::Value([1, 1]),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::PatternBuilds => pattern_builds(),
        Scenario::SessionParses => session_parses(),
        Scenario::UnknownKind => unknown_kind(),
        Scenario::UnknownIndex => unknown_index(),
    }
}

/// A rule whose plan selects syntax nodes of the kind `nonesuch`, compiled with no linked language: refused by the rule's id.
fn unknown_kind() -> Result<Value, Failure> {
    use crate::outbound::query_language::fixtures::fake::QueryLanguageFake;
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::plan_parts::types::{
        ColumnIndex, ComparisonOrigin, Input as PlanInput, LiteralText, Operand, PlanComparison,
        QueryLiteral, StepScope, Test,
    };
    use ryo_values::domain::plans::types::{Operation, PlanStep};
    let mut fake = QueryLanguageFake::default();
    fake.plan.steps = vec![PlanStep {
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::SyntaxTree),
            comparisons: vec![PlanComparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(1)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("nonesuch".into()))),
                origin: ComparisonOrigin::Unrecorded,
            }],
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    }];
    let refused = compiled(
        Arc::new(fake),
        &[rule(
            "kinds".to_owned(),
            QueryBundle::from(QueryText("".into())),
        )],
    );
    Ok(match refused {
        Err(RuleCompilationError::UnknownKind { ref id, ref reason })
            if id.0.as_ref() == "kinds" && reason.0.contains("nonesuch") =>
        {
            [1, 0]
        }
        Err(other) => return Err(other),
        Ok(compiled) => [0, compiled.plans().len()],
    })
}

/// The rule at index 1 of a one-rule request: refused with the index and the count.
fn unknown_index() -> Result<Value, Failure> {
    let rules = [rule(
        "only".to_owned(),
        QueryBundle::from(QueryText("".into())),
    )];
    match crate::application::rule_compilation::service::named(
        &rules,
        ryo_execution_domain::domain::kind_check::types::RulePlace(1),
    ) {
        Err(RuleCompilationError::UnknownRuleIndex { index, rules }) => Ok([index, rules]),
        Err(other) => Err(other),
        Ok(found) => Ok([0, usize::from(found.id.0.is_empty())]),
    }
}

/// The real winnow syntax, counting the modules each read of a whole bundle parses.
struct CountingSyntax {
    /// The syntax it counts for.
    inner: Arc<dyn SyntaxPort>,
    /// How many modules were parsed.
    modules: AtomicUsize,
}
impl SyntaxPort for CountingSyntax {
    fn read(&self, text: QueryView<'_>) -> Result<WrittenProgram, SyntaxError> {
        self.inner.read(text)
    }
    fn read_bundle(&self, bundle: &QueryBundle) -> Result<ParsedQueryBundle, SyntaxError> {
        let _before = self
            .modules
            .fetch_add(bundle.modules.len(), Ordering::SeqCst);
        self.inner.read_bundle(bundle)
    }
    fn read_root(&self, bundle: &QueryBundle) -> Result<WrittenProgram, SyntaxError> {
        self.inner.read_root(bundle)
    }
    fn grammar(&self) -> Grammar {
        self.inner.grammar()
    }
}

/// The real query language over `syntax`.
fn compiler(syntax: Arc<dyn SyntaxPort>) -> Arc<dyn QueryLanguagePort> {
    let catalog = || Arc::new(ryo_query_language::outbound::catalog::factory::language());
    Arc::new(crate::outbound::query_language::factory::use_case(
        Arc::new(QueryCompileUseCaseImpl::new(syntax, catalog())),
        Arc::new(TypecheckUseCaseImpl::new(
            Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
            catalog(),
        )),
    ))
}

/// A rule `id` of the query `query`.
fn rule(id: String, query: QueryBundle) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query,
        applicability: RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("".into()),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

/// Lowering parses each literal pattern and builds no matcher; the prepared slot builds each once however often it is matched. A
/// pattern that parses but whose matcher is over the size limit is accepted by the compile and refused by its slot, the one place
/// it is compiled.
fn pattern_builds() -> Result<[usize; 2], RuleCompilationError> {
    let matching = |pattern: &str| {
        let text = format!(
            r#"let rows = select * from "comment" where matches(text, "{pattern}"); emit rows subject file key(span);"#
        );
        rule(
            format!("r{pattern}"),
            QueryBundle::from(QueryText(text.into())),
        )
    };
    let patterns: Vec<String> = (0..413).map(|place| format!("p{place}x")).collect();
    let rules: Vec<RuleSpec> = patterns.iter().map(|pattern| matching(pattern)).collect();
    let plans = |rules: &[RuleSpec]| {
        compiled(
            compiler(Arc::new(
                ryo_query_language::outbound::syntax::factory::winnow(),
            )),
            rules,
        )
        .map(|queries| PreparedMatchers::new(queries.plans()))
    };
    let slots = plans(&rules)?;
    let prepared = slots.builds.load(Ordering::Relaxed);
    let matcher =
        PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl);
    for _file in 0..2 {
        for pattern in &patterns {
            assert!(matches!(
                slots.regex(TextView(pattern), matcher),
                Some(Ok(_))
            ));
        }
    }
    let oversized = "(?:[a-z]{1000}){1000}";
    let refused = plans(&[matching(oversized)])?;
    assert!(matches!(
        refused.regex(TextView(oversized), matcher),
        Some(Err(OperatingError::InvalidRegex { .. }))
    ));
    assert_eq!(refused.builds.load(Ordering::Relaxed), 1);
    Ok([prepared, slots.builds.load(Ordering::Relaxed)])
}

/// 413 rules over one catalog of 60 modules parse each module once in their session, not once per rule; the same program with one
/// more module that does not parse, imported by no rule, is refused at its first rule as a lone compile of that rule refuses it.
fn session_parses() -> Result<[usize; 2], RuleCompilationError> {
    use ryo_values::domain::program::service::{
        document_source, module_catalog, query_bundle_with_catalog, source_id,
    };
    let over = |modules: Vec<(String, String)>| -> Vec<RuleSpec> {
        let catalog = module_catalog(modules.into_iter().map(|(id, text)| {
            document_source(
                source_id(QuerySpelling(&id)).expect("a module id"),
                text.into_boxed_str(),
            )
        }))
        .expect("a catalog");
        (0..413)
            .map(|place| {
                let root = format!(
                    r#"import choose from "query/m{}"; let files = select path from "filesystem.file"; let result = choose(files=files); emit result subject "path" key("path");"#,
                    place % 60
                );
                let id = source_id(QuerySpelling(&format!("rules/r{place}"))).expect("a root id");
                let query = query_bundle_with_catalog(document_source(id, root.into_boxed_str()), catalog.clone())
                    .expect("a bundle");
                rule(format!("r{place}"), query)
            })
            .collect()
    };
    let mut modules: Vec<(String, String)> = (0..60)
        .map(|place| {
            (
                format!("query/m{place}"),
                format!(
                    "input files: relation<path: file>;\noutput relation<path: file>;\nlet selected = select path from files where starts_with(path, \"m{place}\");\nreturn selected;"
                ),
            )
        })
        .collect();
    let syntax = Arc::new(CountingSyntax {
        inner: Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        modules: AtomicUsize::new(0),
    });
    let session = compiled(
        compiler(Arc::<CountingSyntax>::clone(&syntax)),
        &over(modules.clone()),
    )?;
    let parsed = syntax.modules.load(Ordering::SeqCst);
    modules.push(("query/unused".to_owned(), "not a module".to_owned()));
    let broken = over(modules);
    let winnow = || {
        compiler(Arc::new(
            ryo_query_language::outbound::syntax::factory::winnow(),
        ))
    };
    let refused = compiled(winnow(), &broken)
        .map(|_compiled| ())
        .expect_err("a module that does not parse refuses the run");
    let lone = winnow()
        .compile(&broken[0].query, &[], &[])
        .expect_err("a lone compile refuses the rule");
    assert!(matches!(
        refused,
        RuleCompilationError::Query { ref id, ref error }
            if id == "r0" && QueryLanguageError::Refused { error: error.clone() } == lone
    ));
    Ok([parsed, session.plans().len()])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
