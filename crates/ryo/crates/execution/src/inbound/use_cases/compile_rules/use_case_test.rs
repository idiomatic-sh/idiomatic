//! Compilation compiles each pattern and each module once per session, and refuses as a lone compile refuses.
use super::interface::{CompileRulesUseCase as _, CompiledRules, RuleCompilationError};
use super::types::CompileRequest;
use super::use_case::CompileRulesUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_operators::domain::patterns::types::PatternMatcher;
use ryo_operators::domain::patterns::types::prepared::value::PreparedMatchers;
use ryo_operators::domain::table_texts::types::TextView;
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
use std::borrow::Cow;
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
/// The program a row compiles.
#[derive(Clone, Copy, Debug)]
enum Program {
    /// `rules` rules, each with its own literal pattern, and then one rule of an oversized pattern.
    Patterns { rules: usize },
    /// `rules` rules over one catalog of `modules` modules, then the same with one more module that does not parse.
    Catalog { rules: usize, modules: usize },
    /// A rule whose plan selects syntax nodes of the kind `nonesuch`, with no linked language.
    UnknownKind,
    /// The rule at `index` of a one-rule request.
    PastTheRules { index: usize },
}

/// What compiling the program shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Compiled {
    /// The matchers built after compiling, the matches that succeeded when every pattern was
    /// matched twice, and the matchers built then; and for the oversized pattern, whether its slot
    /// refuses it as an invalid regex and how many matchers it built.
    Matchers {
        prepared: usize,
        matched: usize,
        built: usize,
        oversized_refused: bool,
        oversized_built: usize,
    },
    /// The modules parsed and the plans made in one session; and with the broken module, the rule
    /// the refusal names and whether its error is the one a lone compile of that rule answers.
    Parsed {
        modules: usize,
        plans: usize,
        refused_rule: Cow<'static, str>,
        as_lone_compile: bool,
    },
}

/// A refusal, as a row states it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Refusal {
    UnknownKind {
        id: Cow<'static, str>,
        reason: Cow<'static, str>,
    },
    UnknownRuleIndex {
        index: usize,
        rules: usize,
    },
    /// Any other refusal, as it prints.
    Other(String),
}

type Input = Program;
type Value = Compiled;
type Failure = Refusal;

/// The rows of compiling a 413-rule program, as many rules as the Ruff program has, and of refused rules.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        // No matcher is built by compiling; each pattern's slot builds it once however often it is matched.
        name: CaseName("compiles_each_pattern_once"),
        word: Word::Succeeds,
        input: || Program::Patterns { rules: 413 },
        expect: Expect::Value(Compiled::Matchers {
            prepared: 0,
            matched: 826,
            built: 413,
            oversized_refused: true,
            oversized_built: 1,
        }),
    },
    Case {
        name: CaseName("each_module_is_parsed_once_per_session"),
        word: Word::Succeeds,
        input: || Program::Catalog {
            rules: 413,
            modules: 60,
        },
        expect: Expect::Value(Compiled::Parsed {
            modules: 60,
            plans: 413,
            refused_rule: Cow::Borrowed("r0"),
            as_lone_compile: true,
        }),
    },
    Case {
        name: CaseName("a_word_for_a_kind_no_linked_language_has_is_refused_before_any_file"),
        word: Word::Refuses,
        input: || Program::UnknownKind,
        expect: Expect::Error(Refusal::UnknownKind {
            id: Cow::Borrowed("kinds"),
            reason: Cow::Borrowed("kind `nonesuch` names no kind of "),
        }),
    },
    Case {
        // Never reported under an empty id.
        name: CaseName("an_index_past_the_rules_is_refused_with_the_count"),
        word: Word::Refuses,
        input: || Program::PastTheRules { index: 1 },
        expect: Expect::Error(Refusal::UnknownRuleIndex { index: 1, rules: 1 }),
    },
];

/// The refusal as a row states it.
fn refusal(error: RuleCompilationError) -> Refusal {
    use RuleCompilationError::{Observations, Query, Rules};
    match error {
        RuleCompilationError::UnknownKind { id, reason } => Refusal::UnknownKind {
            id: Cow::Owned(id.0.to_string()),
            reason: Cow::Owned(reason.0.to_string()),
        },
        RuleCompilationError::UnknownRuleIndex { index, rules } => {
            Refusal::UnknownRuleIndex { index, rules }
        }
        other @ (Observations(_) | Rules(_) | Query { .. }) => Refusal::Other(format!("{other:?}")),
    }
}

/// Compiles the program and observes it.
fn subject(program: Input) -> Result<Value, Failure> {
    match program {
        Program::Patterns { rules } => pattern_builds(rules),
        Program::Catalog { rules, modules } => session_parses(rules, modules),
        Program::UnknownKind => unknown_kind(),
        Program::PastTheRules { index } => past_the_rules(index),
    }
    .map_err(refusal)
}

/// A rule whose plan selects syntax nodes of the kind `nonesuch`, compiled with no linked language: refused by the rule's id.
fn unknown_kind() -> Result<Compiled, RuleCompilationError> {
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
    let refused = compiled(Arc::new(fake), &[empty_rule("kinds")]);
    refused.map(|compiled| Compiled::Parsed {
        modules: 0,
        plans: compiled.plans().len(),
        refused_rule: Cow::Borrowed(""),
        as_lone_compile: false,
    })
}

/// The rule at `index` of a one-rule request.
fn past_the_rules(index: usize) -> Result<Compiled, RuleCompilationError> {
    let rules = [empty_rule("only")];
    crate::application::rule_compilation::service::named(
        &rules,
        ryo_execution_domain::domain::kind_check::types::RulePlace(index),
    )
    .map(|found| Compiled::Parsed {
        modules: 0,
        plans: 0,
        refused_rule: Cow::Owned(found.id.0.to_string()),
        as_lone_compile: false,
    })
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

/// A rule `id` of an empty query.
fn empty_rule(id: &str) -> RuleSpec {
    rule(id.to_owned(), QueryBundle::from(QueryText("".into())))
}

/// The real query language over the winnow syntax.
fn winnow() -> Arc<dyn QueryLanguagePort> {
    compiler(Arc::new(
        ryo_query_language::outbound::syntax::factory::winnow(),
    ))
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
fn pattern_builds(count: usize) -> Result<Compiled, RuleCompilationError> {
    let matching = |pattern: &str| {
        let text = format!(
            r#"let rows = select * from "comment" where matches(text, "{pattern}"); emit rows subject file key(span);"#
        );
        rule(
            format!("r{pattern}"),
            QueryBundle::from(QueryText(text.into())),
        )
    };
    let patterns: Vec<String> = (0..count).map(|place| format!("p{place}x")).collect();
    let rules: Vec<RuleSpec> = patterns.iter().map(|pattern| matching(pattern)).collect();
    let plans = |rules: &[RuleSpec]| {
        compiled(winnow(), rules).map(|queries| PreparedMatchers::new(queries.plans()))
    };
    let slots = plans(&rules)?;
    let prepared = slots.builds.load(Ordering::Relaxed);
    let matcher =
        PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl);
    let mut matched = 0;
    for _file in 0..2 {
        for pattern in &patterns {
            matched += usize::from(
                slots
                    .regex(TextView(pattern), matcher)
                    .and_then(Result::ok)
                    .is_some(),
            );
        }
    }
    let oversized = "(?:[a-z]{1000}){1000}";
    let refused = plans(&[matching(oversized)])?;
    let oversized_refused = matches!(
        refused.regex(TextView(oversized), matcher),
        Some(Err(ryo_operators::domain::patterns::errors::PatternsError::InvalidRegex { .. }))
    );
    Ok(Compiled::Matchers {
        prepared,
        matched,
        built: slots.builds.load(Ordering::Relaxed),
        oversized_refused,
        oversized_built: refused.builds.load(Ordering::Relaxed),
    })
}

/// 413 rules over one catalog of 60 modules parse each module once in their session, not once per rule; the same program with one
/// more module that does not parse, imported by no rule, is refused at its first rule as a lone compile of that rule refuses it.
fn session_parses(count: usize, module_count: usize) -> Result<Compiled, RuleCompilationError> {
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
        (0..count)
            .map(|place| {
                let root = format!(
                    r#"import choose from "query/m{}"; let files = select path from "filesystem.file"; let result = choose(files=files); emit result subject "path" key("path");"#,
                    place % module_count
                );
                let id = source_id(QuerySpelling(&format!("rules/r{place}"))).expect("a root id");
                let query = query_bundle_with_catalog(document_source(id, root.into_boxed_str()), catalog.clone())
                    .expect("a bundle");
                rule(format!("r{place}"), query)
            })
            .collect()
    };
    let mut modules: Vec<(String, String)> = (0..module_count)
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
    let refused = compiled(winnow(), &broken).err();
    let lone = winnow().compile(&broken[0].query, &[], &[]).err();
    let (refused_rule, as_lone_compile) = match refused {
        Some(RuleCompilationError::Query { id, error }) => {
            (id, lone == Some(QueryLanguageError::Refused { error }))
        }
        Some(other) => return Err(other),
        None => (String::new(), false),
    };
    Ok(Compiled::Parsed {
        modules: parsed,
        plans: session.plans().len(),
        refused_rule: Cow::Owned(refused_rule),
        as_lone_compile,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
