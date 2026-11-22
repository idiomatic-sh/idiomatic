//! Where each step of a compiled plan was written, from the compile inbound API through the real parser: a statement of the
//! rule's own query is the query's under its `let` name; a module's statement is the module's under its own `let` name (the
//! one it returns too, which the caller names otherwise), reached through the caller's import and call; a step a rewrite
//! makes from another is the other's statement.

use super::super::compiler_over;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::compile::interface::CompileUseCase as _;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::program::service::{document_source, query_bundle, source_id};
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_requests::types::QueryRequest;
use ryo_values::domain::query_tokens::types::{QueryBundle, QuerySpelling};
use ryo_values::domain::written_sites::types::StepWritten;
use std::borrow::Cow;

/// A rule's query and the one module it imports, as their texts.
type Input = (&'static str, &'static str);
/// Each step of the plan, in order: its document, `:`, its statement, then `<` and each document that reaches it; `-` for a
/// step with no record. Steps are separated by ` | `.
type Value = Cow<'static, str>;
type Failure = String;

/// A rule that filters the syntax tree through a module.
const ROOT: &str = "import filter from \"filter\";\nlet nodes = select id, kind from \"syntax-tree\";\nlet result = filter(nodes=nodes);\nemit result subject id key(id);";
/// The module: a filter, then the step it returns.
const MODULE: &str = "input nodes: relation<id: node, kind: symbol>;\noutput relation<id: node, kind: symbol>;\nlet selected = select id, kind from nodes where kind = \"call\";\nlet kept = select id, kind from selected where id != id;\nreturn kept;";
/// A rule of its own statements only, whose filter of a join is pushed below the join by the compiler.
const JOINED: &str = "let nodes = select id, kind, file from \"syntax-tree\";\nlet pairs = select l.id as id, l.kind as kind, r.kind as other from nodes as l join nodes as r on l.id = r.id and l.file = r.file;\nlet calls = select id, kind from pairs where kind = \"call\";\nemit calls subject id key(id);";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_modules_statements_are_the_modules_under_their_own_names_reached_through_the_rule",
        ),
        word: Word::Succeeds,
        input: || (ROOT, MODULE),
        // The module's two filters and the rule's read are fused into one step, the outermost statement's: the one the module
        // returns, which the rule calls `result`, under the module's own name, reached by the rule's import and its call.
        expect: Expect::Value(Cow::Borrowed("filter:kept<root<root")),
    },
    Case {
        name: CaseName("a_step_a_rewrite_makes_is_the_statement_it_was_made_from"),
        word: Word::Succeeds,
        input: || (JOINED, ""),
        // The filter `calls` is pushed below the join `pairs`: the filter made there is `calls`' too.
        expect: Expect::Value(Cow::Borrowed("root:calls | root:pairs | root:calls")),
    },
];

fn subject((root, module): Input) -> Result<Value, Failure> {
    let id = |name| source_id(QuerySpelling(name)).map_err(|error| format!("{error:?}"));
    let modules = if module.is_empty() {
        Vec::new()
    } else {
        vec![document_source(id("filter")?, module)]
    };
    let bundle: QueryBundle = query_bundle(document_source(id("root")?, root), modules)
        .map_err(|error| format!("{error:?}"))?;
    let plan = compiler_over(vec![Relation::SyntaxTree])
        .compile(QueryRequest {
            observations: Vec::new(),
            text: bundle,
            hosted: Vec::new(),
            modules: ParsedModules::default(),
        })
        .map_err(|error| error.to_string())?
        .plan;
    let steps: Vec<String> = plan
        .steps
        .iter()
        .map(|step| match &step.written {
            StepWritten::Written(site) => {
                let mut shown = [&*site.document.0, ":", &site.statement].concat();
                for call in site.trace.iter().rev() {
                    shown.push('<');
                    shown.push_str(&call.source_id.0);
                }
                shown
            }
            StepWritten::Unrecorded => "-".to_owned(),
        })
        .collect();
    Ok(Cow::Owned(steps.join(" | ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
