//! Public preparation accepts agreeing requests and rejects structurally mismatched ones.
use super::{
    interface::PrepareUseCase,
    types::{PlanSource, PrepareRequest},
    use_case::PrepareUseCaseImpl,
};
use ryo_execution_runtime::application::file_dag::program::prepared_execution::interface::PreparedExecution;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::rule_intent::errors::RuleIntentError;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use super::interface::{ExecutionCompilation as _, PreparedQueries as _, QueryCompilation, RuleCompilationError};
use ryo_execution_domain::domain::matching::types::PatternMatcher;
use ryo_execution_domain::domain::matching::types::prepared::value::PreparedMatchers;
use ryo_execution_domain::domain::table_texts::types::TextView;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl;
use ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl;
use ryo_query_language::outbound::syntax::errors::SyntaxError;
use ryo_query_language::outbound::syntax::interface::SyntaxPort;
use ryo_values::domain::descriptions::types::Grammar;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types::{ParsedQueryBundle, WrittenProgram};
use ryo_values::domain::query_tokens::types::{QueryBundle, QuerySpelling, QueryView};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

type Input = PrepareRequest;
type Value = usize;
type Failure = OperatingError;

fn request(bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>) -> Input {
    PrepareRequest {
        plans: PlanSource::Authored {
            plans: vec![],
            bindings,
        },
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    }
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(an_empty_request_prepares_no_rule)),
        word: Word::Succeeds,
        input: || request(vec![]),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName(stringify!(a_binding_population_without_a_rule_is_refused)),
        word: Word::Refuses,
        input: || request(vec![vec![]]),
        expect: Expect::Error(OperatingError::ObservationConfiguration(
            RuleIntentError::InvalidReference,
        )),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let prepare = PrepareUseCaseImpl::new(
        Arc::new(
            ryo_execution_runtime::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            ))),
        ),
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    prepare.prepare(input).map(|program| program.rule_count().0)
}
/// The cost rows of compiling a 413-rule program, as many rules as the Ruff program has: no input, and two counts each.
const COMPILES: &[Case<fn() -> Result<[usize; 2], RuleCompilationError>, [usize; 2], RuleCompilationError>] = &[
    Case {
        // The matchers built after compiling and preparing, then after every pattern was matched twice.
        name: CaseName("compiles_each_pattern_once"),
        word: Word::Succeeds,
        input: || pattern_builds,
        expect: Expect::Value([0, 413]),
    },
    Case {
        // The modules parsed, and the plans made, over one catalog of 60 modules.
        name: CaseName("each_module_is_parsed_once_per_session"),
        word: Word::Succeeds,
        input: || session_parses,
        expect: Expect::Value([60, 413]),
    },
];

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
        let _before = self.modules.fetch_add(bundle.modules.len(), Ordering::SeqCst);
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
        Arc::new(CompileUseCaseImpl::new(syntax, catalog())),
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
        rule(format!("r{pattern}"), QueryBundle::from(QueryText(text.into())))
    };
    let patterns: Vec<String> = (0..413).map(|place| format!("p{place}x")).collect();
    let rules: Vec<RuleSpec> = patterns.iter().map(|pattern| matching(pattern)).collect();
    let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
    let plans = |rules: &[RuleSpec]| {
        QueryCompilation::new(compiler(Arc::new(ryo_query_language::outbound::syntax::factory::winnow())))
            .compile(rules, &[], &digest)
            .map(|queries| PreparedMatchers::new(queries.plans()))
    };
    let slots = plans(&rules)?;
    let prepared = slots.builds.load(Ordering::Relaxed);
    let matcher = PatternMatcher(&ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl);
    for _file in 0..2 {
        for pattern in &patterns {
            assert!(matches!(slots.regex(TextView(pattern), matcher), Some(Ok(_))));
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
    use ryo_values::domain::program::service::{document_source, module_catalog, query_bundle_with_catalog, source_id};
    let over = |modules: Vec<(String, String)>| -> Vec<RuleSpec> {
        let catalog = module_catalog(modules.into_iter().map(|(id, text)| {
            document_source(source_id(QuerySpelling(&id)).expect("a module id"), text.into_boxed_str())
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
    let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
    let compiled = QueryCompilation::new(compiler(Arc::<CountingSyntax>::clone(&syntax)))
        .compile(&over(modules.clone()), &[], &digest)?;
    let parsed = syntax.modules.load(Ordering::SeqCst);
    modules.push(("query/unused".to_owned(), "not a module".to_owned()));
    let broken = over(modules);
    let winnow = || compiler(Arc::new(ryo_query_language::outbound::syntax::factory::winnow()));
    let refused = QueryCompilation::new(winnow())
        .compile(&broken, &[], &digest)
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
    Ok([parsed, compiled.plans().len()])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(COMPILES, Word::Succeeds, |count| count());
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
