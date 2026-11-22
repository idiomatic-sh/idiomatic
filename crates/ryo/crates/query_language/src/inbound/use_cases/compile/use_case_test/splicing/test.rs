//! An expansion of a fragment that binds no relation input is lowered once per use case and spliced into the plans after:
//! every plan is the plan a use case that compiled nothing before makes.

use super::super::compiler_over;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::compile::interface::CompileUseCase as _;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::program::service::{document_source, query_bundle, source_id};
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::query_requests::types::QueryRequest;
use ryo_values::domain::query_tokens::types::{QueryBundle, QuerySpelling};

type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_spliced_expansion_plans_what_its_own_lowering_plans"),
    word: Word::Succeeds,
    input: || a_spliced_expansion_plans_what_its_own_lowering_plans,
    expect: Expect::Value(()),
}];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// A fragment with a parameter, comparisons and a nested fragment, none of them binding a relation input.
const KINDS: &str = "import named from \"query/named\";\nparam wanted: string;\noutput relation<id: node, file: file>;\nlet all = named();\nlet kept = select id, file from all where kind = $wanted;\nlet unused = select id from all where matches(text, \"^b\");\nreturn kept;";
/// The nested fragment.
const NAMED: &str = "output relation<id: node, kind: symbol, file: file, text: symbol>;\nlet rows = select id, kind, file, text from \"syntax-tree\" where matches(text, \"^a\");\nreturn rows;";

/// The rules: the first calls the fragment once after steps of its own; the second twice, under other names, with steps
/// of its own between; the third with another value.
const RULES: [&str; 3] = [
    "import kinds from \"query/kinds\";\nlet own = select id, file from \"syntax-tree\" where kind = \"x\";\nlet more = select id from own;\nlet found = kinds(wanted=\"call\");\nemit found subject id key(id);",
    "import kinds from \"query/kinds\";\nlet first = kinds(wanted=\"call\");\nlet between = select id, file from first where id = id;\nlet second = kinds(wanted=\"call\");\nlet both = union(left=between, right=second);\nemit both subject id key(id);",
    "import kinds from \"query/kinds\";\nlet other = kinds(wanted=\"name\");\nlet again = kinds(wanted=\"call\");\nlet both = union(left=other, right=again);\nemit both subject id key(id);",
];

/// A rule's text over the two fragments.
fn bundle(rule: &str) -> QueryBundle {
    let source = |name: &str, text: &str| {
        document_source(source_id(QuerySpelling(name)).expect("a source id"), text)
    };
    query_bundle(
        source("root", rule),
        [source("query/kinds", KINDS), source("query/named", NAMED)],
    )
    .expect("the bundle reads")
}

/// The plans of `rules`, compiled in order by one use case over the modules the first compile parsed.
fn compiled_in_turn(rules: &[&str]) -> Vec<Plan> {
    let use_case = compiler_over(vec![Relation::SyntaxTree]);
    let mut modules = ParsedModules::default();
    rules
        .iter()
        .map(|rule| {
            let report = use_case
                .compile(QueryRequest {
                    observations: Vec::new(),
                    text: bundle(rule),
                    hosted: Vec::new(),
                    modules: modules.clone(),
                })
                .expect("the rule compiles");
            modules = report.modules;
            report.plan
        })
        .collect()
}

fn a_spliced_expansion_plans_what_its_own_lowering_plans() {
    let shared = compiled_in_turn(&RULES);
    let alone: Vec<Plan> = RULES
        .iter()
        .map(|rule| compiled_in_turn(&[rule]).remove(0))
        .collect();
    assert_eq!(shared.len(), RULES.len());
    for (shared, alone) in shared.iter().zip(&alone) {
        assert!(!alone.steps.is_empty());
        assert!(!alone.comparison_origins.is_empty());
        // Equality leaves out a comparison's origin, which its debug text holds.
        assert_eq!(format!("{shared:?}"), format!("{alone:?}"));
    }
    // The rules compiled in the other order make the same plans.
    let reversed = compiled_in_turn(&[RULES[2], RULES[1], RULES[0]]);
    for (reversed, alone) in reversed.iter().zip(alone.iter().rev()) {
        assert_eq!(format!("{reversed:?}"), format!("{alone:?}"));
    }
}
