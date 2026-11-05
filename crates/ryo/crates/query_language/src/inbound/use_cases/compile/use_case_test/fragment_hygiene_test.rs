//! Real syntax parsing and fragment linking composed at the inbound boundary.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::adapters::winnow::adapter::SyntaxWinnowAdapter;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::program::types::{Expression, WordText, WrittenLiteral};
use ryo_values::domain::program::types::{Program, QueryBundle};

type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("fragment_calls_are_hygienic_and_only_relation_slots_are_linked"),
        word: Word::Succeeds,
        input: || fragment_calls_are_hygienic_and_only_relation_slots_are_linked,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("nested_fragments_forward_relation_and_scalar_inputs"),
        word: Word::Succeeds,
        input: || nested_fragments_forward_relation_and_scalar_inputs,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("fragment_scope_does_not_capture_caller_steps_or_import_aliases"),
        word: Word::Succeeds,
        input: || fragment_scope_does_not_capture_caller_steps_or_import_aliases,
        expect: Expect::Value(()),
    },
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn parse_and_link(bundle: &QueryBundle) -> Result<Program, String> {
    let parsed = SyntaxWinnowAdapter
        .read_bundle(bundle)
        .map_err(|error| error.to_string())?;
    crate::domain::linking::service::link(bundle, parsed.root, &parsed.fragments)
        .map_err(|error| error.message)
}

fn fragment_calls_are_hygienic_and_only_relation_slots_are_linked() {
    use crate::outbound::syntax::interface::SyntaxPort as _;
    use ryo_values::domain::program::types::ArgumentValue;

    let root_id = ryo_values::domain::program::service::source_id("root").unwrap();
    let module_id = ryo_values::domain::program::service::source_id("library/summarize").unwrap();
    let root = "import summarize from \"library/summarize\";\nlet id = select id from \"syntax-tree\";\nlet first = summarize(id=id, label=\"first\");\nlet second = summarize(id=id, label=\"second\");\nemit second subject id key(id);";
    let module = "input id: relation<id: node>;\nparam label: string;\noutput relation<id: node>;\nlet grouped = group-count(from=id, keys=[\"id\"], count=\"n\");\nlet linked = closure(edges=grouped, source=id, destination=grouped, keys=[id]);\nlet matched = capture(from=linked, path=\"path\", pattern=$label, captures=[\"group\"]);\nreturn matched;";
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(root_id, root),
        [ryo_values::domain::program::service::document_source(
            module_id, module,
        )],
    )
    .unwrap();
    let parsed = SyntaxWinnowAdapter.read_bundle(&bundle).unwrap();
    let program =
        crate::domain::linking::service::link(&bundle, parsed.root, &parsed.fragments).unwrap();

    assert_eq!(program.steps.len(), 7);
    let grouped = [&program.steps[1], &program.steps[4]];
    let linked = [&program.steps[2], &program.steps[5]];
    let matched = [&program.steps[3], &program.steps[6]];
    let mut group_ids = Vec::new();
    for step in grouped {
        let Expression::Call(call) = &step.expression else {
            panic!("fragment group-count step is a call");
        };
        let from = call
            .arguments
            .iter()
            .find(|argument| argument.name.0.as_ref() == "from")
            .unwrap();
        assert!(
            matches!(&from.value, ArgumentValue::Step(name) if name.1 == Some(program.steps[0].name.1.unwrap()))
        );
        let keys = call
            .arguments
            .iter()
            .find(|argument| argument.name.0.as_ref() == "keys")
            .unwrap();
        assert_eq!(
            keys.value,
            ArgumentValue::List(vec![ArgumentValue::Text(WrittenLiteral("id".into()))])
        );
        group_ids.push(step.name.1.unwrap());
    }
    assert_ne!(group_ids[0], group_ids[1]);

    for (place, step) in linked.iter().enumerate() {
        let Expression::Call(call) = &step.expression else {
            panic!("fragment closure step is a call");
        };
        let argument = |name: &str| {
            call.arguments
                .iter()
                .find(|argument| argument.name.0.as_ref() == name)
                .unwrap()
        };
        assert_eq!(
            argument("edges").value,
            ArgumentValue::Step(ryo_values::domain::program::types::StepName(
                "grouped".into(),
                Some(group_ids[place])
            ))
        );
        assert_eq!(
            argument("source").value,
            ArgumentValue::Word(WordText("id".into()))
        );
        assert_eq!(
            argument("destination").value,
            ArgumentValue::Word(WordText("grouped".into()))
        );
        assert_eq!(
            argument("keys").value,
            ArgumentValue::List(vec![ArgumentValue::Word(WordText("id".into()))])
        );
    }

    let patterns = matched
        .iter()
        .map(|step| {
            let Expression::Call(call) = &step.expression else {
                panic!("fragment capture step is a call");
            };
            call.arguments
                .iter()
                .find(|argument| argument.name.0.as_ref() == "pattern")
                .unwrap()
                .value
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        patterns,
        [
            ArgumentValue::Text(WrittenLiteral("first".into())),
            ArgumentValue::Text(WrittenLiteral("second".into())),
        ]
    );
    let sources = matched
        .iter()
        .filter_map(|step| {
            let Expression::Call(call) = &step.expression else {
                return None;
            };
            call.arguments
                .iter()
                .find(|argument| argument.name.0.as_ref() == "from")
                .map(|argument| argument.value.clone())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sources,
        [
            ArgumentValue::Step(ryo_values::domain::program::types::StepName(
                "linked".into(),
                Some(linked[0].name.1.unwrap())
            )),
            ArgumentValue::Step(ryo_values::domain::program::types::StepName(
                "linked".into(),
                Some(linked[1].name.1.unwrap())
            )),
        ]
    );
}

fn nested_fragments_forward_relation_and_scalar_inputs() {
    use crate::outbound::syntax::interface::SyntaxPort as _;
    use ryo_values::domain::program::types::{ArgumentValue, ParsedQueryBundle};

    let root_id = ryo_values::domain::program::service::source_id("root").unwrap();
    let outer_id = ryo_values::domain::program::service::source_id("outer").unwrap();
    let inner_id = ryo_values::domain::program::service::source_id("inner").unwrap();
    let root_source = "import outer from \"outer\";\nlet source = select id from \"syntax-tree\";\nlet first = outer(source=source, label=\"a\");\nlet second = outer(source=source, label=\"b\");\nemit second subject id key(id);";
    let outer = "import inner from \"inner\";\ninput source: relation<id: node>;\nparam label: string;\noutput relation<id: node>;\nlet found = inner(rows=source, label=$label);\nreturn found;";
    let inner = "input rows: relation<id: node>;\nparam label: string;\noutput relation<id: node>;\nlet found = capture(from=rows, path=\"id\", pattern=$label, captures=[\"hit\"]);\nreturn found;";
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(root_id, root_source),
        [
            ryo_values::domain::program::service::document_source(outer_id, outer),
            ryo_values::domain::program::service::document_source(inner_id, inner),
        ],
    )
    .unwrap();
    let ParsedQueryBundle {
        root: linked_root,
        fragments,
    } = SyntaxWinnowAdapter.read_bundle(&bundle).unwrap();
    let program = crate::domain::linking::service::link(&bundle, linked_root, &fragments).unwrap();
    assert_eq!(program.steps.len(), 3);
    for (step, pattern) in program.steps[1..].iter().zip(["a", "b"]) {
        let Expression::Call(call) = &step.expression else {
            panic!("the nested fragment expands to capture");
        };
        let source = call
            .arguments
            .iter()
            .find(|argument| argument.name.0.as_ref() == "from")
            .unwrap();
        assert!(
            matches!(&source.value, ArgumentValue::Step(name) if name.1 == Some(program.steps[0].name.1.unwrap()))
        );
        let value = call
            .arguments
            .iter()
            .find(|argument| argument.name.0.as_ref() == "pattern")
            .unwrap();
        assert_eq!(
            value.value,
            ArgumentValue::Text(WrittenLiteral(pattern.into()))
        );
    }
}

fn fragment_scope_does_not_capture_caller_steps_or_import_aliases() {
    use ryo_values::domain::program::types::ArgumentValue;

    let root = "import outer from \"outer\";\nimport nested from \"root/nested\";\nlet secret = select id from \"syntax-tree\";\nlet result = outer();\nemit result subject id key(id);";
    let outer = "import nested from \"outer/nested\";\noutput relation<id: node>;\nlet counted = group-count(from=secret, keys=[\"id\"], count=\"n\");\nlet result = nested();\nlet counted_nested = group-count(from=hidden, keys=[\"id\"], count=\"n\");\nreturn result;";
    let outer_identity = "output relation<id: node>;\nlet hidden = select id from \"function\";\nlet rows = select id from hidden;\nreturn rows;";
    let root_identity =
        "output relation<id: node>;\nlet rows = select id from \"syntax-tree\";\nreturn rows;";
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id("root").unwrap(),
            root,
        ),
        [
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("outer").unwrap(),
                outer,
            ),
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("outer/nested").unwrap(),
                outer_identity,
            ),
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("root/nested").unwrap(),
                root_identity,
            ),
        ],
    )
    .unwrap();
    let program = parse_and_link(&bundle).unwrap();
    for (name, reference) in [("counted", "secret"), ("counted_nested", "hidden")] {
        let step = program
            .steps
            .iter()
            .find(|step| step.name.0.as_ref() == name)
            .unwrap();
        let Expression::Call(counted) = &step.expression else {
            panic!("the fragment's local count operation remains present");
        };
        assert_eq!(
            counted.arguments[0].value,
            ArgumentValue::Word(WordText(reference.into()))
        );
    }
    let result = program.emit.step.1.unwrap();
    assert_eq!(
        program.step_origins[&result].source_id.0.as_ref(),
        "outer/nested"
    );
}
