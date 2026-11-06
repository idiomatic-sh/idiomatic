//! Real syntax parsing and fragment linking composed at the inbound boundary.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::program::types::{QueryBundle, WrittenProgram};

type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_fragment_cannot_forward_an_undeclared_caller_parameter"),
        word: Word::Succeeds,
        input: || a_fragment_cannot_forward_an_undeclared_caller_parameter,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("duplicate_fragment_aliases_and_invalid_signatures_are_refused"),
        word: Word::Succeeds,
        input: || duplicate_fragment_aliases_and_invalid_signatures_are_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("imported_module_cycle_is_refused"),
        word: Word::Succeeds,
        input: || imported_module_cycle_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("imported_module_must_be_present_and_arguments_must_be_unique"),
        word: Word::Succeeds,
        input: || imported_module_must_be_present_and_arguments_must_be_unique,
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

fn parse_and_link(bundle: &QueryBundle) -> Result<WrittenProgram, String> {
    let parsed = winnow()
        .read_bundle(bundle)
        .map_err(|error| error.to_string())?;
    crate::domain::expanding::service::link(bundle, parsed.root, &parsed.fragments)
        .map_err(|error| error.to_string())
}

fn a_fragment_cannot_forward_an_undeclared_caller_parameter() {
    let root = "import outer from \"outer\";\nparam secret: string;\nlet result = outer();\nemit result subject id key(id);";
    let outer = "import inner from \"inner\";\noutput relation<id: node>;\nlet result = inner(secret=$secret);\nreturn result;";
    let inner = "param secret: string;\noutput relation<id: node>;\nlet rows = select id from \"syntax-tree\";\nreturn rows;";
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(
                ryo_values::domain::program::types::QuerySpelling("root"),
            )
            .unwrap(),
            root,
        ),
        [
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(
                    ryo_values::domain::program::types::QuerySpelling("outer"),
                )
                .unwrap(),
                outer,
            ),
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(
                    ryo_values::domain::program::types::QuerySpelling("inner"),
                )
                .unwrap(),
                inner,
            ),
        ],
    )
    .unwrap();
    assert!(
        parse_and_link(&bundle)
            .unwrap_err()
            .contains("fragment parameter `$secret` is not declared")
    );
}

fn duplicate_fragment_aliases_and_invalid_signatures_are_refused() {
    let root =
        "import query from \"query\";\nlet result = query();\nemit result subject id key(id);";
    let cases = [
        (
            "fragment input `rows` is declared more than once",
            "input rows: relation<id: node>;\ninput rows: relation<id: node>;\noutput relation<id: node>;\nlet result = select id from rows;\nreturn result;",
        ),
        (
            "fragment parameter `limit` is declared more than once",
            "param limit: integer;\nparam limit: integer;\noutput relation<id: node>;\nlet result = select id from \"syntax-tree\";\nreturn result;",
        ),
        (
            "relation column `id` is declared more than once",
            "output relation<id: node, id: node>;\nlet result = select id from \"syntax-tree\";\nreturn result;",
        ),
        (
            "unknown relation cell type `unknown`",
            "output relation<id: unknown>;\nlet result = select id from \"syntax-tree\";\nreturn result;",
        ),
        (
            "unsupported fragment parameter type `list<string>`",
            "param limit: list<string>;\noutput relation<id: node>;\nlet result = select id from \"syntax-tree\";\nreturn result;",
        ),
    ];
    for (expected, module) in cases {
        let bundle = ryo_values::domain::program::service::query_bundle(
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(
                    ryo_values::domain::program::types::QuerySpelling("root"),
                )
                .unwrap(),
                root,
            ),
            [ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(
                    ryo_values::domain::program::types::QuerySpelling("query"),
                )
                .unwrap(),
                module,
            )],
        )
        .unwrap();
        assert_eq!(parse_and_link(&bundle).unwrap_err(), expected);
    }

    let duplicate_aliases = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("root")).unwrap(),
            root,
        ),
        [
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("query")).unwrap(),
                "import helper from \"one\";\nimport helper from \"two\";\noutput relation<id: node>;\nlet result = select id from \"syntax-tree\";\nreturn result;",
            ),
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("one")).unwrap(),
                "output relation<id: node>;\nlet result = select id from \"syntax-tree\";\nreturn result;",
            ),
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("two")).unwrap(),
                "output relation<id: node>;\nlet result = select id from \"syntax-tree\";\nreturn result;",
            ),
        ],
    )
    .unwrap();
    assert!(
        parse_and_link(&duplicate_aliases)
            .unwrap_err()
            .contains("import alias is declared more than once")
    );
}

fn imported_module_cycle_is_refused() {
    let bundle = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("root")).unwrap(),
            "import query from \"query\";\nlet result = query();\nemit result subject id key(id);",
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("query")).unwrap(),
            "import recursive from \"query\";\noutput relation<id: node>;\nlet rows = select id from \"syntax-tree\";\nreturn rows;",
        )],
    )
    .unwrap();
    assert!(
        parse_and_link(&bundle)
            .unwrap_err()
            .contains("import cycle")
    );
}

fn imported_module_must_be_present_and_arguments_must_be_unique() {
    let missing = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("root")).unwrap(),
            "import query from \"missing\";\nlet result = query();\nemit result subject id key(id);",
        ),
        [],
    )
    .unwrap();
    assert!(
        parse_and_link(&missing)
            .unwrap_err()
            .contains("was not supplied")
    );

    let repeated = ryo_values::domain::program::service::query_bundle(
        ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("root")).unwrap(),
            "import query from \"query\";\nlet result = query(source=\"syntax-tree\", source=\"syntax-tree\");\nemit result subject id key(id);",
        ),
        [ryo_values::domain::program::service::document_source(
            ryo_values::domain::program::service::source_id(ryo_values::domain::program::types::QuerySpelling("query")).unwrap(),
            "input source: relation<id: node>;\noutput relation<id: node>;\nlet rows = select id from source;\nreturn rows;",
        )],
    )
    .unwrap();
    assert!(
        parse_and_link(&repeated)
            .unwrap_err()
            .contains("given more than once")
    );
}
