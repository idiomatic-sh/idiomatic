//! Which closures of the Ruff cohort the compile inbound API fuses into a join that walks only the sources it binds: only
//! the one whose edges are a stored relation through pure selects and projections; edges a join or a union makes stay a
//! closure walked from every source. Each program copies one cohort closure verbatim: its edges, the closure and the one
//! step that reads it, from `languages/python/benchmarks/linters/ruff`. Upstream steps the shape does not depend on are
//! cut to one select.

use super::super::compiler_over_the_language;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::compile::errors::CompileError;
use crate::inbound::use_cases::compile::interface::CompileUseCase as _;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::plans::types::{Operation, QueryRequest};
use ryo_values::domain::query_tokens::types::QueryBundle;

/// `fragments/noqa-line-map.ryoql:2-13`: the closure is the left input of the join that reads it, keyed on its destination.
const NOQA_LINE_MAP_MAPPING_PATHS: &str = r#"
let quoted = select id from "syntax-tree" where kind = "f-string-part" or kind = "t-string-part" or kind = "string" and matches(text, "(?i)^[rub]*(?:\x22\x22\x22|'\x27')");
let continued = select id from "syntax-tree" where kind = "whitespace" and matches(text, "\\(?:\r\n|\r|\n)");
let mapped_tokens = union(left=quoted, right=continued);
let mapping_ranges = select l.id as id, r.file as file, r."start-line" as line, r."end-line" as endline from mapped_tokens as l join "source-location" as r on l.id = r.id where r."start-line" < r."end-line";
let mapping_edges = select l.id as origin, r.id as destination from mapping_ranges as l join mapping_ranges as r on l.file = r.file where l.line <= r.endline and r.line <= l.endline and l.endline < r.endline;
let mapping_paths = closure(edges=mapping_edges, source="origin", destination="destination");
let reachable_ranges = select l.origin as id, r.endline as endline from mapping_paths as l join mapping_ranges as r on l.destination = r.id;
let initial_ranges = select id, endline from mapping_ranges;
let all_ranges = union(left=initial_ranges, right=reachable_ranges);
let earlier_endings = select l.id as id, l.endline as endline from all_ranges as l join all_ranges as r on l.id = r.id where l.endline < r.endline;
let last_ranges = antijoin(left=all_ranges, right=earlier_endings, left-on=["id", "endline"], right-on=["id", "endline"]);
let mappings = select l.id as id, l.file as file, l.line as line, r.endline as endline from mapping_ranges as l join last_ranges as r on l.id = r.id;
emit mappings subject id key(line, endline);
"#;

/// `rules/F401/query.ryoql:6,20-25`, with `export_values` cut to one select.
const F401_LEFT_PATHS: &str = r#"
let nodes = select id, kind, file, span from "semantic-syntax";
let export_values = select parent as statement, child as value from "semantic-child" where field = "value";
let binary_nodes = select id from nodes where kind = "bin-op";
let left_fields = select parent, child from "semantic-child" where field = "left";
let binary_left = select l.id as origin, r.child as destination from binary_nodes as l join left_fields as r on l.id = r.parent;
let left_paths = closure(edges=binary_left,source="origin",destination="destination");
let binary_values = select l.statement as statement, l.value as value from export_values as l join binary_nodes as r on l.value = r.id;
let left_values = select l.statement as statement, r.destination as value from binary_values as l join left_paths as r on l.value = r.origin;
emit left_values subject statement key(value);
"#;

/// `rules/F401/query.ryoql:6,18,33-36`, with `extraction_roots` cut to one select.
const F401_NAMED_PATHS: &str = r#"
let nodes = select id, kind, file, span from "semantic-syntax";
let value_fields = select parent, child from "semantic-child" where field = "value";
let extraction_roots = select parent as statement, child as value from "semantic-child" where field = "targets";
let named_nodes = select id from nodes where kind = "named-expr";
let named_edges = select l.id as origin, r.child as destination from named_nodes as l join value_fields as r on l.id = r.parent;
let named_paths = closure(edges=named_edges,source="origin",destination="destination");
let unwrapped_values = select l.statement as statement, r.destination as value from extraction_roots as l join named_paths as r on l.value = r.origin;
emit unwrapped_values subject statement key(value);
"#;

/// `rules/F401/query.ryoql:75-76,97-107`.
const F401_INHERITED: &str = r#"
let events = select occurrence, name, scope, phase, kind, ordinal, file from "binding-event";
let bindings = select occurrence as binding, name, scope, phase, ordinal from events where kind = "bind";
let shadow_replacements = select previous as earlier, binding as later from "binding-shadow" where relationship = "replacement";
let timed_replacements = select l.earlier as earlier, l.later as later, r.phase as phase, r.ordinal as ordinal from shadow_replacements as l join bindings as r on l.later = r.binding;
let local_replacements = select l.earlier as earlier, l.later as later, l.ordinal as ordinal from timed_replacements as l join bindings as r on l.earlier = r.binding and l.phase = r.phase where r.ordinal < l.ordinal;
let replacement_edges = select earlier, later from local_replacements;
let inherited = closure(edges=replacement_edges,source="earlier",destination="later");
let read_events = select occurrence as reference, ordinal from "binding-event" where kind = "read" or kind = "delete";
let dated_uses = select l.binding as binding, r.ordinal as ordinal from "binding-use" as l join read_events as r on l.reference = r.reference;
let copied_uses = select r.later as binding from dated_uses as l join local_replacements as r on l.binding = r.earlier where l.ordinal < r.ordinal;
let inherited_uses = select r.later as binding from copied_uses as l join inherited as r on l.binding = r.earlier;
emit inherited_uses subject binding key(binding);
"#;

/// `rules/F401/query.ryoql:75,111-113`.
const F401_EVENT_ANCESTRY: &str = r#"
let events = select occurrence, name, scope, phase, kind, ordinal, file from "binding-event";
let child_parent_edges = select child as origin, parent as destination from "semantic-child";
let event_ancestry = closure(edges=child_parent_edges, source="origin", destination="destination");
let event_branch_ancestors = select l.occurrence as binding, r.destination as owner, l.file as file from events as l join event_ancestry as r on l.occurrence = r.origin;
emit event_branch_ancestors subject binding key(owner);
"#;

/// `rules/B006/query.ryoql:7,21-23,45,49,51`, with `wrapper_parameters` cut to one select and `annotation_flow_edges` to
/// the union of its `bit_or` edges.
const B006_ANNOTATION_PATHS: &str = r#"
let wrapper_parameters = select parent as wrapper, file, child as parameter from "semantic-child" where field = "args";
let parameter_annotations = select l.wrapper as wrapper, l.file as file, r.child as annotation from wrapper_parameters as l join "semantic-child" as r on l.parameter = r.parent and l.file = r.file where r.field = "annotation";
let binops = select id, file from "semantic-syntax" where kind = "bin-op";
let left_fields = select parent, child from "semantic-child" where field = "left";
let right_fields = select parent, child from "semantic-child" where field = "right";
let bit_or_left_edges = select l.id as origin, r.child as destination from binops as l join left_fields as r on l.id = r.parent;
let bit_or_right_edges = select l.id as origin, r.child as destination from binops as l join right_fields as r on l.id = r.parent;
let annotation_flow_edges = union(left=bit_or_left_edges, right=bit_or_right_edges);
let annotation_paths = closure(edges=annotation_flow_edges, source="origin", destination="destination");
let annotation_descendant_nodes = select l.wrapper as wrapper, r.destination as node, l.file as file from parameter_annotations as l join annotation_paths as r on l.annotation = r.origin;
emit annotation_descendant_nodes subject wrapper key(node);
"#;

/// The text a row compiles.
type Input = &'static str;

/// How the plan reads the closure: a join that walks only the sources its left rows bind, or a closure over every source.
#[derive(Debug, PartialEq, Eq)]
enum Reading {
    /// Every closure became a join over its edges that walks from the sources it binds.
    Seeded,
    /// The closure stays, walked from every source.
    Whole,
}

/// What a row answers.
type Value = Reading;

type Failure = CompileError;

/// The plan of `text` from the compile inbound API, read for its closures.
fn subject(text: Input) -> Result<Value, Failure> {
    let plan = compiler_over_the_language()
        .compile(QueryRequest {
            text: QueryBundle::from(QueryText(text.into())),
            hosted: Vec::new(),
            observations: Vec::new(),
            modules: ryo_values::domain::program::types::ParsedModules::default(),
        })?
        .plan;
    let closures = plan
        .steps
        .iter()
        .filter(|step| matches!(step.operation, Operation::Closure { .. }))
        .count();
    let seeded = plan
        .steps
        .iter()
        .filter(|step| matches!(step.operation, Operation::Join { reach: Some(_), .. }))
        .count();
    match (closures, seeded) {
        (0, 1) => Ok(Reading::Seeded),
        (1, 0) => Ok(Reading::Whole),
        other => panic!(
            "one closure, read one way, is expected; the plan holds (closures, seeded joins) {other:?}"
        ),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("cohort_closures_fuse_where_keyed_noqa_line_map_mapping_paths"),
        word: Word::Succeeds,
        input: || NOQA_LINE_MAP_MAPPING_PATHS,
        expect: Expect::Value(Reading::Whole),
    },
    Case {
        name: CaseName("cohort_closures_fuse_where_keyed_f401_left_paths"),
        word: Word::Succeeds,
        input: || F401_LEFT_PATHS,
        expect: Expect::Value(Reading::Whole),
    },
    Case {
        name: CaseName("cohort_closures_fuse_where_keyed_f401_named_paths"),
        word: Word::Succeeds,
        input: || F401_NAMED_PATHS,
        expect: Expect::Value(Reading::Whole),
    },
    Case {
        name: CaseName("cohort_closures_fuse_where_keyed_f401_inherited"),
        word: Word::Succeeds,
        input: || F401_INHERITED,
        expect: Expect::Value(Reading::Whole),
    },
    Case {
        name: CaseName("cohort_closures_fuse_where_keyed_f401_event_ancestry"),
        word: Word::Succeeds,
        input: || F401_EVENT_ANCESTRY,
        expect: Expect::Value(Reading::Seeded),
    },
    Case {
        name: CaseName("cohort_closures_fuse_where_keyed_b006_annotation_paths"),
        word: Word::Succeeds,
        input: || B006_ANNOTATION_PATHS,
        expect: Expect::Value(Reading::Whole),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
